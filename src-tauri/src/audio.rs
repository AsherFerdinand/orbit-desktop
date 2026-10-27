#![allow(non_snake_case)]
use crate::runtime::Message;
use serde::Serialize;
use std::sync::mpsc::Sender;
use windows::{
    core::{implement, Result, GUID, PCWSTR},
    Win32::{
        Foundation::PROPERTYKEY,
        Media::Audio::{Endpoints::*, *},
        System::Com::{CoCreateInstance, CLSCTX_ALL},
    },
};
pub const ORBIT_CONTEXT: GUID = GUID::from_u128(0xb5fcf378_2045_4fb4_9f04_6798f5c10102);
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub percent: u8,
    pub muted: bool,
}
pub fn scalar(percent: f64) -> std::result::Result<f32, String> {
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return Err("Volume must be between 0 and 100".into());
    }
    Ok((percent / 100.0) as f32)
}
pub fn percent(scalar: f32) -> u8 {
    (scalar.clamp(0.0, 1.0) * 100.0).round() as u8
}
#[implement(IAudioEndpointVolumeCallback)]
struct VolumeCallback {
    tx: Sender<Message>,
    generation: u64,
}
impl IAudioEndpointVolumeCallback_Impl for VolumeCallback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        if !data.is_null() {
            // The callback data is valid only during this call; copy the fields into an owned message.
            let d = unsafe { &*data };
            let _ = self.tx.send(Message::AudioChanged {
                generation: self.generation,
                volume: Volume {
                    percent: percent(d.fMasterVolume),
                    muted: d.bMuted.as_bool(),
                },
                external: d.guidEventContext != ORBIT_CONTEXT,
            });
        }
        Ok(())
    }
}
#[implement(IMMNotificationClient)]
struct DeviceCallback {
    tx: Sender<Message>,
}
impl IMMNotificationClient_Impl for DeviceCallback_Impl {
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> Result<()> {
        let _ = self.tx.send(Message::RefreshAudio);
        Ok(())
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> Result<()> {
        let _ = self.tx.send(Message::RefreshAudio);
        Ok(())
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> Result<()> {
        let _ = self.tx.send(Message::RefreshAudio);
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _: &PCWSTR) -> Result<()> {
        if flow == eRender && role == eMultimedia {
            let _ = self.tx.send(Message::RefreshAudio);
        }
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}
pub struct Audio {
    enumerator: IMMDeviceEnumerator,
    device_callback: IMMNotificationClient,
    endpoint: Option<IAudioEndpointVolume>,
    callback: Option<IAudioEndpointVolumeCallback>,
    tx: Sender<Message>,
    pub generation: u64,
}
impl Audio {
    pub fn new(tx: Sender<Message>) -> Result<Self> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let callback: IMMNotificationClient = DeviceCallback { tx: tx.clone() }.into();
            enumerator.RegisterEndpointNotificationCallback(&callback)?;
            Ok(Self {
                enumerator,
                device_callback: callback,
                endpoint: None,
                callback: None,
                tx,
                generation: 0,
            })
        }
    }
    pub fn refresh(&mut self) -> Result<Volume> {
        self.clear();
        self.generation += 1;
        unsafe {
            let device = self
                .enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
            let callback: IAudioEndpointVolumeCallback = VolumeCallback {
                tx: self.tx.clone(),
                generation: self.generation,
            }
            .into();
            endpoint.RegisterControlChangeNotify(&callback)?;
            self.endpoint = Some(endpoint);
            self.callback = Some(callback);
            self.read()
        }
    }
    pub fn read(&self) -> Result<Volume> {
        let endpoint = self.endpoint.as_ref().ok_or_else(|| {
            windows::core::Error::from_hresult(windows::core::HRESULT(0x80070490u32 as i32))
        })?;
        unsafe {
            Ok(Volume {
                percent: percent(endpoint.GetMasterVolumeLevelScalar()?),
                muted: endpoint.GetMute()?.as_bool(),
            })
        }
    }
    pub fn set(
        &self,
        value: Option<f64>,
        muted: Option<bool>,
    ) -> std::result::Result<Volume, String> {
        let endpoint = self.endpoint.as_ref().ok_or("No default output device")?;
        unsafe {
            if let Some(v) = value {
                endpoint
                    .SetMasterVolumeLevelScalar(scalar(v)?, &ORBIT_CONTEXT)
                    .map_err(|e| e.to_string())?;
            }
            if let Some(m) = muted {
                endpoint
                    .SetMute(m, &ORBIT_CONTEXT)
                    .map_err(|e| e.to_string())?;
            }
        }
        self.read().map_err(|e| e.to_string())
    }
    fn clear(&mut self) {
        if let (Some(endpoint), Some(callback)) = (&self.endpoint, &self.callback) {
            unsafe {
                let _ = endpoint.UnregisterControlChangeNotify(callback);
            }
        }
        self.callback = None;
        self.endpoint = None;
    }
}
impl Drop for Audio {
    fn drop(&mut self) {
        self.clear();
        unsafe {
            let _ = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.device_callback);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conversion_and_limits() {
        assert_eq!(scalar(70.0).unwrap(), 0.7);
        assert!(scalar(-1.0).is_err());
        assert!(scalar(101.0).is_err());
        assert!(scalar(f64::NAN).is_err());
        assert_eq!(percent(0.704), 70);
        assert_eq!(percent(2.0), 100);
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    #[test]
    #[ignore = "Requires an interactive Windows default output; temporarily changes volume by 1% and restores it"]
    fn native_volume_and_external_callback() {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
            .ok()
            .unwrap();
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let mut audio = Audio::new(tx).expect("Core Audio enumerator");
        let original = audio.refresh().expect("Default output endpoint");
        struct Restore<'a> {
            audio: &'a Audio,
            original: Volume,
        }
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                let _ = self.audio.set(
                    Some(self.original.percent as f64),
                    Some(self.original.muted),
                );
            }
        }
        let guard = Restore {
            audio: &audio,
            original: original.clone(),
        };
        let target = if original.percent > 0 {
            original.percent - 1
        } else {
            1
        };
        // A null context represents an independent Windows client, exercising the external callback route.
        unsafe {
            audio
                .endpoint
                .as_ref()
                .unwrap()
                .SetMasterVolumeLevelScalar(scalar(target as f64).unwrap(), std::ptr::null())
                .unwrap();
        }
        assert_eq!(audio.read().unwrap().percent, target);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut received = false;
        while std::time::Instant::now() < deadline {
            if let Ok(Message::AudioChanged {
                volume,
                external: true,
                ..
            }) = rx.recv_timeout(std::time::Duration::from_millis(100))
            {
                if volume.percent == target {
                    received = true;
                    break;
                }
            }
        }
        assert!(
            received,
            "Independent Windows change must reach the registered callback"
        );
        drop(guard);
        assert_eq!(audio.read().unwrap(), original);
        println!("Real audio read/write and external callback verified; restored {original:?}");
        drop(audio);
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}
