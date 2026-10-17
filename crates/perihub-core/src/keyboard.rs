//! Stato e operazioni della tastiera, eseguite nel thread dedicato al dispositivo.

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::apex::{Apex3Tkl, KeyBinding, Layer, MAX_BRIGHTNESS};
use crate::config::{KeyboardConfig, write_backup};
use crate::effects::{Frame, KeyboardLighting};
use crate::error::{Error, Result};
use crate::worker::DeviceTask;

const FRAME_INTERVAL: Duration = Duration::from_millis(33);
/// Ogni tanto rimandiamo i colori fissi: dopo una sospensione la tastiera può tornare all'arcobaleno.
const STATIC_REFRESH: Duration = Duration::from_secs(5);

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

pub struct KeyboardTask {
    dev: Apex3Tkl,
    pub info: KeyboardInfo,
    lighting: KeyboardLighting,
    started: Instant,
    last_frame: Option<Instant>,
}

impl KeyboardTask {
    pub fn open(dev: Apex3Tkl, config: &KeyboardConfig) -> Result<Self> {
        let firmware = dev.firmware_version()?;
        let mut task = Self {
            dev,
            info: KeyboardInfo { name: "SteelSeries Apex 3 TKL", firmware },
            lighting: config.lighting.clone(),
            started: Instant::now(),
            last_frame: None,
        };
        task.dev.set_brightness(config.brightness.min(MAX_BRIGHTNESS))?;
        task.render()?;
        Ok(task)
    }

    fn render(&mut self) -> Result<()> {
        match self.lighting.frame(self.started.elapsed()) {
            Frame::Colors(colors) => self.dev.set_colors(&colors)?,
            Frame::HardwareRainbow => self.dev.set_hardware_rainbow()?,
        }
        self.last_frame = Some(Instant::now());
        Ok(())
    }

    pub fn set_lighting(&mut self, lighting: KeyboardLighting) -> Result<()> {
        self.lighting = lighting;
        self.started = Instant::now();
        self.render()
    }

    pub fn set_brightness(&mut self, level: u8) -> Result<()> {
        if level > MAX_BRIGHTNESS {
            return Err(Error::Invalid(format!("luminosità {level}")));
        }
        self.dev.set_brightness(level)
    }

    /// Luminosità attuale, che l'utente può aver cambiato con SteelSeries + F11/F12.
    pub fn brightness(&self) -> Result<u8> {
        self.dev.brightness()
    }

    pub fn keymap(&self) -> Result<Keymap> {
        Ok(Keymap { normal: self.dev.read_layer(0)?, steelseries: self.dev.read_layer(1)? })
    }

    /// Modifica la mappa dei tasti dopo averne salvato una copia di sicurezza.
    pub fn apply_keymap(&mut self, changes: &[(u8, u8, KeyBinding)]) -> Result<Keymap> {
        let before = self.keymap()?;
        write_backup("apex3tkl", "keymap", &[("keymap.json", serde_json::to_string_pretty(&before)?.as_bytes())])?;
        let [normal, steelseries] = self.dev.apply_changes(changes)?;
        Ok(Keymap { normal, steelseries })
    }

    /// Riporta tutti i tasti ai valori di fabbrica.
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
        let interval = if self.lighting.is_animated() { FRAME_INTERVAL } else { STATIC_REFRESH };
        let due = self.last_frame.map_or(Duration::ZERO, |t| interval.saturating_sub(t.elapsed()));
        if due.is_zero() {
            self.render()?;
            return Ok(interval);
        }
        Ok(due)
    }
}
