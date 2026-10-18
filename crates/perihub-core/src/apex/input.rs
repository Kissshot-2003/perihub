//! Key presses of the Apex 3 TKL, for the reactive lighting effect.
//!
//! The keyboard reports up to six keys on interface 0 (boot keyboard report) and any further keys
//! on interface 2 (one bit per key). Linux gives PeriHub a copy of these reports while the system
//! keeps reading them as usual. Only the lighting zone of each new press is kept: which keys were
//! pressed is neither stored nor logged, and the interfaces are open only while the effect is on.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};

use super::ZONES;
use super::keys::zone_of;
use crate::error::Result;

/// How often the reading threads check whether they should stop.
const READ_TIMEOUT_MS: i32 = 200;

type Hits = Arc<Mutex<[Option<Instant>; ZONES]>>;

/// Keys held down in one report, one bit per HID usage.
#[derive(Clone, Copy, Default, PartialEq)]
struct Pressed([u64; 4]);

impl Pressed {
    fn set(&mut self, usage: u8) {
        self.0[usize::from(usage / 64)] |= 1 << (usage % 64);
    }

    fn contains(&self, usage: u8) -> bool {
        self.0[usize::from(usage / 64)] & (1 << (usage % 64)) != 0
    }

    /// Boot keyboard report: modifier bits, a reserved byte, then up to six usages.
    fn from_boot_report(report: &[u8]) -> Self {
        let mut keys = Self::default();
        if let [modifiers, _, usages @ ..] = report {
            (0..8).filter(|bit| modifiers >> bit & 1 != 0).for_each(|bit| keys.set(0xE0 + bit));
            // 0x01 means "too many keys" (rollover error), not a key.
            usages.iter().filter(|&&u| u > 0x01).for_each(|&u| keys.set(u));
        }
        keys
    }

    /// Bitmap report: bit n of the report is HID usage n.
    fn from_bitmap(report: &[u8]) -> Self {
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

/// Reads key presses in the background until dropped.
pub struct KeyWatcher {
    hits: Hits,
    stop: Arc<AtomicBool>,
}

impl KeyWatcher {
    /// Opens the keyboard's key interfaces: `boot` (interface 0) and `bitmap` (interface 2).
    pub fn start(boot: &CString, bitmap: Option<&CString>) -> Result<Self> {
        let api = HidApi::new()?;
        let hits: Hits = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let mut readers = vec![(api.open_path(boot)?, Pressed::from_boot_report as fn(&[u8]) -> Pressed)];
        if let Some(path) = bitmap {
            readers.push((api.open_path(path)?, Pressed::from_bitmap));
        }
        for (dev, parse) in readers {
            let (hits, stop) = (hits.clone(), stop.clone());
            thread::Builder::new().name("perihub-keys".into()).spawn(move || read_loop(dev, parse, &hits, &stop))?;
        }
        Ok(Self { hits, stop })
    }

    /// Time since the last press in each zone.
    pub fn since_press(&self) -> [Option<Duration>; ZONES] {
        self.hits.lock().unwrap().map(|hit| hit.map(|t| t.elapsed()))
    }
}

impl Drop for KeyWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn read_loop(dev: HidDevice, parse: fn(&[u8]) -> Pressed, hits: &Hits, stop: &AtomicBool) {
    let mut buf = [0u8; 64];
    let mut before = Pressed::default();
    while !stop.load(Ordering::SeqCst) {
        match dev.read_timeout(&mut buf, READ_TIMEOUT_MS) {
            Ok(0) => {}
            Ok(len) => {
                let now = parse(&buf[..len]);
                let zones: Vec<usize> = now.new_since(before).filter_map(zone_of).collect();
                if !zones.is_empty() {
                    let mut hits = hits.lock().unwrap();
                    zones.into_iter().for_each(|z| hits[z] = Some(Instant::now()));
                }
                before = now;
            }
            // Unplugged: the keyboard thread notices and the effect is set up again on reconnect.
            Err(_) => break,
        }
    }
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
    fn every_key_of_the_board_has_a_zone() {
        for usage in (0x04..=0x52).chain(0xE0..=0xE7).filter(|&u| !matches!(u, 0x46..=0x48)) {
            assert!(zone_of(usage).is_some(), "0x{usage:02x}");
        }
    }
}
