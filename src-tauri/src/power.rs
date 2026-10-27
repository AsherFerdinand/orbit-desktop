use crate::runtime::Message;
use std::{ffi::c_void, sync::mpsc::Sender};
use windows_sys::Win32::{
    System::{
        Power::*,
        SystemServices::{GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING},
    },
    UI::WindowsAndMessaging::DEVICE_NOTIFY_CALLBACK,
};
// The owned context remains stable until both registrations have been unregistered.
pub struct PowerSubscription {
    handles: Vec<HPOWERNOTIFY>,
    _context: Box<Sender<Message>>,
}
unsafe extern "system" fn changed(context: *const c_void, _: u32, _: *const c_void) -> u32 {
    if !context.is_null() {
        let tx = unsafe { &*context.cast::<Sender<Message>>() };
        let _ = tx.send(Message::PowerChanged);
    }
    0
}
impl PowerSubscription {
    pub fn new(tx: Sender<Message>) -> Self {
        let context = Box::new(tx);
        let params = DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(changed),
            Context: (&*context as *const Sender<Message>).cast_mut().cast(),
        };
        let mut handles = Vec::new();
        for guid in [GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING] {
            let mut handle = std::ptr::null_mut();
            if unsafe {
                PowerSettingRegisterNotification(
                    &guid,
                    DEVICE_NOTIFY_CALLBACK,
                    (&params as *const DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS)
                        .cast_mut()
                        .cast(),
                    &mut handle,
                )
            } == 0
            {
                handles.push(handle as HPOWERNOTIFY);
            }
        }
        Self {
            handles,
            _context: context,
        }
    }
}
impl Drop for PowerSubscription {
    fn drop(&mut self) {
        for handle in &self.handles {
            unsafe {
                PowerSettingUnregisterNotification(*handle);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_power_callback_registration() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let subscription = PowerSubscription::new(tx);
        assert_eq!(
            subscription.handles.len(),
            2,
            "Both Windows power-setting subscriptions must register"
        );
        println!("Registered real AC/DC and battery-percentage Windows callbacks");
    }
}
