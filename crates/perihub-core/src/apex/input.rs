//! Key presses of the Apex 3 TKL, for the reactive lighting effect (and, with the same report
//! parsing, for the keys that start software macros: see `crate::softmacro`).
//!
//! Linux: the keyboard reports up to six keys on interface 0 (boot keyboard report) and any further
//! keys on interface 2 (one bit per key), and Linux gives PeriHub a copy of these reports while the
//! system keeps reading them as usual. Windows keeps keyboards to itself, so the keys come from Raw
//! Input instead, filtered to the Apex.
//!
//! Only the lighting zone of each new press is kept: which keys were pressed is neither stored nor
//! logged, and keys are read only while the effect is on.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(not(windows))]
use std::thread;
use std::time::{Duration, Instant};

#[cfg(not(windows))]
use hidapi::{HidApi, HidDevice};

use super::ZONES;
use super::keys::zone_of;
use crate::error::Result;

/// Raw Input device names contain the USB ids, e.g. `\\?\HID#VID_1038&PID_1622&MI_00#…`.
#[cfg(windows)]
pub(crate) const APEX: &str = "VID_1038&PID_1622";

/// How often the reading threads check whether they should stop.
#[cfg(not(windows))]
const READ_TIMEOUT_MS: i32 = 200;

/// Keys held down in one report, one bit per HID usage.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct Pressed([u64; 4]);

impl Pressed {
    pub(crate) fn set(&mut self, usage: u8) {
        self.0[usize::from(usage / 64)] |= 1 << (usage % 64);
    }

    fn contains(&self, usage: u8) -> bool {
        self.0[usize::from(usage / 64)] & (1 << (usage % 64)) != 0
    }

    /// Boot keyboard report: modifier bits, a reserved byte, then up to six usages.
    #[cfg_attr(windows, allow(dead_code))]
    pub(crate) fn from_boot_report(report: &[u8]) -> Self {
        let mut keys = Self::default();
        if let [modifiers, _, usages @ ..] = report {
            (0..8).filter(|bit| modifiers >> bit & 1 != 0).for_each(|bit| keys.set(0xE0 + bit));
            // 0x01 means "too many keys" (rollover error), not a key.
            usages.iter().filter(|&&u| u > 0x01).for_each(|&u| keys.set(u));
        }
        keys
    }

    /// Bitmap report: bit n of the report is HID usage n.
    #[cfg_attr(windows, allow(dead_code))]
    pub(crate) fn from_bitmap(report: &[u8]) -> Self {
        let mut keys = Self::default();
        for (i, byte) in report.iter().enumerate().take(32) {
            (0..8).filter(|bit| byte >> bit & 1 != 0).for_each(|bit| keys.set(i as u8 * 8 + bit));
        }
        keys
    }

    /// Keys down now that were not down before.
    fn new_since(self, before: Self) -> impl Iterator<Item = u8> {
        (0..=255u8).filter(move |&u| self.contains(u) && !before.contains(u))
    }
}

/// Keys held on the keyboard's two key interfaces.
#[derive(Default)]
pub(crate) struct HeldKeys([Pressed; 2]);

impl HeldKeys {
    /// Records the keys now held on one interface and returns the keys newly pressed. A key counts
    /// as pressed only if no interface was holding it, so a key moving from one report to the
    /// other is not pressed again.
    pub(crate) fn update(&mut self, interface: usize, now: Pressed) -> Vec<u8> {
        let before = self.union();
        self.0[interface] = now;
        self.union().new_since(before).collect()
    }

    fn union(&self) -> Pressed {
        let [a, b] = self.0;
        Pressed(std::array::from_fn(|i| a.0[i] | b.0[i]))
    }
}

/// What the reading threads share: the last press in each zone, and the keys held on each interface.
#[derive(Default)]
struct Shared {
    hits: [Option<Instant>; ZONES],
    held: HeldKeys,
}

impl Shared {
    fn update(&mut self, interface: usize, now: Pressed) {
        let at = Instant::now();
        for zone in self.held.update(interface, now).into_iter().filter_map(zone_of) {
            self.hits[zone] = Some(at);
        }
    }
}

/// Reads key presses in the background until dropped.
pub struct KeyWatcher {
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    #[cfg(windows)]
    _keys: crate::rawinput::Subscription,
}

