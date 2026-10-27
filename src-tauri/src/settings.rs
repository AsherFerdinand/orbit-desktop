use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::Manager;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Settings {
    pub collapsed_artwork: bool,
    pub playback_progress: bool,
    pub quick_notes: bool,
    pub paper_stack: bool,
    pub note_animations: bool,
    pub compact_metrics: bool,
    pub theme: String,
    pub width: u32,
    pub height: u32,
    pub collapse_delay: f64, // Seconds; legacy integer JSON values retain their meaning.
    pub animation_speed: f64,
    pub battery_enabled: bool,
    pub system_enabled: bool,
    pub volume_notifications: bool,
    pub charging_notifications: bool,
    pub network_notifications: bool,
    pub refresh_interval: u32,
    pub notification_duration: u32,
    pub low_battery_threshold: u8,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            collapsed_artwork: true,
            playback_progress: true,
            quick_notes: true,
            paper_stack: true,
            note_animations: true,
            compact_metrics: true,
            theme: "system".into(),
            width: 420,
            height: 56,
            collapse_delay: 0.4,
            animation_speed: 1.0,
            battery_enabled: true,
            system_enabled: true,
            volume_notifications: true,
            charging_notifications: true,
            network_notifications: true,
            refresh_interval: 2,
            notification_duration: 4,
            low_battery_threshold: 20,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !["dark", "light", "system"].contains(&self.theme.as_str())
            || !(360..=480).contains(&self.width)
            || !(48..=64).contains(&self.height)
            || (!self.collapse_delay.is_finite() || !(0.1..=20.0).contains(&self.collapse_delay))
            || ![0.75, 1.0, 1.5].contains(&self.animation_speed)
            || !(1..=10).contains(&self.refresh_interval)
            || !(2..=10).contains(&self.notification_duration)
            || !(15..=40).contains(&self.low_battery_threshold)
        {
            return Err("Settings are outside supported ranges".into());
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct SettingsLock(pub Mutex<()>);
fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json"))
}
#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let data = fs::read(&path).map_err(|e| format!("Cannot read settings: {e}"))?;
    let s:Settings=serde_json::from_slice(&data).map_err(|e|format!("Settings file is invalid: {e}. Restore it from a backup or remove settings.json to reset."))?;
    s.validate()?;
    Ok(s)
}
#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    settings: Settings,
    lock: tauri::State<SettingsLock>,
) -> Result<(), String> {
    settings.validate()?;
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    let path = settings_path(&app)?;
    fs::create_dir_all(path.parent().ok_or("Invalid settings directory")?)
        .map_err(|e| e.to_string())?;
    write_settings(&path, &settings)?;
    if let Some(runtime) = app.try_state::<crate::runtime::Runtime>() {
        let _ = runtime.tx.send(crate::runtime::Message::Settings(settings));
    }
    Ok(())
}
fn write_settings(path: &std::path::Path, settings: &Settings) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    // Keep a recoverable previous version and replace the file only after a complete write.
    if path.exists() {
        let legacy = path.with_file_name("settings-v0.1.json");
        let old = fs::read(path).map_err(|e| e.to_string())?;
        if old == bytes {
            return Ok(());
        }
        let old: serde_json::Value = serde_json::from_slice(&old).map_err(|e| e.to_string())?;
        if old.get("systemEnabled").is_none() && !legacy.exists() {
            fs::copy(path, &legacy).map_err(|e| e.to_string())?;
        }
        let prior04 = path.with_file_name("settings-v0.3.json");
        if old.get("quickNotes").is_none() && !prior04.exists() {
            fs::copy(path, prior04).map_err(|e| e.to_string())?;
        }
        fs::copy(path, path.with_extension("json.bak")).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_replaces_and_backs_up() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("settings-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let first = Settings::default();
        write_settings(&path, &first).unwrap();
        write_settings(&path, &first).unwrap();
        assert!(
            !path.with_extension("json.bak").exists(),
            "unchanged settings must not rewrite/back up"
        );
        let second = Settings {
            theme: "light".into(),
            ..first.clone()
        };
        write_settings(&path, &second).unwrap();
        assert_eq!(
            serde_json::from_slice::<Settings>(&fs::read(&path).unwrap()).unwrap(),
            second
        );
        assert_eq!(
            serde_json::from_slice::<Settings>(&fs::read(path.with_extension("json.bak")).unwrap())
                .unwrap(),
            first
        );
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn migrates_03_without_resetting_preferences() {
        let bytes=br#"{"theme":"light","width":390,"height":50,"collapseDelay":9,"animationSpeed":0.75,"batteryEnabled":false,"volumeNotifications":false}"#;
        let settings: Settings = serde_json::from_slice(bytes).unwrap();
        assert_eq!(settings.width, 390);
        assert_eq!(settings.collapse_delay, 9.0);
        assert!(!settings.volume_notifications);
        assert!(!settings.battery_enabled);
        assert!(
            settings.quick_notes
                && settings.paper_stack
                && settings.collapsed_artwork
                && settings.note_animations
                && settings.compact_metrics
                && settings.playback_progress
        );
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("migration04-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(&path, bytes).unwrap();
        write_settings(&path, &settings).unwrap();
        assert_eq!(fs::read(dir.join("settings-v0.3.json")).unwrap(), bytes);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn migrates_01() {
        let s:Settings=serde_json::from_str(r#"{"theme":"light","width":420,"height":56,"collapseDelay":5,"animationSpeed":1,"batteryEnabled":true}"#).unwrap();
        assert_eq!(s.refresh_interval, 2);
        assert!(s.charging_notifications);
        assert_eq!(s.theme, "light");
    }
    #[test]
    fn defaults_and_roundtrip() {
        let s = Settings::default();
        s.validate().unwrap();
        assert_eq!(
            serde_json::from_str::<Settings>(&serde_json::to_string(&s).unwrap()).unwrap(),
            s
        );
    }
    #[test]
    fn rejects_invalid_ranges() {
        let mut s = Settings {
            width: 900,
            ..Settings::default()
        };
        assert!(s.validate().is_err());
        s.width = 420;
        s.animation_speed = f64::NAN;
        assert!(s.validate().is_err());
    }
    #[test]
    fn collapse_delay_accepts_fractional_seconds_and_preserves_legacy_values() {
        let defaults = Settings::default();
        assert_eq!(defaults.collapse_delay, 0.4);
        for (json, expected) in [
            (r#"{"collapseDelay":0.4}"#, 0.4),
            (r#"{"collapseDelay":5}"#, 5.0),
            (r#"{"collapseDelay":9}"#, 9.0),
        ] {
            let settings: Settings = serde_json::from_str(json).unwrap();
            settings.validate().unwrap();
            assert_eq!(settings.collapse_delay, expected);
            assert_eq!(
                serde_json::from_str::<Settings>(&serde_json::to_string(&settings).unwrap())
                    .unwrap(),
                settings
            );
        }
        for delay in [0.0, -1.0, 20.1, f64::NAN, f64::INFINITY] {
            assert!(Settings {
                collapse_delay: delay,
                ..Settings::default()
            }
            .validate()
            .is_err());
        }
    }
    #[test]
    fn system_theme_is_valid_and_manual_preferences_are_preserved() {
        assert_eq!(Settings::default().theme, "system");
        for theme in ["system", "light", "dark"] {
            let settings: Settings =
                serde_json::from_value(serde_json::json!({ "theme": theme })).unwrap();
            settings.validate().unwrap();
            assert_eq!(settings.theme, theme);
        }
    }
    #[test]
    fn missing_fields_use_defaults() {
        assert_eq!(
            serde_json::from_str::<Settings>("{}").unwrap(),
            Settings::default()
        );
    }
}
