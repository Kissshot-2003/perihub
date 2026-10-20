//! The keys that start software macros, read from the devices themselves.
//!
//! Mouse, on Linux: PeriHub opens the mouse's HID interface a second time and reads its keyboard
//! reports (report 1: modifiers, a reserved byte, up to six keys), which only carry what the mouse
//! sends: its buttons' key bindings and macros, never what is typed on a keyboard.
//!
//! Keyboard, on Linux: the Apex 3 TKL's key interfaces, which report every key typed on it. They
//! can be opened only with the optional rule for its keys (71-perihub-keys.rules, or a similar
//! rule of another program), and PeriHub reads them only while one of the keyboard's keys sends
//! the key of a macro.
//!
//! Windows keeps keyboards to itself, so the keys come from Raw Input, filtered to the device (see
//! `crate::rawinput`). Only presses of the trigger keys are acted on: every other key is dropped
//! as it arrives, and nothing is kept.

use std::ffi::CString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::TRIGGER_KEYS;
#[cfg(not(windows))]
use crate::apex::input::{HeldKeys, Pressed};
use crate::error::Result;

/// Report id of the keyboard reports on the G502's HID interface.
#[cfg_attr(windows, allow(dead_code))]
const KEYBOARD_REPORT: u8 = 0x01;

/// Where trigger keys come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The mouse's HID interface that also carries its keyboard reports, and its USB product id.
    Mouse { path: CString, product_id: u16 },
    /// The Apex 3 TKL's interfaces that report key presses: the boot report (interface 0) and the
    /// keys beyond six (interface 2).
    Keyboard { boot: Option<CString>, bitmap: Option<CString> },
}

/// Called with each trigger key pressed.
pub type OnPress = Arc<dyn Fn(u8) + Send + Sync>;

/// Keys of the mouse's keyboard report; `None` for its other reports (HID++), which must not
/// change the held keys.
#[cfg(not(windows))]
fn mouse_report(report: &[u8]) -> Option<Pressed> {
    let [KEYBOARD_REPORT, _modifiers, _reserved, keys @ ..] = report else { return None };
    let mut pressed = Pressed::default();
    keys.iter().filter(|&&k| k > 0x01).for_each(|&k| pressed.set(k));
    Some(pressed)
}

/// Reads a device's trigger keys until dropped.
pub struct Trigger {
    pub source: Source,
    stop: Arc<AtomicBool>,
    #[cfg(windows)]
    _keys: crate::rawinput::Subscription,
}

impl Trigger {
    pub fn start(source: Source, on_press: OnPress) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        {
            let device = match &source {
                Source::Mouse { product_id, .. } => format!("VID_046D&PID_{product_id:04X}"),
                Source::Keyboard { .. } => crate::apex::input::APEX.to_string(),
            };
            // A held key repeats its event: only a key going down counts.
            let held = std::sync::Mutex::new(Vec::new());
            let keys = crate::rawinput::subscribe(&device, move |key| {
                let mut held = held.lock().unwrap();
                if key.released {
                    held.retain(|&k| k != key.usage);
                } else if !held.contains(&key.usage) {
                    held.push(key.usage);
                    if TRIGGER_KEYS.contains(&key.usage) {
                        on_press(key.usage);
                    }
                }
            })?;
            Ok(Self { source, stop, _keys: keys })
        }
        #[cfg(not(windows))]
        {
            type Parse = fn(&[u8]) -> Option<Pressed>;
            let api = hidapi::HidApi::new()?;
            let readers: Vec<(hidapi::HidDevice, Parse)> = match &source {
                Source::Mouse { path, .. } => vec![(api.open_path(path)?, mouse_report)],
                Source::Keyboard { boot, bitmap } => {
                    let boot = boot.as_ref().ok_or(crate::Error::NotConnected)?;
                    let mut readers: Vec<(hidapi::HidDevice, Parse)> =
                        vec![(api.open_path(boot)?, |r| Some(Pressed::from_boot_report(r)))];
                    if let Some(path) = bitmap {
                        readers.push((api.open_path(path)?, |r| Some(Pressed::from_bitmap(r))));
                    }
                    readers
                }
            };
            let held = Arc::new(std::sync::Mutex::new(HeldKeys::default()));
            for (interface, (dev, parse)) in readers.into_iter().enumerate() {
                let (held, stop, on_press) = (held.clone(), stop.clone(), on_press.clone());
                std::thread::Builder::new().name("perihub-triggers".into()).spawn(move || {
                    let mut buf = [0u8; 64];
                    while !stop.load(Ordering::SeqCst) {
                        match dev.read_timeout(&mut buf, 200) {
                            Ok(0) => {}
                            Ok(len) => {
                                let Some(keys) = parse(&buf[..len]) else { continue };
                                let pressed = held.lock().unwrap().update(interface, keys);
                                pressed.into_iter().filter(|k| TRIGGER_KEYS.contains(k)).for_each(|k| on_press(k));
                            }
                            // Unplugged: the engine starts a new reader when the device is back.
                            Err(_) => break,
                        }
                    }
                    stop.store(true, Ordering::SeqCst);
                })?;
            }
            Ok(Self { source, stop })
        }
    }

    /// Still reading (the threads end when the device is unplugged).
    pub fn is_alive(&self) -> bool {
        !self.stop.load(Ordering::SeqCst)
    }
}

