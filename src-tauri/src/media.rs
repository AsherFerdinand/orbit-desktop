//! Windows sessions are the source of truth. No browser/tab enumeration or polling.
use base64::Engine;
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc, Arc, Mutex,
};
use tauri::{Emitter, Manager};
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
use windows::{Foundation::TypedEventHandler, Media::Control::*, Storage::Streams::DataReader};
type Session = GlobalSystemMediaTransportControlsSession;
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    revision: u64,
    sessions: Vec<Item>,
    current_id: Option<u64>,
    error: Option<String>,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    id: u64,
    source: String,
    title: String,
    artist: String,
    subtitle: String,
    album: String,
    artwork_id: Option<u64>,
    #[serde(skip_serializing)]
    artwork: Option<Arc<Artwork>>,
    status: i32,
    play: bool,
    pause: bool,
    toggle: bool,
    seekable: bool,
    next: bool,
    previous: bool,
    sampled_at: f64,
    position: Option<f64>,
    duration: Option<f64>,
    rate: f64,
    error: Option<String>,
}
// Only the current cover per exposed session is retained; no native track history.
struct Artwork {
    id: u64,
    data: String,
}
static ARTWORK_SEQUENCE: AtomicU64 = AtomicU64::new(1);
fn replace_artwork(old: Option<Arc<Artwork>>, data: Option<String>) -> Option<Arc<Artwork>> {
    let data = data?;
    if let Some(old) = old {
        if old.data == data {
            return Some(old);
        }
    }
    Some(Arc::new(Artwork {
        id: ARTWORK_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        data,
    }))
}
pub enum Message {
    Wake,
    Action(
        u64,
        String,
        tokio::sync::oneshot::Sender<Result<(), String>>,
    ),
    Seek(
        u64,
        f64,
        tokio::sync::oneshot::Sender<Result<State, String>>,
    ),
    Stop,
}
pub struct Media {
    pub tx: mpsc::SyncSender<Message>,
    state: Arc<Mutex<State>>,
}
struct Watched {
    id: u64,
    session: Session,
    tokens: Vec<(u8, i64)>,
    metadata_dirty: Arc<AtomicBool>,
    cached: Option<Item>,
    subscription_failed: bool,
}
impl Drop for Watched {
    fn drop(&mut self) {
        for &(kind, token) in &self.tokens {
            let _ = match kind {
                0 => self.session.RemoveMediaPropertiesChanged(token),
                1 => self.session.RemovePlaybackInfoChanged(token),
                _ => self.session.RemoveTimelinePropertiesChanged(token),
            };
        }
    }
}
impl Media {
    pub fn start(app: tauri::AppHandle) -> Self {
        let (tx, rx) = mpsc::sync_channel(64);
        let state = Arc::new(Mutex::new(State::default()));
        let cache = state.clone();
        let sender = tx.clone();
        std::thread::Builder::new()
            .name("orbit-media".into())
            .spawn(move || {
                let initialized = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
                let result = initialized
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|_| worker(&app, &cache, &sender, rx));
                if let Err(e) = result {
                    let mut data = cache.lock().unwrap();
                    data.revision += 1;
                    data.error = Some(format!("Windows media sessions unavailable: {e}"));
                    let _ = app.emit("orbit-media", data.revision);
                }
                if initialized.is_ok() {
                    unsafe {
                        RoUninitialize();
                    }
                }
            })
            .expect("media worker");
        Self { tx, state }
    }
}
fn wait<T>(
    runtime: &tokio::runtime::Runtime,
    operation: impl std::future::IntoFuture<Output = windows::core::Result<T>>,
) -> windows::core::Result<T> {
    runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(2), operation.into_future())
            .await
            .map_err(|_| {
                windows::core::Error::new(
                    windows::core::HRESULT(0x800705B4u32 as i32),
                    "Windows media request timed out",
                )
            })?
    })
}
fn artwork(
    runtime: &tokio::runtime::Runtime,
    session: &GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> Option<String> {
    let stream = wait(runtime, session.Thumbnail().ok()?.OpenReadAsync().ok()?).ok()?;
    let size = stream.Size().ok()?;
    if size == 0 || size > 2 * 1024 * 1024 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).ok()?).ok()?;
    let count = wait(runtime, reader.LoadAsync(size as u32).ok()?).ok()?;
    if count != size as u32 {
        return None;
    }
    let mut bytes = vec![0; count as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    let mime = raster_mime(&bytes)?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}
