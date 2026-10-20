//! The keys that start software macros, read from the mouse itself.
//!
//! Linux: PeriHub opens the mouse's HID interface a second time and reads its keyboard reports
//! (report 1: modifiers, a reserved byte, up to six keys), which only carry what the mouse sends:
//! its buttons' key bindings and macros, never what is typed on a keyboard. Windows keeps the
//! mouse's keyboard to itself, so the keys come from Raw Input, filtered to the mouse
//! (see `crate::rawinput`). Only presses of the trigger keys are acted on; nothing else is kept.

use std::ffi::CString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::TRIGGER_KEYS;
use crate::error::Result;

/// Report id of the keyboard reports on the G502's HID interface.
#[cfg_attr(windows, allow(dead_code))]
const KEYBOARD_REPORT: u8 = 0x01;

/// Trigger keys newly pressed in a keyboard report, given the ones held before; updates `held`.
#[cfg_attr(windows, allow(dead_code))]
fn new_presses(report: &[u8], held: &mut Vec<u8>) -> Vec<u8> {
    let [KEYBOARD_REPORT, _modifiers, _reserved, keys @ ..] = report else { return Vec::new() };
    let now: Vec<u8> = keys.iter().copied().filter(|k| TRIGGER_KEYS.contains(k)).collect();
    let pressed = now.iter().copied().filter(|k| !held.contains(k)).collect();
    *held = now;
    pressed
}

/// Reads the mouse's trigger keys until dropped.
pub struct MouseTrigger {
    pub path: CString,
    stop: Arc<AtomicBool>,
    #[cfg(windows)]
    _keys: crate::rawinput::Subscription,
}

impl MouseTrigger {
    /// `path`: the mouse's HID interface that carries its keyboard reports (Linux);
    /// `product_id`: its USB product id, to recognize its keys (Windows).
    pub fn start(path: CString, product_id: u16, on_press: impl Fn(u8) + Send + 'static) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        {
            // A held key repeats its event: only a key going down counts.
            let held = std::sync::Mutex::new(Vec::new());
            let keys = crate::rawinput::subscribe(&format!("VID_046D&PID_{product_id:04X}"), move |key| {
                let mut held = held.lock().unwrap();
                if key.released {
                    held.retain(|&k| k != key.usage);
                } else if TRIGGER_KEYS.contains(&key.usage) && !held.contains(&key.usage) {
                    held.push(key.usage);
                    on_press(key.usage);
                }
            })?;
            Ok(Self { path, stop, _keys: keys })
        }
        #[cfg(not(windows))]
        {
            let _ = product_id;
            let dev = hidapi::HidApi::new()?.open_path(&path)?;
            let flag = stop.clone();
            std::thread::Builder::new().name("perihub-triggers".into()).spawn(move || {
                let mut buf = [0u8; 64];
                let mut held = Vec::new();
                while !flag.load(Ordering::SeqCst) {
                    match dev.read_timeout(&mut buf, 200) {
                        Ok(0) => {}
                        Ok(len) => new_presses(&buf[..len], &mut held).into_iter().for_each(&on_press),
                        // Unplugged: the engine starts a new reader when the mouse is back.
                        Err(_) => break,
                    }
                }
                flag.store(true, Ordering::SeqCst);
            })?;
            Ok(Self { path, stop })
        }
    }
}

impl MouseTrigger {
    /// Still reading (the thread ends when the mouse is unplugged).
    pub fn is_alive(&self) -> bool {
        !self.stop.load(Ordering::SeqCst)
    }
}

impl Drop for MouseTrigger {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presses_of_trigger_keys_only() {
        let mut held = Vec::new();
        // F19 (0x6E) pressed, with an ordinary key that is not a trigger.
        assert_eq!(new_presses(&[0x01, 0, 0, 0x6E, 0x04, 0, 0, 0, 0], &mut held), [0x6E]);
        // Still held: not pressed again.
        assert_eq!(new_presses(&[0x01, 0, 0, 0x6E, 0, 0, 0, 0, 0], &mut held), Vec::<u8>::new());
        // Released, then pressed again.
        assert_eq!(new_presses(&[0x01, 0, 0, 0, 0, 0, 0, 0, 0], &mut held), Vec::<u8>::new());
        assert_eq!(new_presses(&[0x01, 0, 0, 0x6E, 0, 0, 0, 0, 0], &mut held), [0x6E]);
        // F20 is never a trigger; HID++ reports are ignored.
        assert_eq!(new_presses(&[0x01, 0, 0, 0x6F, 0, 0, 0, 0, 0], &mut held), Vec::<u8>::new());
        assert_eq!(new_presses(&[0x11, 0xFF, 0x0B, 0x6E], &mut held), Vec::<u8>::new());
    }
}