impl Drop for Trigger {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    fn presses(held: &mut HeldKeys, interface: usize, keys: Option<Pressed>) -> Vec<u8> {
        keys.map(|k| held.update(interface, k).into_iter().filter(|k| TRIGGER_KEYS.contains(k)).collect())
            .unwrap_or_default()
    }

    #[test]
    fn mouse_presses_of_trigger_keys_only() {
        let mut held = HeldKeys::default();
        // F19 (0x6E) pressed, with an ordinary key that is not a trigger.
        assert_eq!(presses(&mut held, 0, mouse_report(&[0x01, 0, 0, 0x6E, 0x04, 0, 0, 0, 0])), [0x6E]);
        // Still held: not pressed again.
        assert_eq!(presses(&mut held, 0, mouse_report(&[0x01, 0, 0, 0x6E, 0, 0, 0, 0, 0])), Vec::<u8>::new());
        // HID++ reports do not release it.
        assert_eq!(presses(&mut held, 0, mouse_report(&[0x11, 0xFF, 0x0B, 0x6E])), Vec::<u8>::new());
        assert_eq!(presses(&mut held, 0, mouse_report(&[0x01, 0, 0, 0x6E, 0, 0, 0, 0, 0])), Vec::<u8>::new());
        // Released, then pressed again.
        assert_eq!(presses(&mut held, 0, mouse_report(&[0x01, 0, 0, 0, 0, 0, 0, 0, 0])), Vec::<u8>::new());
        assert_eq!(presses(&mut held, 0, mouse_report(&[0x01, 0, 0, 0x6E, 0, 0, 0, 0, 0])), [0x6E]);
        // F20 is never a trigger.
        assert_eq!(presses(&mut held, 0, mouse_report(&[0x01, 0, 0, 0x6F, 0, 0, 0, 0, 0])), Vec::<u8>::new());
    }

    #[test]
    fn keyboard_trigger_moving_between_interfaces_is_one_press() {
        let mut held = HeldKeys::default();
        // The Apex sends F19 in its boot report (as seen on the real keyboard)…
        let boot = |keys: &[u8]| {
            let mut report = [0u8; 8];
            report[2..2 + keys.len()].copy_from_slice(keys);
            Some(Pressed::from_boot_report(&report))
        };
        assert_eq!(presses(&mut held, 0, boot(&[0x6E])), [0x6E]);
        // …then moves it to the bitmap report when more than six keys are down.
        let mut bitmap = [0u8; 32];
        bitmap[0x6E / 8] |= 1 << (0x6E % 8);
        assert_eq!(presses(&mut held, 1, Some(Pressed::from_bitmap(&bitmap))), Vec::<u8>::new());
        assert_eq!(presses(&mut held, 0, boot(&[0x04, 0x05, 0x06, 0x07, 0x08, 0x09])), Vec::<u8>::new());
    }
}
