use serde::Serialize;
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub present: Option<bool>,
    pub percent: Option<u8>,
    pub plugged: Option<bool>,
    pub charging: Option<bool>,
    pub remaining_seconds: Option<u32>,
}
fn decode(flags: u8, percent: u8, ac: u8, seconds: u32) -> Battery {
    let present = if flags == 255 {
        None
    } else {
        Some(flags & 128 == 0)
    };
    let has_battery = present == Some(true);
    Battery {
        present,
        percent: (has_battery && percent <= 100).then_some(percent),
        plugged: match ac {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
        charging: if has_battery {
            Some(flags & 8 != 0)
        } else {
            None
        },
        remaining_seconds: (has_battery && ac == 0 && seconds != u32::MAX).then_some(seconds),
    }
}
#[tauri::command]
pub fn get_battery() -> Result<Battery, String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
        let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        // Windows writes the complete, correctly aligned POD struct. The pointer is valid for this call.
        if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(decode(
            status.BatteryFlag,
            status.BatteryLifePercent,
            status.ACLineStatus,
            status.BatteryLifeTime,
        ))
    }
    #[cfg(not(windows))]
    {
        Err("Windows battery integration is unavailable on this platform".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_flags_are_not_charging_or_absent() {
        let b = decode(255, 255, 255, u32::MAX);
        assert_eq!(b.present, None);
        assert_eq!(b.percent, None);
        assert_eq!(b.charging, None);
        assert_eq!(b.plugged, None);
    }
    #[test]
    fn absent_battery_has_no_percentage_or_estimate() {
        let b = decode(128, 100, 1, 300);
        assert_eq!(b.present, Some(false));
        assert_eq!(b.percent, None);
        assert_eq!(b.remaining_seconds, None);
    }
    #[test]
    fn charging_does_not_invent_estimate() {
        let b = decode(8, 72, 1, 3600);
        assert_eq!(b.charging, Some(true));
        assert_eq!(b.percent, Some(72));
        assert_eq!(b.remaining_seconds, None);
    }
    #[test]
    fn discharge_and_unknown_estimates() {
        assert_eq!(decode(1, 70, 0, 3600).remaining_seconds, Some(3600));
        assert_eq!(decode(1, 70, 0, u32::MAX).remaining_seconds, None);
    }
    #[test]
    fn reads_real_windows_api() {
        let b = get_battery().expect("Windows power API");
        assert!(b.percent.is_none_or(|p| p <= 100));
        println!("Actual Windows power status: {b:?}");
    }
}
