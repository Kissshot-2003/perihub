//! Effetti di illuminazione per la Apex 3 TKL.
//!
//! La tastiera conosce solo "colori fissi per zona" e "arcobaleno": tutti gli altri effetti
//! (respiro, ciclo colori, onda…) vengono calcolati qui e inviati fotogramma per fotogramma,
//! esattamente come fa SteelSeries GG.

use std::f32::consts::PI;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::apex::ZONES;
use crate::color::Rgb;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyboardLighting {
    Off,
    /// Un colore per ciascuna delle 8 zone, da sinistra a destra.
    Static { colors: [Rgb; ZONES] },
    /// Sfumatura fissa tra due colori.
    Gradient { from: Rgb, to: Rgb },
    /// I colori delle zone pulsano insieme.
    Breathing { colors: [Rgb; ZONES], period_ms: u32 },
    /// Tutte le zone scorrono insieme lungo la ruota dei colori.
    ColorCycle { period_ms: u32 },
    /// Arcobaleno che scorre sulle zone.
    Wave { period_ms: u32, reverse: bool },
    /// Arcobaleno integrato nella tastiera (non richiede il programma in esecuzione).
    HardwareRainbow,
}

impl Default for KeyboardLighting {
    fn default() -> Self {
        Self::Static { colors: [Rgb::new(0xFF, 0x52, 0x00); ZONES] }
    }
}

/// Cosa inviare alla tastiera per un dato istante.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Frame {
    Colors([Rgb; ZONES]),
    HardwareRainbow,
}

impl KeyboardLighting {
    pub fn is_animated(&self) -> bool {
        matches!(self, Self::Breathing { .. } | Self::ColorCycle { .. } | Self::Wave { .. })
    }

    pub fn frame(&self, elapsed: Duration) -> Frame {
        let phase = |period_ms: u32| {
            let period = period_ms.max(200) as f32 / 1000.0;
            (elapsed.as_secs_f32() / period).fract()
        };
        Frame::Colors(match self {
            Self::Off => [Rgb::BLACK; ZONES],
            Self::Static { colors } => *colors,
            Self::Gradient { from, to } => {
                std::array::from_fn(|i| from.lerp(*to, i as f32 / (ZONES - 1) as f32))
            }
            Self::Breathing { colors, period_ms } => {
                // Coseno rialzato: da spento a pieno e ritorno, con un minimo leggermente acceso.
                let level = 0.05 + 0.95 * (0.5 - 0.5 * (2.0 * PI * phase(*period_ms)).cos());
                colors.map(|c| c.scale(level))
            }
            Self::ColorCycle { period_ms } => [Rgb::from_hsv(phase(*period_ms), 1.0, 1.0); ZONES],
            Self::Wave { period_ms, reverse } => {
                let p = phase(*period_ms);
                std::array::from_fn(|i| {
                    let offset = i as f32 / ZONES as f32;
                    let hue = if *reverse { p + offset } else { p - offset };
                    Rgb::from_hsv(hue, 1.0, 1.0)
                })
            }
            Self::HardwareRainbow => return Frame::HardwareRainbow,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_ends_match_inputs() {
        let lighting = KeyboardLighting::Gradient { from: Rgb::new(255, 0, 0), to: Rgb::new(0, 0, 255) };
        let Frame::Colors(colors) = lighting.frame(Duration::ZERO) else { panic!() };
        assert_eq!(colors[0], Rgb::new(255, 0, 0));
        assert_eq!(colors[ZONES - 1], Rgb::new(0, 0, 255));
    }

    #[test]
    fn breathing_reaches_full_brightness_at_half_period() {
        let lighting = KeyboardLighting::Breathing { colors: [Rgb::WHITE; ZONES], period_ms: 2000 };
        let Frame::Colors(colors) = lighting.frame(Duration::from_millis(1000)) else { panic!() };
        assert_eq!(colors[0], Rgb::WHITE);
    }

    #[test]
    fn lighting_json_shape() {
        let json = serde_json::to_string(&KeyboardLighting::Wave { period_ms: 3000, reverse: false }).unwrap();
        assert_eq!(json, r#"{"type":"wave","period_ms":3000,"reverse":false}"#);
    }
}