impl KeyWatcher {
    /// Linux: opens the keyboard's key interfaces, `boot` (interface 0) and `bitmap` (interface 2).
    /// Windows: receives the keyboard's keys through Raw Input (the paths are not used).
    pub fn start(boot: Option<&CString>, bitmap: Option<&CString>) -> Result<Self> {
        let shared = Arc::<Mutex<Shared>>::default();
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        {
            let _ = (boot, bitmap);
            // A held key repeats its event: only changes to the held keys matter.
            let (held, target) = (Mutex::new(std::collections::HashSet::new()), shared.clone());
            let keys = crate::rawinput::subscribe(APEX, move |key| {
                let mut held = held.lock().unwrap();
                let changed = if key.released { held.remove(&key.usage) } else { held.insert(key.usage) };
                if changed {
                    let mut keys = Pressed::default();
                    held.iter().for_each(|&u| keys.set(u));
                    target.lock().unwrap().update(0, keys);
                }
            })?;
            Ok(Self { shared, stop, _keys: keys })
        }
        #[cfg(not(windows))]
        {
            let api = HidApi::new()?;
            let boot = boot.ok_or(crate::Error::NotConnected)?;
            let mut readers = vec![(api.open_path(boot)?, Pressed::from_boot_report as fn(&[u8]) -> Pressed)];
            if let Some(path) = bitmap {
                readers.push((api.open_path(path)?, Pressed::from_bitmap));
            }
            for (interface, (dev, parse)) in readers.into_iter().enumerate() {
                let (shared, stop) = (shared.clone(), stop.clone());
                thread::Builder::new()
                    .name("perihub-keys".into())
                    .spawn(move || read_loop(dev, parse, interface, &shared, &stop))?;
            }
            Ok(Self { shared, stop })
        }
    }

    /// Time since the last press in each zone.
    pub fn since_press(&self) -> [Option<Duration>; ZONES] {
        self.shared.lock().unwrap().hits.map(|hit| hit.map(|t| t.elapsed()))
    }
}

impl Drop for KeyWatcher {
    fn drop(&mut self) {
        // On Windows dropping the subscription also stops the keys.
        self.stop.store(true, Ordering::SeqCst);
    }
}

#[cfg(not(windows))]
fn read_loop(dev: HidDevice, parse: fn(&[u8]) -> Pressed, interface: usize, shared: &Mutex<Shared>, stop: &AtomicBool) {
    let mut buf = [0u8; 64];
    while !stop.load(Ordering::SeqCst) {
        match dev.read_timeout(&mut buf, READ_TIMEOUT_MS) {
            Ok(0) => {}
            Ok(len) => shared.lock().unwrap().update(interface, parse(&buf[..len])),
            // Unplugged: the keyboard thread notices and the effect is set up again on reconnect.
            Err(_) => break,
        }
    }
}

/// HID usage of a key from its scan code (set 1), as Windows reports it; `e0` is the extended-key prefix.
/// The key left of Enter on ISO boards has the same scan code as the ANSI backslash.
#[cfg(any(windows, test))]
#[rustfmt::skip]
pub(crate) fn scan_code_usage(code: u16, e0: bool) -> Option<u8> {
    Some(match (e0, code) {
        (false, 0x01) => 0x29,
        (false, 0x02..=0x0A) => 0x1E + (code - 0x02) as u8, // 1–9
        (false, 0x0B) => 0x27,
        (false, 0x0C) => 0x2D, (false, 0x0D) => 0x2E, (false, 0x0E) => 0x2A, (false, 0x0F) => 0x2B,
        (false, 0x10) => 0x14, (false, 0x11) => 0x1A, (false, 0x12) => 0x08, (false, 0x13) => 0x15,
        (false, 0x14) => 0x17, (false, 0x15) => 0x1C, (false, 0x16) => 0x18, (false, 0x17) => 0x0C,
        (false, 0x18) => 0x12, (false, 0x19) => 0x13, (false, 0x1A) => 0x2F, (false, 0x1B) => 0x30,
        (false, 0x1C) => 0x28, (false, 0x1D) => 0xE0,
        (false, 0x1E) => 0x04, (false, 0x1F) => 0x16, (false, 0x20) => 0x07, (false, 0x21) => 0x09,
        (false, 0x22) => 0x0A, (false, 0x23) => 0x0B, (false, 0x24) => 0x0D, (false, 0x25) => 0x0E,
        (false, 0x26) => 0x0F, (false, 0x27) => 0x33, (false, 0x28) => 0x34, (false, 0x29) => 0x35,
        (false, 0x2A) => 0xE1, (false, 0x2B) => 0x31,
        (false, 0x2C) => 0x1D, (false, 0x2D) => 0x1B, (false, 0x2E) => 0x06, (false, 0x2F) => 0x19,
        (false, 0x30) => 0x05, (false, 0x31) => 0x11, (false, 0x32) => 0x10, (false, 0x33) => 0x36,
        (false, 0x34) => 0x37, (false, 0x35) => 0x38, (false, 0x36) => 0xE5, (false, 0x38) => 0xE2,
        (false, 0x39) => 0x2C, (false, 0x3A) => 0x39,
        (false, 0x3B..=0x44) => 0x3A + (code - 0x3B) as u8, // F1–F10
        (false, 0x45) => 0x53, (false, 0x46) => 0x47, // Num Lock, Scroll Lock
        (false, 0x47) => 0x5F, (false, 0x48) => 0x60, (false, 0x49) => 0x61, (false, 0x4A) => 0x56,
        (false, 0x4B) => 0x5C, (false, 0x4C) => 0x5D, (false, 0x4D) => 0x5E, (false, 0x4E) => 0x57,
        (false, 0x4F) => 0x59, (false, 0x50) => 0x5A, (false, 0x51) => 0x5B, (false, 0x52) => 0x62,
        (false, 0x53) => 0x63, (false, 0x37) => 0x55, // keypad
        (false, 0x56) => 0x64, (false, 0x57) => 0x44, (false, 0x58) => 0x45,
        (false, 0x64..=0x6E) => 0x68 + (code - 0x64) as u8, // F13–F23
        (false, 0x76) => 0x73, // F24
        (true, 0x1C) => 0x58, (true, 0x35) => 0x54, (true, 0x37) => 0x46, // keypad Enter and /, Print Screen
        (true, 0x1D) => 0xE4, (true, 0x38) => 0xE6, (true, 0x47) => 0x4A, (true, 0x48) => 0x52,
        (true, 0x49) => 0x4B, (true, 0x4B) => 0x50, (true, 0x4D) => 0x4F, (true, 0x4F) => 0x4D,
        (true, 0x50) => 0x51, (true, 0x51) => 0x4E, (true, 0x52) => 0x49, (true, 0x53) => 0x4C,
        (true, 0x5B) => 0xE3, (true, 0x5C) => 0xE7, (true, 0x5D) => 0x65,
        _ => return None,
    })
}