// Some session streams omit ContentType. Identify supported raster formats by bytes.
fn raster_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else if bytes.starts_with(b"BM") {
        Some("image/bmp")
    } else {
        None
    }
}
fn timeline(start: i64, end: i64, position: i64) -> Option<(f64, f64)> {
    let span = end.checked_sub(start)?;
    if span <= 0 {
        return None;
    }
    let duration = span as f64 / 10_000_000.0;
    let position = (position as f64 - start as f64) / 10_000_000.0;
    Some((position.clamp(0.0, duration), duration))
}
// Convert relative UI seconds to absolute 100ns ticks, respecting native seek bounds.
fn seek_ticks(seconds: f64, start: i64, end: i64, min: i64, max: i64) -> Result<i64, String> {
    if !seconds.is_finite() {
        return Err("Invalid seek position".into());
    }
    let span = end
        .checked_sub(start)
        .filter(|v| *v > 0 && *v < 6_048_000_000_000)
        .ok_or("Timeline unavailable for seeking")?;
    let low = start.max(min);
    let high = end.min(max);
    if low >= high {
        return Err("Session does not expose a usable seek range".into());
    }
    let offset = (seconds.clamp(0.0, span as f64 / 10_000_000.0) * 10_000_000.0).round() as i64;
    start
        .checked_add(offset.min(span))
        .map(|v| v.clamp(low, high))
        .ok_or("Seek position overflow".into())
}
fn seek(runtime: &tokio::runtime::Runtime, w: &Watched, seconds: f64) -> Result<(), String> {
    let session = &w.session;
    let controls = session
        .GetPlaybackInfo()
        .and_then(|p| p.Controls())
        .map_err(|e| e.to_string())?;
    if !controls.IsPlaybackPositionEnabled().unwrap_or(false) {
        return Err("The session does not expose seeking".into());
    }
    let timeline = session.GetTimelineProperties().map_err(|e| e.to_string())?;
    let read = || -> windows::core::Result<i64> {
        let target = seek_ticks(
            seconds,
            timeline.StartTime()?.Duration,
            timeline.EndTime()?.Duration,
            timeline.MinSeekTime()?.Duration,
            timeline.MaxSeekTime()?.Duration,
        )
        .map_err(|message| {
            windows::core::Error::new(windows::core::HRESULT(0x80070057u32 as i32), message)
        })?;
        Ok(target)
    };
    let target = read().map_err(|e| e.to_string())?;
    if wait(
        runtime,
        session
            .TryChangePlaybackPositionAsync(target)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?
    {
        Ok(())
    } else {
        Err("The media session declined the seek".into())
    }
}
fn read(runtime: &tokio::runtime::Runtime, w: &mut Watched) -> Item {
    let s = &w.session;
    let mut item = Item {
        id: w.id,
        source: s
            .SourceAppUserModelId()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        title: String::new(),
        artist: String::new(),
        subtitle: String::new(),
        album: String::new(),
        artwork_id: None,
        artwork: None,
        status: 0,
        play: false,
        pause: false,
        toggle: false,
        seekable: false,
        next: false,
        previous: false,
        sampled_at: 0.0,
        position: None,
        duration: None,
        rate: 1.0,
        error: None,
    };
    if let Some(old) = &w.cached {
        item.title = old.title.clone();
        item.artist = old.artist.clone();
        item.subtitle = old.subtitle.clone();
        item.album = old.album.clone();
        item.artwork = old.artwork.clone();
        item.error = old.error.clone();
    }
    if w.metadata_dirty.swap(false, Ordering::AcqRel) {
        item.error = None;
        item.title.clear();
        item.artist.clear();
        item.subtitle.clear();
        item.album.clear();
        let old_artwork = item.artwork.take();
        match s
            .TryGetMediaPropertiesAsync()
            .and_then(|p| wait(runtime, p))
        {
            Ok(p) => {
                item.title = p.Title().map(|v| v.to_string()).unwrap_or_default();
                item.artist = p.Artist().map(|v| v.to_string()).unwrap_or_default();
                if item.artist.trim().is_empty() {
                    item.artist = p.AlbumArtist().map(|v| v.to_string()).unwrap_or_default();
                }
                item.subtitle = p.Subtitle().map(|v| v.to_string()).unwrap_or_default();
                item.album = p.AlbumTitle().map(|v| v.to_string()).unwrap_or_default();
                item.artwork = replace_artwork(old_artwork, artwork(runtime, &p));
            }
            Err(e) => item.error = Some(format!("Metadata unavailable: {e}")),
        }
    }
    if let Ok(p) = s.GetPlaybackInfo() {
        item.status = p.PlaybackStatus().map(|v| v.0).unwrap_or(0);
        item.rate = p
            .PlaybackRate()
            .and_then(|v| v.Value())
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .unwrap_or(1.0);
        if let Ok(c) = p.Controls() {
            item.play = c.IsPlayEnabled().unwrap_or(false);
            item.pause = c.IsPauseEnabled().unwrap_or(false);
            item.seekable = c.IsPlaybackPositionEnabled().unwrap_or(false);
            item.toggle = c.IsPlayPauseToggleEnabled().unwrap_or(false);
            item.next = c.IsNextEnabled().unwrap_or(false);
            item.previous = c.IsPreviousEnabled().unwrap_or(false);
        }
    }
    if let Ok(t) = s.GetTimelineProperties() {
        if let (Ok(start), Ok(end), Ok(pos)) = (t.StartTime(), t.EndTime(), t.Position()) {
            if let Some((position, duration)) = timeline(start.Duration, end.Duration, pos.Duration)
            {
                item.duration = Some(duration);
                item.position = Some(position);
                item.seekable &= match (t.MinSeekTime(), t.MaxSeekTime()) {
                    (Ok(min), Ok(max)) => seek_ticks(
                        0.0,
                        start.Duration,
                        end.Duration,
                        min.Duration,
                        max.Duration,
                    )
                    .is_ok(),
                    _ => false,
                };
            }
        }
    }
    item.seekable &= item.position.is_some() && item.duration.is_some();
    if item.status == 4 {
        if let (Some(pos), Some(duration), Ok(t)) =
            (item.position, item.duration, s.GetTimelineProperties())
        {
            if let Ok(updated) = t.LastUpdatedTime() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
                    + 11_644_473_600.0;
                let elapsed = (now - updated.UniversalTime as f64 / 10_000_000.0).max(0.0);
                if updated.UniversalTime > 0 && item.rate.is_finite() {
                    item.position = Some((pos + elapsed * item.rate).clamp(0.0, duration));
                }
            }
        }
    }
    item.sampled_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0;
    item.artwork_id = item.artwork.as_ref().map(|a| a.id);
    w.cached = Some(item.clone());
    item
}
fn worker(
    app: &tauri::AppHandle,
    cache: &Arc<Mutex<State>>,
    tx: &mpsc::SyncSender<Message>,
    rx: mpsc::Receiver<Message>,
) -> windows::core::Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|e| {
            windows::core::Error::new(windows::core::HRESULT(0x80004005u32 as i32), e.to_string())
        })?;
    let manager = wait(
        &runtime,
        GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?,
    )?;
    let sender = tx.clone();
    let a = manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
        let _ = sender.try_send(Message::Wake);
        Ok(())
    }))?;
    let sender = tx.clone();
    let b = match manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
        let _ = sender.try_send(Message::Wake);
        Ok(())
    })) {
        Ok(token) => token,
        Err(e) => {
            let _ = manager.RemoveSessionsChanged(a);
            return Err(e);
        }
    };
    let mut watched: Vec<Watched> = Vec::new();
    let mut sequence = 0;
    let _ = tx.try_send(Message::Wake);
    while let Ok(message) = rx.recv() {
        let mut seek_reply = None;
        match message {
            Message::Stop => break,
            Message::Action(id, action, reply) => {
                if reply.is_closed() {
                    continue;
                }
                let result = watched
                    .iter()
                    .find(|w| w.id == id)
                    .ok_or_else(|| "This Windows media session has closed".to_string())
                    .and_then(|w| {
                        let c = w
                            .session
                            .GetPlaybackInfo()
                            .and_then(|p| p.Controls())
                            .map_err(|e| e.to_string())?;
                        let op = match action.as_str() {
                            "play" if c.IsPlayEnabled().unwrap_or(false) => {
                                w.session.TryPlayAsync()
                            }
                            "pause" if c.IsPauseEnabled().unwrap_or(false) => {
                                w.session.TryPauseAsync()
                            }
                            "toggle" if c.IsPlayPauseToggleEnabled().unwrap_or(false) => {
                                w.session.TryTogglePlayPauseAsync()
                            }
                            "next" if c.IsNextEnabled().unwrap_or(false) => {
                                w.session.TrySkipNextAsync()
                            }
                            "previous" if c.IsPreviousEnabled().unwrap_or(false) => {
                                w.session.TrySkipPreviousAsync()
                            }
                            _ => return Err("The session does not expose this control".into()),
                        };
                        if op
                            .and_then(|v| wait(&runtime, v))
                            .map_err(|e| e.to_string())?
                        {
                            Ok(())
                        } else {
                            Err("The media session declined the command".into())
                        }
                    });
                let _ = reply.send(result);
            }
            Message::Seek(id, seconds, reply) => {
                if reply.is_closed() {
                    continue;
                }
                let result = watched
                    .iter()
                    .find(|w| w.id == id)
                    .ok_or_else(|| "This Windows media session has closed".to_string())
                    .and_then(|w| {
                        // A removal callback may still be queued behind this request.
                        let sessions = manager.GetSessions().map_err(|e| e.to_string())?;
                        if !sessions.into_iter().any(|s| s == w.session) {
                            return Err("This Windows media session has closed".into());
                        }
                        seek(&runtime, w, seconds)
                    });
                seek_reply = Some((reply, result));
            }
            Message::Wake => {}
        }
        match manager.GetSessions() {
            Ok(sessions) => {
                let sessions: Vec<_> = sessions.into_iter().collect();
                watched.retain(|w| sessions.iter().any(|s| s == &w.session));
                for session in sessions {
                    if watched.iter().any(|w| w.session == session) {
                        continue;
                    }
                    sequence += 1;
                    let mut w = Watched {
                        id: sequence,
                        session,
                        tokens: Vec::new(),
                        metadata_dirty: Arc::new(AtomicBool::new(true)),
                        cached: None,
                        subscription_failed: false,
                    };
                    let sender = tx.clone();
                    let dirty = w.metadata_dirty.clone();
                    if let Ok(t) =
                        w.session
                            .MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
                                dirty.store(true, Ordering::Release);
                                let _ = sender.try_send(Message::Wake);
                                Ok(())
                            }))
                    {
                        w.tokens.push((0, t));
                    } else {
                        w.subscription_failed = true;
                    }
                    let sender = tx.clone();
                    if let Ok(t) =
                        w.session
                            .PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
                                let _ = sender.try_send(Message::Wake);
                                Ok(())
                            }))
                    {
                        w.tokens.push((1, t));
                    } else {
                        w.subscription_failed = true;
                    }
                    let sender = tx.clone();
                    if let Ok(t) =
                        w.session
                            .TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
                                let _ = sender.try_send(Message::Wake);
                                Ok(())
                            }))
                    {
                        w.tokens.push((2, t));
                    } else {
                        w.subscription_failed = true;
                    }
                    watched.push(w);
                }
                let current = manager.GetCurrentSession().ok();
                let current_id = watched
                    .iter()
                    .find(|w| Some(&w.session) == current.as_ref())
                    .map(|w| w.id);
                let sessions = watched.iter_mut().map(|w| read(&runtime, w)).collect();
                let mut data = cache.lock().unwrap();
                data.revision += 1;
                data.sessions = sessions;
                data.current_id = current_id;
                data.error = watched.iter().any(|w|w.subscription_failed).then(|| "Windows could not subscribe to all session updates. Restart ORBIT to reconnect.".into());
                let _ = app.emit("orbit-media", data.revision);
            }
            Err(e) => {
                watched.clear();
                let mut data = cache.lock().unwrap();
                data.revision += 1;
                data.sessions.clear();
                data.current_id = None;
                data.error = Some(format!("Cannot enumerate Windows sessions: {e}"));
                let _ = app.emit("orbit-media", data.revision);
            }
        }
        if let Some((reply, result)) = seek_reply {
            // The normal refresh above publishes actual GSMTC state before replying.
            let _ = reply.send(result.map(|_| cache.lock().unwrap().clone()));
        }
    }
    let _ = manager.RemoveSessionsChanged(a);
    let _ = manager.RemoveCurrentSessionChanged(b);
    Ok(())
}
#[tauri::command]
pub fn get_media(app: tauri::AppHandle) -> State {
    app.state::<Media>().state.lock().unwrap().clone()
}
// Large data is copied/serialized only on an explicit cover request, never a timeline read.
#[tauri::command]
pub fn get_media_artwork(app: tauri::AppHandle, id: u64) -> Option<String> {
    app.state::<Media>()
        .state
        .lock()
        .unwrap()
        .sessions
        .iter()
        .filter_map(|s| s.artwork.as_ref())
        .find(|a| a.id == id)
        .map(|a| a.data.clone())
}
#[tauri::command]
pub async fn media_action(app: tauri::AppHandle, id: u64, action: String) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.state::<Media>()
        .tx
        .try_send(Message::Action(id, action, tx))
        .map_err(|_| "Media worker unavailable".to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(4), rx)
        .await
        .map_err(|_| "Media command timed out; check the browser".to_string())?
        .map_err(|_| "Media worker stopped".to_string())?
}

