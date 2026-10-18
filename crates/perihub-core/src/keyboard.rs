//! Keyboard state and operations, run on the device's dedicated thread.

use std::ffi::CString;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::apex::input::KeyWatcher;
use crate::apex::{Apex3Tkl, KeyBinding, Layer, MAX_BRIGHTNESS};
use crate::config::{KeyboardConfig, write_backup};
use crate::effects::{Frame, KeyboardLighting, NO_PRESSES};
use crate::error::{Error, Result};
use crate::worker::DeviceTask;

const FRAME_INTERVAL: Duration = Duration::from_millis(33);
/// Static colors are resent now and then: after a suspend the keyboard may fall back to the rainbow.
const STATIC_REFRESH: Duration = Duration::from_secs(5);
/// After a reconnect the key interfaces may not be accessible yet: reading them is retried this often.
const KEYS_RETRY: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize)]
pub struct KeyboardInfo {
    pub name: &'static str,
    pub firmware: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Keymap {
    pub normal: Layer,
    pub steelseries: Layer,
}

/// Paths of the interfaces that report key presses (interface 0, and interface 2 for keys beyond six).
#[derive(Debug, Clone, Default)]
pub struct KeyInterfaces {
    pub boot: Option<CString>,
    pub bitmap: Option<CString>,
}

pub struct KeyboardTask {
    dev: Apex3Tkl,
    pub info: KeyboardInfo,
    lighting: KeyboardLighting,
    started: Instant,
    last_frame: Option<Instant>,
    key_interfaces: KeyInterfaces,
    /// Reads key presses, only while the lighting reacts to them.
    keys: Option<KeyWatcher>,
    keys_failed: Option<Instant>,
}

impl KeyboardTask {
    pub fn open(dev: Apex3Tkl, config: &KeyboardConfig, key_interfaces: KeyInterfaces) -> Result<Self> {
        let firmware = dev.firmware_version()?;
        let mut task = Self {
            dev,
            info: KeyboardInfo { name: "SteelSeries Apex 3 TKL", firmware },
            lighting: config.lighting.clone(),
            started: Instant::now(),
            last_frame: None,
            key_interfaces,
            keys: None,
            keys_failed: None,
        };
        task.dev.set_brightness(config.brightness.min(MAX_BRIGHTNESS))?;
        if let Err(error) = task.watch_keys() {
            log::warn!("reactive lighting: {error}");
        }
        task.render()?;
        Ok(task)
    }

    /// Starts or stops reading key presses, depending on the lighting.
    fn watch_keys(&mut self) -> Result<()> {
        self.keys_failed = None;
        if self.lighting.fade().is_none() {
            self.keys = None;
        } else if self.keys.is_none() {
            let watcher = match &self.key_interfaces.boot {
                Some(boot) => KeyWatcher::start(boot, self.key_interfaces.bitmap.as_ref()),
                None => Err(Error::NotConnected),
            };
            match watcher {
                Ok(watcher) => self.keys = Some(watcher),
                Err(error) => {
                    self.keys_failed = Some(Instant::now());
                    return Err(Error::Invalid(format!("key presses cannot be read: {error}")));
                }
            }
        }
        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        let since_press = self.keys.as_ref().map_or(NO_PRESSES, KeyWatcher::since_press);
        match self.lighting.frame(self.started.elapsed(), &since_press) {
            Frame::Colors(colors) => self.dev.set_colors(&colors)?,
            Frame::HardwareRainbow => self.dev.set_hardware_rainbow()?,
        }
        self.last_frame = Some(Instant::now());
        Ok(())
    }

    /// Applies the lighting. If it reacts to keys and they cannot be read, the base color is shown
    /// and the error is returned.
    pub fn set_lighting(&mut self, lighting: KeyboardLighting) -> Result<()> {
        self.lighting = lighting;
        self.started = Instant::now();
        let watching = self.watch_keys();
        self.render()?;
        watching
    }

    /// Whether a pressed zone is still fading, so frames must keep coming.
    fn reacting(&self) -> bool {
        let (Some(keys), Some(fade)) = (&self.keys, self.lighting.fade()) else { return false };
        keys.since_press().iter().flatten().any(|&t| t < fade + 2 * FRAME_INTERVAL)
    }

    pub fn set_brightness(&mut self, level: u8) -> Result<()> {
        if level > MAX_BRIGHTNESS {
            return Err(Error::Invalid(format!("brightness {level}")));
        }
        self.dev.set_brightness(level)
    }

    /// Current brightness, which the user may have changed with SteelSeries + F11/F12.
    pub fn brightness(&self) -> Result<u8> {
        self.dev.brightness()
    }

    pub fn keymap(&self) -> Result<Keymap> {
        Ok(Keymap { normal: self.dev.read_layer(0)?, steelseries: self.dev.read_layer(1)? })
    }

    /// Changes the key map after saving a backup of it.
    pub fn apply_keymap(&mut self, changes: &[(u8, u8, KeyBinding)]) -> Result<Keymap> {
        let before = self.keymap()?;
        write_backup("apex3tkl", "keymap", &[("keymap.json", serde_json::to_string_pretty(&before)?.as_bytes())])?;
        let [normal, steelseries] = self.dev.apply_changes(changes)?;
        Ok(Keymap { normal, steelseries })
    }

    /// Resets all keys to their factory values.
    pub fn reset_keymap(&mut self) -> Result<Keymap> {
        let current = self.keymap()?;
        let mut changes = Vec::new();
        for (layer, map) in [(0u8, &current.normal), (1u8, &current.steelseries)] {
            for (&source, &binding) in map {
                let factory = KeyBinding::factory(layer, source);
                if binding != factory && crate::apex::keys::is_remappable_source(source) {
                    changes.push((layer, source, factory));
                }
            }
        }
        self.apply_keymap(&changes)
    }
}

impl DeviceTask for KeyboardTask {
    fn tick(&mut self) -> Result<Duration> {
        if self.keys_failed.is_some_and(|t| t.elapsed() >= KEYS_RETRY) {
            let _ = self.watch_keys();
        }
        // Reactive lighting checks for key presses every frame, but only sends colors when they change.
        if self.keys.is_some() {
            let refresh_due = self.last_frame.is_none_or(|t| t.elapsed() >= STATIC_REFRESH);
            if self.reacting() || refresh_due {
                self.render()?;
            }
            return Ok(FRAME_INTERVAL);
        }
        let interval = if self.lighting.is_animated() { FRAME_INTERVAL } else { STATIC_REFRESH };
        let due = self.last_frame.map_or(Duration::ZERO, |t| interval.saturating_sub(t.elapsed()));
        if due.is_zero() {
            self.render()?;
            return Ok(interval);
        }
        Ok(due)
    }
}
