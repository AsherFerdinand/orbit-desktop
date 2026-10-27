//! On-demand current-user Windows appearance. No saved ORBIT preference or watcher.
use serde::Serialize;
use std::sync::Mutex;
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::*,
    UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    },
};
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const APPS: &str = "AppsUseLightTheme";
const SYSTEM: &str = "SystemUsesLightTheme";
static WRITE_LOCK: Mutex<()> = Mutex::new(());
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowsTheme {
    Light,
    Dark,
    Mixed,
}
#[derive(Serialize)]
pub struct ThemeChange {
    pub mode: Option<WindowsTheme>,
    pub warning: Option<String>,
}
struct RawValue {
    kind: u32,
    bytes: Vec<u8>,
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn error(operation: &str, code: u32) -> String {
    format!(
        "{operation}: {}",
        std::io::Error::from_raw_os_error(code as i32)
    )
}
impl Key {
    fn open(write: bool) -> Result<Option<Self>, String> {
        let path = wide(PERSONALIZE);
        let mut key = std::ptr::null_mut();
        let code = unsafe {
            if write {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    path.as_ptr(),
                    0,
                    std::ptr::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                    std::ptr::null(),
                    &mut key,
                    std::ptr::null_mut(),
                )
            } else {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    path.as_ptr(),
                    0,
                    KEY_QUERY_VALUE,
                    &mut key,
                )
            }
        };
        if !write && matches!(code, ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) {
            return Ok(None);
        }
        if code != ERROR_SUCCESS {
            return Err(error("Cannot open current-user Windows appearance", code));
        }
        Ok(Some(Self(key)))
    }
    fn read(&self, name: &str) -> Result<Option<RawValue>, String> {
        let name = wide(name);
        let mut size = 0;
        let mut kind = 0;
        let code = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if code == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if code != ERROR_SUCCESS {
            return Err(error("Cannot read Windows appearance", code));
        }
        if size > 1024 {
            return Err("Windows appearance value is malformed".into());
        }
        let mut bytes = vec![0; size as usize];
        let code = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                bytes.as_mut_ptr(),
                &mut size,
            )
        };
        if code != ERROR_SUCCESS {
            return Err(error("Cannot read Windows appearance", code));
        }
        bytes.truncate(size as usize);
        Ok(Some(RawValue { kind, bytes }))
    }
    fn write(&self, name: &str, value: Option<&RawValue>) -> Result<(), String> {
        let name = wide(name);
        let code = unsafe {
            match value {
                Some(value) => RegSetValueExW(
                    self.0,
                    name.as_ptr(),
                    0,
                    value.kind,
                    value.bytes.as_ptr(),
                    value.bytes.len() as u32,
                ),
                None => RegDeleteValueW(self.0, name.as_ptr()),
            }
        };
        if code != ERROR_SUCCESS && !(value.is_none() && code == ERROR_FILE_NOT_FOUND) {
            return Err(error("Cannot write Windows appearance", code));
        }
        Ok(())
    }
    fn mode(&self) -> Result<Option<WindowsTheme>, String> {
        Ok(interpret(
            self.read(APPS)?.as_ref(),
            self.read(SYSTEM)?.as_ref(),
        ))
    }
}
fn light(value: Option<&RawValue>) -> Option<bool> {
    let value = value?;
    if value.kind != REG_DWORD {
        return None;
    }
    match u32::from_le_bytes(value.bytes.as_slice().try_into().ok()?) {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}
fn interpret(apps: Option<&RawValue>, system: Option<&RawValue>) -> Option<WindowsTheme> {
    match (light(apps)?, light(system)?) {
        (true, true) => Some(WindowsTheme::Light),
        (false, false) => Some(WindowsTheme::Dark),
        _ => Some(WindowsTheme::Mixed),
    }
}
fn requested(mode: &str) -> Result<[u32; 2], String> {
    match mode {
        "light" => Ok([1, 1]),
        "dark" => Ok([0, 0]),
        _ => Err("Choose Light or Dark for Windows appearance".into()),
    }
}
fn broadcast() -> Option<String> {
    let area = wide("ImmersiveColorSet");
    let mut result = 0;
    // Pointer-bearing system message must finish synchronously. Short timeout per
    // recipient, on Tauri's existing blocking pool, never on the UI/native worker.
    let sent = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            area.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            100,
            &mut result,
        )
    };
    (sent == 0).then(|| "Windows preferences were saved, but some applications did not acknowledge the theme refresh.".into())
}
#[tauri::command]
pub fn get_windows_theme() -> Result<Option<WindowsTheme>, String> {
    match Key::open(false)? {
        Some(key) => key.mode(),
        None => Ok(None),
    }
}
fn apply(mode: &str) -> Result<ThemeChange, String> {
    let values = requested(mode)?;
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let key = Key::open(true)?.ok_or("Current-user theme key unavailable")?;
    let previous_apps = key.read(APPS)?;
    // Read both before writing, so errors never cause an avoidable half-write.
    let _previous_system = key.read(SYSTEM)?;
    let apps = RawValue {
        kind: REG_DWORD,
        bytes: values[0].to_le_bytes().to_vec(),
    };
    let system = RawValue {
        kind: REG_DWORD,
        bytes: values[1].to_le_bytes().to_vec(),
    };
    key.write(APPS, Some(&apps))?;
    if let Err(error) = key.write(SYSTEM, Some(&system)) {
        let rollback = key.write(APPS, previous_apps.as_ref());
        let _ = broadcast();
        return Err(match rollback {
            Ok(()) => error,
            Err(restore) => format!("{error}; restoring the app preference also failed: {restore}"),
        });
    }
    let warning = broadcast();
    if let Some(warning) = &warning {
        eprintln!("ORBIT Windows appearance: {warning}");
    }
    Ok(ThemeChange {
        mode: key.mode()?,
        warning,
    })
}
#[tauri::command]
pub async fn set_windows_theme(mode: String) -> Result<ThemeChange, String> {
    let result = tauri::async_runtime::spawn_blocking(move || apply(&mode))
        .await
        .map_err(|e| e.to_string())?;
    if let Err(error) = &result {
        eprintln!("ORBIT Windows appearance: {error}");
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn dword(value: u32) -> RawValue {
        RawValue {
            kind: REG_DWORD,
            bytes: value.to_le_bytes().to_vec(),
        }
    }
    #[test]
    fn reads_actual_light_dark_mixed_and_unknown_without_guessing() {
        let dark = dword(0);
        let light = dword(1);
        assert_eq!(
            interpret(Some(&dark), Some(&dark)),
            Some(WindowsTheme::Dark)
        );
        assert_eq!(
            interpret(Some(&light), Some(&light)),
            Some(WindowsTheme::Light)
        );
        assert_eq!(
            interpret(Some(&light), Some(&dark)),
            Some(WindowsTheme::Mixed)
        );
        assert_eq!(interpret(None, Some(&light)), None);
        assert_eq!(interpret(Some(&dword(2)), Some(&light)), None);
        assert_eq!(
            interpret(
                Some(&RawValue {
                    kind: REG_SZ,
                    bytes: vec![1, 0, 0, 0]
                }),
                Some(&light)
            ),
            None
        );
        assert_eq!(
            interpret(
                Some(&RawValue {
                    kind: REG_DWORD,
                    bytes: vec![1]
                }),
                Some(&light)
            ),
            None
        );
    }
    #[test]
    fn only_light_and_dark_requests_generate_consistent_dword_pairs() {
        assert_eq!(requested("light").unwrap(), [1, 1]);
        assert_eq!(requested("dark").unwrap(), [0, 0]);
        for invalid in ["mixed", "system", "", "LIGHT"] {
            assert!(requested(invalid).is_err());
        }
    }
}
