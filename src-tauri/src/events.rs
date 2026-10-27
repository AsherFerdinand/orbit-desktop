use crate::{battery::Battery, settings::Settings};
use serde::Serialize;
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub priority: u8,
    pub duration_ms: u32,
}
impl Event {
    pub fn new(kind: &str, title: &str, detail: String, priority: u8, s: &Settings) -> Self {
        Self {
            kind: kind.into(),
            title: title.into(),
            detail,
            priority,
            duration_ms: s.notification_duration * 1000,
        }
    }
}
#[derive(Default)]
pub struct Transitions {
    battery: Option<Battery>,
    network: Option<bool>,
}
impl Transitions {
    pub fn battery(&mut self, current: &Battery, s: &Settings) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(old) = &self.battery {
            if s.charging_notifications && current.present == Some(true) {
                let detail = current
                    .percent
                    .map_or("Percentage unavailable".into(), |p| format!("{p}% battery"));
                match (old.plugged, current.plugged) {
                    (Some(false), Some(true)) => events.push(Event::new(
                        "CHARGER_CONNECTED",
                        "Power connected",
                        detail.clone(),
                        80,
                        s,
                    )),
                    (Some(true), Some(false)) => events.push(Event::new(
                        "CHARGER_DISCONNECTED",
                        "Power disconnected",
                        detail.clone(),
                        80,
                        s,
                    )),
                    _ => {}
                }
                if let (Some(previous), Some(next)) = (old.percent, current.percent) {
                    if current.plugged == Some(false) {
                        if previous >= 10 && next < 10 {
                            events.push(Event::new(
                                "BATTERY_CRITICAL",
                                "Battery critical",
                                detail.clone(),
                                100,
                                s,
                            ));
                        } else if previous >= s.low_battery_threshold
                            && next < s.low_battery_threshold
                        {
                            events.push(Event::new(
                                "BATTERY_LOW",
                                "Battery running low",
                                detail.clone(),
                                85,
                                s,
                            ));
                        }
                    }
                    if previous < 100 && next == 100 {
                        events.push(Event::new("BATTERY_FULL", "Fully charged", detail, 60, s));
                    }
                }
            }
        }
        // Unknown readings do not erase the last valid transition baseline.
        if current.present == Some(false) {
            self.battery = None;
        } else if let Some(old) = self.battery.as_mut() {
            if current.plugged.is_some() {
                old.plugged = current.plugged;
            }
            if current.percent.is_some() {
                old.percent = current.percent;
            }
            old.present = current.present.or(old.present);
        } else if current.present == Some(true) {
            self.battery = Some(current.clone());
        }
        events
    }
    pub fn network(&mut self, next: Option<bool>, s: &Settings) -> Vec<Event> {
        let mut events = Vec::new();
        if let (Some(old), Some(next)) = (self.network, next) {
            if s.network_notifications && old != next {
                events.push(Event::new(
                    if next {
                        "NETWORK_RECONNECTED"
                    } else {
                        "NETWORK_DISCONNECTED"
                    },
                    if next {
                        "Network reconnected"
                    } else {
                        "Network disconnected"
                    },
                    "Local adapter connectivity".into(),
                    if next { 40 } else { 75 },
                    s,
                ));
            }
        }
        if next.is_some() {
            self.network = next;
        }
        events
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn b(p: u8, plugged: bool) -> Battery {
        Battery {
            present: Some(true),
            percent: Some(p),
            plugged: Some(plugged),
            charging: Some(plugged),
            remaining_seconds: None,
        }
    }
    #[test]
    fn no_startup_or_duplicate_charging() {
        let mut t = Transitions::default();
        let s = Settings::default();
        assert!(t.battery(&b(80, true), &s).is_empty());
        assert!(t.battery(&b(80, true), &s).is_empty());
        assert_eq!(t.battery(&b(80, false), &s)[0].kind, "CHARGER_DISCONNECTED");
        assert!(t.battery(&b(80, false), &s).is_empty());
        assert_eq!(t.battery(&b(80, true), &s)[0].kind, "CHARGER_CONNECTED");
    }
    #[test]
    fn thresholds_and_critical_priority() {
        let mut t = Transitions::default();
        let s = Settings::default();
        t.battery(&b(21, false), &s);
        assert!(t.battery(&b(20, false), &s).is_empty());
        assert_eq!(t.battery(&b(19, false), &s)[0].kind, "BATTERY_LOW");
        assert!(t.battery(&b(18, false), &s).is_empty());
        let critical = t.battery(&b(9, false), &s);
        assert_eq!(critical[0].priority, 100);
        assert_eq!(critical[0].kind, "BATTERY_CRITICAL");
    }
    #[test]
    fn full_and_preferences() {
        let mut t = Transitions::default();
        let s = Settings::default();
        t.battery(&b(99, true), &s);
        assert_eq!(t.battery(&b(100, true), &s)[0].kind, "BATTERY_FULL");
        assert!(t.battery(&b(100, true), &s).is_empty());
        let off = Settings {
            charging_notifications: false,
            ..s
        };
        assert!(t.battery(&b(19, false), &off).is_empty());
    }
    #[test]
    fn network_first_unknown_and_reconnect() {
        let mut t = Transitions::default();
        let s = Settings::default();
        assert!(t.network(Some(true), &s).is_empty());
        assert!(t.network(None, &s).is_empty());
        assert_eq!(t.network(Some(false), &s)[0].kind, "NETWORK_DISCONNECTED");
        assert!(t.network(Some(false), &s).is_empty());
        assert_eq!(t.network(Some(true), &s)[0].kind, "NETWORK_RECONNECTED");
    }
}
