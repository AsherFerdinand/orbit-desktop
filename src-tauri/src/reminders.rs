//! Native toast delivery only. Scheduling is part of orbit-native's deadline loop.
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
use windows::{
    core::HSTRING,
    Data::Xml::Dom::XmlDocument,
    UI::Notifications::{NotificationSetting, ToastNotification, ToastNotificationManager},
};
pub fn notify(app: &tauri::AppHandle, text: &str) -> Result<(), String> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|e| e.to_string())?;
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                RoUninitialize();
            }
        }
    }
    let _apartment = Apartment;
    if !cfg!(debug_assertions) {
        static IDENTITY: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
        IDENTITY
            .get_or_init(|| register_identity(&app.config().identifier))
            .clone()?;
    }
    send(app, text).map_err(|e| e.to_string())
}
fn send(app: &tauri::AppHandle, text: &str) -> windows::core::Result<()> {
    // Uninstalled development builds have no registered ORBIT AUMID. This is
    // the standard winrt-notification fallback; Windows shows PowerShell as source.
    let identity = if cfg!(debug_assertions) {
        "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe"
    } else {
        &app.config().identifier
    };
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(identity))?;
    if notifier.Setting()? != NotificationSetting::Enabled {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(0x80070005u32 as i32),
            "Windows notifications are disabled for this application",
        ));
    }
    let xml = XmlDocument::new()?;
    xml.LoadXml(&HSTRING::from("<toast><visual><binding template=\"ToastGeneric\"><text>ORBIT</text><text/></binding></visual></toast>"))?;
    let body = xml.GetElementsByTagName(&HSTRING::from("text"))?.Item(1)?;
    body.AppendChild(&xml.CreateTextNode(&HSTRING::from(text))?)?;
    let toast = ToastNotification::CreateToastNotification(&xml)?;
    notifier.Show(&toast)
}

// Windows requires a Start Menu shortcut with a matching AppUserModelID for
// unpackaged desktop toasts. Create/update only ORBIT's own per-user shortcut.
// No autostart, notification-permission override or COM activation server.
fn register_identity(id: &str) -> Result<(), String> {
    use windows::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Programs, SHGetKnownFolderPath, KF_FLAG_DEFAULT},
    };
    let programs = unsafe { SHGetKnownFolderPath(&FOLDERID_Programs, KF_FLAG_DEFAULT, None) }
        .map_err(|e| e.to_string())?;
    let directory = unsafe { programs.to_string() };
    unsafe {
        CoTaskMemFree(Some(programs.0.cast()));
    }
    let directory = std::path::PathBuf::from(directory.map_err(|e| e.to_string())?).join("ORBIT");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    write_shortcut(&directory.join("ORBIT.lnk"), &exe, id).map_err(|e| e.to_string())
}
const APP_ID_KEY: windows::Win32::Foundation::PROPERTYKEY =
    windows::Win32::Foundation::PROPERTYKEY {
        fmtid: windows::core::GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
        pid: 5,
    };
fn write_shortcut(
    path: &std::path::Path,
    exe: &std::path::Path,
    id: &str,
) -> windows::core::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::{Interface, PCWSTR},
        Win32::{
            System::{
                Com::{
                    CoCreateInstance, IPersistFile,
                    StructuredStorage::{PropVariantChangeType, PROPVARIANT, PVCHF_DEFAULT},
                    CLSCTX_INPROC_SERVER,
                },
                Variant::VT_LPWSTR,
            },
            UI::Shell::{IShellLinkW, PropertiesSystem::IPropertyStore, ShellLink},
        },
    };
    let wide = |p: &std::path::Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>()
    };
    let exe = wide(exe);
    let path = wide(path);
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(PCWSTR(exe.as_ptr()))?;
        link.SetDescription(&HSTRING::from("ORBIT reminder notifications"))?;
        let store: IPropertyStore = link.cast()?;
        // Owned variants: conversion allocates the LPWSTR Windows expects and
        // windows-rs Drop releases it with PropVariantClear on every exit path.
        let mut value = PROPVARIANT::default();
        PropVariantChangeType(&mut value, &PROPVARIANT::from(id), PVCHF_DEFAULT, VT_LPWSTR)?;
        store.SetValue(&APP_ID_KEY, &value)?;
        store.Commit()?;
        let file: IPersistFile = link.cast()?;
        file.Save(PCWSTR(path.as_ptr()), true)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_shortcut_preserves_notification_identity() {
        use std::os::windows::ffi::OsStrExt;
        use windows::{
            core::{Interface, PCWSTR},
            Win32::{
                System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ},
                UI::Shell::{IShellLinkW, PropertiesSystem::IPropertyStore, ShellLink},
            },
        };
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.unwrap();
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("notification-test-{}.lnk", std::process::id()));
        let result = (|| -> windows::core::Result<()> {
            let exe = std::env::current_exe().unwrap();
            write_shortcut(&path, &exe, "com.orbit.desktop.test")?;
            write_shortcut(&path, &exe, "com.orbit.desktop.test")?;
            unsafe {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                let file: IPersistFile = link.cast()?;
                let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
                file.Load(PCWSTR(wide.as_ptr()), STGM_READ)?;
                let store: IPropertyStore = link.cast()?;
                let value = store.GetValue(&APP_ID_KEY)?;
                assert_eq!(value.to_string(), "com.orbit.desktop.test");
            }
            Ok(())
        })();
        let _ = std::fs::remove_file(path);
        unsafe {
            RoUninitialize();
        }
        result.unwrap();
    }
}
