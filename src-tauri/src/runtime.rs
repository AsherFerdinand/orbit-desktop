use crate::{
    audio::{Audio, Volume},
    battery::{self, Battery},
    events::{Event, Transitions},
    settings::Settings,
    system::{self, SystemInfo},
};
use serde::Serialize;
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub battery: Option<Battery>,
    pub system: SystemInfo,
    pub volume: Option<Volume>,
    pub audio_error: Option<String>,
}
pub enum Message {
    Activity(bool),
    Settings(Settings),
    PowerChanged,
    NotesChanged(Option<u64>),
    RefreshAudio,
    AudioChanged {
        generation: u64,
        volume: Volume,
        external: bool,
    },
    SetVolume {
        percent: Option<f64>,
        muted: Option<bool>,
        reply: tokio::sync::oneshot::Sender<Result<Volume, String>>,
    },
    Stop,
}
pub struct Runtime {
    pub tx: Sender<Message>,
    snapshot: Arc<Mutex<Snapshot>>,
}
impl Runtime {
    pub fn start(app: tauri::AppHandle, initial: Settings) -> Self {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let cache = snapshot.clone();
        let sender = tx.clone();
        std::thread::Builder::new()
            .name("orbit-native".into())
            .spawn(move || {
                // All Core Audio objects live and are released on this dedicated MTA thread.
                let initialized = unsafe {
                    windows::Win32::System::Com::CoInitializeEx(
                        None,
                        windows::Win32::System::Com::COINIT_MULTITHREADED,
                    )
                }
                .is_ok();
                let mut audio = if initialized {
                    Audio::new(sender.clone()).ok()
                } else {
                    None
                };
                let _power = crate::power::PowerSubscription::new(sender);
                let mut data = Snapshot::default();
                if let Some(a) = audio.as_mut() {
                    match a.refresh() {
                        Ok(v) => data.volume = Some(v),
                        Err(e) => data.audio_error = Some(e.to_string()),
                    }
                } else {
                    data.audio_error = Some("Core Audio initialization unavailable".into());
                }
                let mut settings = initial;
                let mut active = false;
                let mut previous = None;
                let mut transitions = Transitions::default();
                let mut next_system = Instant::now();
                let mut next_background = Instant::now();
                let mut next_network = Instant::now();
                let mut last_visible = false;
                let mut last_sampling = false;
                let mut next_retry = Instant::now() + Duration::from_secs(30);
                let mut next_reminder = crate::notes::next_reminder(&app).unwrap_or_else(|e| {
                    eprintln!("ORBIT reminder load: {e}");
                    None
                });
                let mut reminder_retry: Option<Instant> = None;
                loop {
                    let now = Instant::now();
                    let wall_now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    if next_reminder.is_some_and(|at| at <= wall_now)
                        && reminder_retry.is_none_or(|retry| now >= retry)
                    {
                        match crate::notes::deliver_due(&app, wall_now, &settings) {
                            Ok(next) => {
                                next_reminder = next;
                                reminder_retry = None;
                            }
                            Err(e) => {
                                eprintln!("ORBIT reminder persistence: {e}");
                                reminder_retry = Some(now + Duration::from_secs(30));
                            }
                        }
                    }
                    let visible = app
                        .get_webview_window("main")
                        .is_some_and(|w| w.is_visible().unwrap_or(false));
                    let sampling = visible && active && settings.system_enabled;
                    if refresh_on_wake(visible, last_visible, sampling, last_sampling) {
                        previous = None;
                        next_system = now;
                        next_background = now;
                        next_network = now;
                    }
                    if (visible != last_visible || sampling != last_sampling)
                        && !sampling
                        && !(visible && !last_visible)
                    {
                        next_network =
                            now + network_interval(visible, sampling, settings.refresh_interval);
                    }
                    if !sampling {
                        previous = None;
                    }
                    last_visible = visible;
                    last_sampling = sampling;
                    if sampling && now >= next_system {
                        let network = data.system.network;
                        data.system = system::sample(&mut previous);
                        data.system.network = network;
                        next_system = now + Duration::from_secs(settings.refresh_interval as u64);
                    }
                    if now >= next_background {
                        if let Ok(b) = battery::get_battery() {
                            for event in transitions.battery(&b, &settings) {
                                if visible {
                                    let _ = app.emit("orbit-event", event);
                                }
                            }
                            data.battery = Some(b);
                        } else {
                            data.battery = None;
                        }
                        next_background = now + Duration::from_secs(if visible { 5 } else { 30 });
                    }
                    if now >= next_network {
                        let network = system::network();
                        data.system.network = network;
                        for event in transitions.network(network, &settings) {
                            if visible {
                                let _ = app.emit("orbit-event", event);
                            }
                        }
                        next_network =
                            now + network_interval(visible, sampling, settings.refresh_interval);
                    }
                    if now >= next_retry {
                        if audio.is_none() && initialized {
                            audio = Audio::new(tx_for_retry(&app)).ok();
                        }
                        if data.volume.is_none() {
                            if let Some(a) = audio.as_mut() {
                                match a.refresh() {
                                    Ok(v) => {
                                        data.volume = Some(v);
                                        data.audio_error = None;
                                    }
                                    Err(e) => data.audio_error = Some(e.to_string()),
                                }
                            }
                        }
                        next_retry = now + Duration::from_secs(30);
                    }
                    if let Ok(mut cached) = cache.lock() {
                        if *cached != data {
                            *cached = data.clone();
                            let _ = app.emit("orbit-snapshot", &data);
                        }
                    }
                    let deadline = if sampling {
                        next_system.min(next_background).min(next_retry)
                    } else {
                        next_background.min(next_retry)
                    };
                    let deadline = deadline.min(next_network);
                    let mut wait = deadline.saturating_duration_since(Instant::now());
                    if let Some(at) = next_reminder {
                        let reminder_wait = reminder_retry
                            .map(|retry| retry.saturating_duration_since(Instant::now()))
                            .unwrap_or_else(|| Duration::from_millis(at.saturating_sub(wall_now)));
                        wait = wait.min(reminder_wait);
                    }
                    match rx.recv_timeout(wait) {
                        Ok(Message::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Ok(Message::Activity(value)) => {
                            active = value;
                        }
                        Ok(Message::Settings(value)) => {
                            if settings != value {
                                settings = value;
                                next_network = Instant::now();
                                next_system = Instant::now();
                                next_background = Instant::now();
                            }
                        }
                        Ok(Message::PowerChanged) => next_background = Instant::now(),
                        Ok(Message::NotesChanged(next)) => {
                            next_reminder = next;
                            reminder_retry = None;
                        }
                        Ok(Message::RefreshAudio) => {
                            if let Some(a) = audio.as_mut() {
                                match a.refresh() {
                                    Ok(v) => {
                                        data.volume = Some(v);
                                        data.audio_error = None;
                                    }
                                    Err(e) => {
                                        data.volume = None;
                                        data.audio_error = Some(e.to_string());
                                    }
                                }
                            }
                        }
                        Ok(Message::AudioChanged {
                            generation,
                            volume,
                            external,
                        }) => {
                            if audio.as_ref().is_some_and(|a| a.generation == generation) {
                                let changed = data.volume.as_ref() != Some(&volume);
                                data.volume = Some(volume.clone());
                                if changed && external && settings.volume_notifications && visible {
                                    let _ = app.emit(
                                        "orbit-event",
                                        Event::new(
                                            "VOLUME_CHANGED",
                                            if volume.muted {
                                                "Sound muted"
                                            } else {
                                                "System volume"
                                            },
                                            format!("{}%", volume.percent),
                                            30,
                                            &settings,
                                        ),
                                    );
                                }
                            }
                        }
                        Ok(Message::SetVolume {
                            percent,
                            muted,
                            reply,
                        }) => {
                            let result = audio
                                .as_ref()
                                .ok_or("Core Audio unavailable".into())
                                .and_then(|a| a.set(percent, muted));
                            if let Ok(v) = &result {
                                data.volume = Some(v.clone());
                                if settings.volume_notifications && visible {
                                    let _ = app.emit(
                                        "orbit-event",
                                        Event::new(
                                            "VOLUME_CHANGED",
                                            if v.muted {
                                                "Sound muted"
                                            } else {
                                                "System volume"
                                            },
                                            format!("{}%", v.percent),
                                            30,
                                            &settings,
                                        ),
                                    );
                                }
                            } else {
                                data.audio_error = result.as_ref().err().cloned();
                            }
                            let _ = reply.send(result);
                        }
                    }
                }
                drop(audio);
                drop(_power);
                if initialized {
                    unsafe {
                        windows::Win32::System::Com::CoUninitialize();
                    }
                }
            })
            .expect("Could not create native service worker");
        Self { tx, snapshot }
    }
}
fn refresh_on_wake(visible: bool, was_visible: bool, sampling: bool, was_sampling: bool) -> bool {
    visible && (!was_visible || (sampling && !was_sampling))
}
fn network_interval(visible: bool, sampling: bool, refresh: u32) -> Duration {
    Duration::from_secs(if !visible {
        30
    } else if sampling {
        refresh as u64
    } else {
        10
    })
}
fn tx_for_retry(app: &tauri::AppHandle) -> Sender<Message> {
    app.state::<Runtime>().tx.clone()
}
#[tauri::command]
pub fn get_snapshot(runtime: tauri::State<Runtime>) -> Result<Snapshot, String> {
    runtime
        .snapshot
        .lock()
        .map(|v| v.clone())
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn set_monitoring(active: bool, runtime: tauri::State<Runtime>) -> Result<(), String> {
    runtime
        .tx
        .send(Message::Activity(active))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn set_volume(
    percent: Option<f64>,
    muted: Option<bool>,
    runtime: tauri::State<'_, Runtime>,
) -> Result<Volume, String> {
    if let Some(p) = percent {
        crate::audio::scalar(p)?;
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    runtime
        .tx
        .send(Message::SetVolume {
            percent,
            muted,
            reply: tx,
        })
        .map_err(|e| e.to_string())?;
    tokio::time::timeout(Duration::from_secs(3), rx)
        .await
        .map_err(|_| "Audio request timed out".to_string())?
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn telemetry_state_and_wake_policy() {
        assert_eq!(network_interval(true, true, 2), Duration::from_secs(2));
        assert_eq!(network_interval(true, true, 7), Duration::from_secs(7));
        assert_eq!(network_interval(true, false, 2), Duration::from_secs(10));
        assert_eq!(network_interval(false, true, 2), Duration::from_secs(30));
        assert!(refresh_on_wake(true, false, false, false));
        assert!(refresh_on_wake(true, true, true, false));
        assert!(!refresh_on_wake(true, true, true, true));
        assert!(!refresh_on_wake(false, true, false, true));
        assert!(!refresh_on_wake(true, true, false, false));
    }
}