/// Scan code (set 1) and extended prefix of a key from its HID usage: the reverse of [`scan_code_usage`].
#[cfg(any(windows, test))]
pub fn usage_scan_code(usage: u8) -> Option<(u16, bool)> {
    // The ISO key left of Enter shares its scan code with the ANSI backslash.
    if usage == 0x32 {
        return Some((0x2B, false));
    }
    [false, true]
        .into_iter()
        .flat_map(|e0| (1..0x80).map(move |code| (code, e0)))
        .find(|&(code, e0)| scan_code_usage(code, e0) == Some(usage))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_report_with_modifier() {
        // Left Ctrl + C
        let keys = Pressed::from_boot_report(&[0x01, 0, 0x06, 0, 0, 0, 0, 0]);
        assert!(keys.contains(0xE0) && keys.contains(0x06));
        assert_eq!(keys.new_since(Pressed::default()).count(), 2);
    }

    #[test]
    fn rollover_error_is_not_a_key() {
        let keys = Pressed::from_boot_report(&[0, 0, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01]);
        assert!(keys == Pressed::default());
    }

    #[test]
    fn bitmap_report_and_new_presses() {
        let mut report = [0u8; 32];
        report[0x0D / 8] |= 1 << (0x0D % 8); // J
        let before = Pressed::from_bitmap(&report);
        report[0x0A / 8] |= 1 << (0x0A % 8); // G
        let now = Pressed::from_bitmap(&report);
        assert_eq!(now.new_since(before).collect::<Vec<_>>(), [0x0A]);
    }

    #[test]
    fn a_key_moving_between_interfaces_is_not_a_new_press() {
        let mut shared = Shared::default();
        let mut boot = Pressed::default();
        boot.set(0x04); // A, in the boot report
        shared.update(0, boot);
        let first = shared.hits[zone_of(0x04).unwrap()];
        assert!(first.is_some());
        let mut bitmap = Pressed::default();
        bitmap.set(0x04); // the keyboard moves A to the bitmap report…
        shared.update(1, bitmap);
        shared.update(0, Pressed::default()); // …and drops it from the boot report
        assert_eq!(shared.hits[zone_of(0x04).unwrap()], first);
    }

    #[test]
    fn windows_scan_codes() {
        assert_eq!(scan_code_usage(0x01, false), Some(0x29)); // Esc
        assert_eq!(scan_code_usage(0x05, false), Some(0x21)); // 4
        assert_eq!(scan_code_usage(0x44, false), Some(0x43)); // F10
        assert_eq!(scan_code_usage(0x4D, true), Some(0x4F)); // right arrow
        assert_eq!(scan_code_usage(0x4D, false), Some(0x5E)); // keypad 6, for software macros
        for code in 0x01..=0x58 {
            // Keys the TKL has (no keypad, Print Screen, Scroll Lock or Pause) light a zone.
            if let Some(usage) = scan_code_usage(code, false).filter(|u| !matches!(u, 0x46..=0x48 | 0x53..=0x63)) {
                assert!(zone_of(usage).is_some(), "scan code {code:#04x}");
            }
        }
    }

    #[test]
    fn scan_codes_of_usages() {
        assert_eq!(usage_scan_code(0x04), Some((0x1E, false))); // A
        assert_eq!(usage_scan_code(0x6E), Some((0x6A, false))); // F19
        assert_eq!(usage_scan_code(0x73), Some((0x76, false))); // F24
        assert_eq!(usage_scan_code(0xE4), Some((0x1D, true))); // right Ctrl
        for usage in (0x04..=0x45).chain(0x68..=0x73).chain(0xE0..=0xE7) {
            assert!(usage_scan_code(usage).is_some(), "0x{usage:02x}");
        }
    }

    #[test]
    fn every_key_of_the_board_has_a_zone() {
        for usage in (0x04..=0x52).chain(0xE0..=0xE7).filter(|&u| !matches!(u, 0x46..=0x48)) {
            assert!(zone_of(usage).is_some(), "0x{usage:02x}");
        }
    }
}