#[tauri::command]
pub async fn media_seek(app: tauri::AppHandle, id: u64, position: f64) -> Result<State, String> {
    if !position.is_finite() {
        return Err("Invalid seek position".into());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.state::<Media>()
        .tx
        .try_send(Message::Seek(id, position, tx))
        .map_err(|_| "Media worker unavailable".to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(4), rx)
        .await
        .map_err(|_| "Media seek timed out; check the browser".to_string())?
        .map_err(|_| "Media worker stopped".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn artwork_identity_reuses_current_cover_without_retaining_history() {
        let first = replace_artwork(None, Some("data:image/png;base64,first".into())).unwrap();
        let same = replace_artwork(Some(first.clone()), Some(first.data.clone())).unwrap();
        assert!(Arc::ptr_eq(&first, &same));
        let next = replace_artwork(Some(same), Some("data:image/png;base64,next".into())).unwrap();
        assert_ne!(first.id, next.id);
        assert!(replace_artwork(Some(next), None).is_none());
        // Frequent state/seek serialization carries only the small ID.
        let state = State {
            revision: 1,
            sessions: vec![Item {
                artwork_id: Some(first.id),
                artwork: Some(first.clone()),
                ..Item::default()
            }],
            ..State::default()
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.contains("base64"));
        assert!(json.contains("artworkId"));
        let weak = Arc::downgrade(&first);
        drop(state);
        drop(first);
        assert!(
            weak.upgrade().is_none(),
            "old cover is released when current owners drop"
        );
    }
    #[test]
    fn raster_artwork_handles_missing_mime_and_rejects_active_content() {
        assert_eq!(raster_mime(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
        assert_eq!(raster_mime(&[0xff, 0xd8, 0xff, 1]), Some("image/jpeg"));
        assert_eq!(raster_mime(b"RIFF0000WEBP"), Some("image/webp"));
        assert_eq!(raster_mime(b"GIF89a"), Some("image/gif"));
        assert_eq!(raster_mime(b"<svg/>"), None);
        assert_eq!(raster_mime(b""), None);
    }
    #[test]
    fn missing_or_invalid_timeline_is_unavailable() {
        assert_eq!(timeline(0, 0, 0), None);
        assert_eq!(timeline(10, 0, 0), None);
        assert_eq!(timeline(i64::MIN, i64::MAX, 0), None);
    }
    #[test]
    fn seek_conversion_clamps_to_timeline_and_native_range() {
        let ticks = |seconds| {
            seek_ticks(
                seconds,
                10_000_000,
                3_010_000_000,
                10_000_000,
                3_010_000_000,
            )
            .unwrap()
        };
        assert_eq!(ticks(0.0), 10_000_000);
        assert_eq!(ticks(75.0), 760_000_000);
        assert_eq!(ticks(150.0), 1_510_000_000);
        assert_eq!(ticks(300.0), 3_010_000_000);
        assert_eq!(ticks(-10.0), ticks(0.0));
        assert_eq!(ticks(999.0), ticks(300.0));
        assert_eq!(seek_ticks(0.0, 0, 1_000, 100, 900).unwrap(), 100);
        assert_eq!(seek_ticks(100.0, 0, 1_000, 100, 900).unwrap(), 900);
        for position in [f64::NAN, f64::INFINITY] {
            assert!(seek_ticks(position, 0, 100, 0, 100).is_err());
        }
        for (start, end, min, max) in [
            (0, 0, 0, 0),
            (0, 100, 0, 0),
            (0, 100, 90, 10),
            (i64::MIN, i64::MAX, 0, 100),
        ] {
            assert!(seek_ticks(0.0, start, end, min, max).is_err());
        }
    }
    #[test]
    fn timeline_uses_start_offset_and_bounds() {
        assert_eq!(
            timeline(10_000_000, 110_000_000, 30_000_000),
            Some((2.0, 10.0))
        );
        assert_eq!(timeline(0, 10_000_000, -1), Some((0.0, 1.0)));
        assert_eq!(timeline(0, 10_000_000, 20_000_000), Some((1.0, 1.0)));
    }
}
