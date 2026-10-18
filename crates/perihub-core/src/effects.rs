//! Lighting effects for the Apex 3 TKL.
//!
//! The keyboard only knows "fixed colors per zone" and "rainbow": every other effect
//! (breathing, color cycle, wave…) is computed here and sent frame by frame,
//! exactly like SteelSeries GG does.

use std::f32::consts::PI;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::apex::ZONES;
use crate::color::Rgb;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyboardLighting {
    Off,
    /// One color for each of the 8 zones, left to right.
    Static {
        colors: [Rgb; ZONES],
    },
    /// Fixed gradient between two colors.
    Gradient {
        from: Rgb,
        to: Rgb,
    },
    /// The zone colors pulse together.
    Breathing {
        colors: [Rgb; ZONES],
        period_ms: u32,
    },
    /// All zones move together around the color wheel.
    ColorCycle {
        period_ms: u32,
    },
    /// Rainbow scrolling across the zones.
    Wave {
        period_ms: u32,
        reverse: bool,
    },
    /// The keyboard's built-in rainbow (does not need the app running).
    HardwareRainbow,
    /// The zone of each pressed key lights up and fades back to the base color.
    Reactive {
        base: Rgb,
        flash: Rgb,
        fade_ms: u32,
    },
}

/// No key pressed recently, for effects that do not react to keys.
pub const NO_PRESSES: [Option<Duration>; ZONES] = [None; ZONES];

impl Default for KeyboardLighting {
    fn default() -> Self {
        Self::Static { colors: [Rgb::new(0xFF, 0x52, 0x00); ZONES] }
    }
}

/// What to send to the keyboard at a given moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Frame {
    Colors([Rgb; ZONES]),
    HardwareRainbow,
}

impl KeyboardLighting {
    pub fn is_animated(&self) -> bool {
        matches!(self, Self::Breathing { .. } | Self::ColorCycle { .. } | Self::Wave { .. })
    }

    /// How long a pressed zone takes to fade back, for effects that react to keys.
    pub fn fade(&self) -> Option<Duration> {
        match self {
            Self::Reactive { fade_ms, .. } => Some(Duration::from_millis(u64::from((*fade_ms).max(50)))),
            _ => None,
        }
    }

    /// Colors at `elapsed` since the effect started; `since_press` is the time since the last key
    /// press in each zone (only used by the reactive effect).
    pub fn frame(&self, elapsed: Duration, since_press: &[Option<Duration>; ZONES]) -> Frame {
        let phase = |period_ms: u32| {
            let period = period_ms.max(200) as f32 / 1000.0;
            (elapsed.as_secs_f32() / period).fract()
        };
        Frame::Colors(match self {
            Self::Off => [Rgb::BLACK; ZONES],
            Self::Static { colors } => *colors,
            Self::Gradient { from, to } => std::array::from_fn(|i| from.lerp(*to, i as f32 / (ZONES - 1) as f32)),
            Self::Breathing { colors, period_ms } => {
                // Raised cosine: from off to full and back, with a slightly lit minimum.
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
            Self::Reactive { base, flash, .. } => {
                let fade = self.fade().unwrap_or_default();
                std::array::from_fn(|i| match since_press[i] {
                    Some(t) if t < fade => flash.lerp(*base, t.as_secs_f32() / fade.as_secs_f32()),
                    _ => *base,
                })
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_ends_match_inputs() {
        let lighting = KeyboardLighting::Gradient { from: Rgb::new(255, 0, 0), to: Rgb::new(0, 0, 255) };
        let Frame::Colors(colors) = lighting.frame(Duration::ZERO, &NO_PRESSES) else { panic!() };
        assert_eq!(colors[0], Rgb::new(255, 0, 0));
        assert_eq!(colors[ZONES - 1], Rgb::new(0, 0, 255));
    }

    #[test]
    fn breathing_reaches_full_brightness_at_half_period() {
        let lighting = KeyboardLighting::Breathing { colors: [Rgb::WHITE; ZONES], period_ms: 2000 };
        let Frame::Colors(colors) = lighting.frame(Duration::from_millis(1000), &NO_PRESSES) else { panic!() };
        assert_eq!(colors[0], Rgb::WHITE);
    }

    #[test]
    fn reactive_zone_fades_back_to_base() {
        let lighting = KeyboardLighting::Reactive { base: Rgb::BLACK, flash: Rgb::WHITE, fade_ms: 1000 };
        let mut presses = NO_PRESSES;
        presses[2] = Some(Duration::ZERO);
        presses[3] = Some(Duration::from_millis(2000));
        let Frame::Colors(colors) = lighting.frame(Duration::ZERO, &presses) else { panic!() };
        assert_eq!(colors[2], Rgb::WHITE);
        assert_eq!(colors[3], Rgb::BLACK);
        assert_eq!(colors[0], Rgb::BLACK);
    }

    #[test]
    fn lighting_json_shape() {
        let json = serde_json::to_string(&KeyboardLighting::Wave { period_ms: 3000, reverse: false }).unwrap();
        assert_eq!(json, r#"{"type":"wave","period_ms":3000,"reverse":false}"#);
    }
}
