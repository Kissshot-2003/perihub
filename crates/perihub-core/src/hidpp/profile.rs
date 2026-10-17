//! Formato dei profili salvati nella memoria del mouse (HID++ 0x8100, formato "G303"/2).
//!
//! Le modifiche vengono sempre applicate sopra i byte originali del settore: i campi che
//! non gestiamo (pulsanti G-Shift non modellati, LED alternativi, nome, macro…) restano intatti.

use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::error::{Error, Result};

const OFF_REPORT_RATE: usize = 0;
const OFF_DEFAULT_DPI: usize = 1;
const OFF_SHIFT_DPI: usize = 2;
const OFF_DPI: usize = 3;
const OFF_BUTTONS: usize = 32;
const OFF_GSHIFT_BUTTONS: usize = 96;
const OFF_LEDS: usize = 208;
const LED_LEN: usize = 11;

pub const DPI_SLOTS: usize = 5;
pub const BUTTON_SLOTS: usize = 16;
pub const LED_SLOTS: usize = 2;
const MIN_SECTOR_SIZE: usize = 256;

pub fn crc_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        let temp = (crc >> 8) ^ byte as u16;
        crc <<= 8;
        let mut quick = temp ^ (temp >> 4);
        crc ^= quick;
        quick <<= 5;
        crc ^= quick;
        quick <<= 7;
        crc ^= quick;
    }
    crc
}

pub fn crc_is_valid(sector: &[u8]) -> bool {
    let n = sector.len();
    n >= 2 && crc_ccitt(&sector[..n - 2]) == u16::from_be_bytes([sector[n - 2], sector[n - 1]])
}

pub fn write_crc(sector: &mut [u8]) {
    let n = sector.len();
    let crc = crc_ccitt(&sector[..n - 2]);
    sector[n - 2..].copy_from_slice(&crc.to_be_bytes());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialFunction {
    TiltLeft,
    TiltRight,
    DpiUp,
    DpiDown,
    DpiCycle,
    DpiDefault,
    DpiShift,
    ProfileNext,
    ProfilePrev,
    ProfileCycle,
    GShift,
}

impl SpecialFunction {
    const ALL: [SpecialFunction; 11] = [
        Self::TiltLeft,
        Self::TiltRight,
        Self::DpiUp,
        Self::DpiDown,
        Self::DpiCycle,
        Self::DpiDefault,
        Self::DpiShift,
        Self::ProfileNext,
        Self::ProfilePrev,
        Self::ProfileCycle,
        Self::GShift,
    ];

    fn code(self) -> u8 {
        Self::ALL.iter().position(|&f| f == self).unwrap() as u8 + 1
    }

    fn from_code(code: u8) -> Option<Self> {
        Self::ALL.get((code as usize).checked_sub(1)?).copied()
    }
}

/// Cosa fa un pulsante del mouse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ButtonAction {
    /// Pulsante del mouse, 1 = sinistro, 2 = destro, 3 = centrale, 4 = indietro, 5 = avanti…
    Mouse { button: u8 },
    /// Tasto della tastiera (usage HID) con i modificatori HID standard (bit 0 = Ctrl sx, 1 = Shift sx, 2 = Alt sx, 3 = Win sx…).
    Key { modifiers: u8, key: u8 },
    /// Tasto multimediale (usage della pagina HID Consumer, es. 0x00CD = play/pausa).
    Consumer { usage: u16 },
    Special { function: SpecialFunction },
    Disabled,
    /// Macro o tipo non gestito: conservato byte per byte.
    Raw { bytes: [u8; 4] },
}

impl ButtonAction {
    pub fn decode(b: [u8; 4]) -> Self {
        match b {
            [0x80, 0x01, hi, lo] => {
                let mask = u16::from_be_bytes([hi, lo]);
                if mask.count_ones() == 1 {
                    return Self::Mouse { button: mask.trailing_zeros() as u8 + 1 };
                }
            }
            [0x80, 0x02, modifiers, key] => return Self::Key { modifiers, key },
            [0x80, 0x03, hi, lo] => return Self::Consumer { usage: u16::from_be_bytes([hi, lo]) },
            [0x90, code, 0xFF, 0xFF] => {
                if let Some(function) = SpecialFunction::from_code(code) {
                    return Self::Special { function };
                }
            }
            [0xFF, 0xFF, 0xFF, 0xFF] => return Self::Disabled,
            _ => {}
        }
        Self::Raw { bytes: b }
    }

    pub fn encode(&self) -> Result<[u8; 4]> {
        Ok(match *self {
            Self::Mouse { button } => {
                if !(1..=16).contains(&button) {
                    return Err(Error::Invalid(format!("mouse button {button}")));
                }
                let [hi, lo] = (1u16 << (button - 1)).to_be_bytes();
                [0x80, 0x01, hi, lo]
            }
            Self::Key { modifiers, key } => [0x80, 0x02, modifiers, key],
            Self::Consumer { usage } => {
                let [hi, lo] = usage.to_be_bytes();
                [0x80, 0x03, hi, lo]
            }
            Self::Special { function } => [0x90, function.code(), 0xFF, 0xFF],
            Self::Disabled => [0xFF; 4],
            Self::Raw { bytes } => bytes,
        })
    }
}

/// Effetto di una zona LED, nel formato a 11 byte condiviso da profili e comandi 0x8070.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LedEffect {
    Off,
    Fixed { color: Rgb },
    Breathing { color: Rgb, period_ms: u16, brightness: u8 },
    Cycle { period_ms: u16, brightness: u8 },
    /// Effetto non gestito: conservato byte per byte.
    Raw { bytes: [u8; LED_LEN] },
}

impl LedEffect {
    pub fn decode(b: &[u8]) -> Self {
        let raw: [u8; LED_LEN] = b[..LED_LEN].try_into().unwrap();
        let zero = |range: std::ops::Range<usize>| raw[range].iter().all(|&v| v == 0);
        let brightness = |v: u8| if v == 0 { 100 } else { v };
        match raw[0] {
            0x00 if zero(1..LED_LEN) => Self::Off,
            0x01 if zero(4..LED_LEN) => Self::Fixed { color: Rgb::from_bytes(&raw[1..4]) },
            0x0A if zero(6..7) && zero(8..LED_LEN) => Self::Breathing {
                color: Rgb::from_bytes(&raw[1..4]),
                period_ms: u16::from_be_bytes([raw[4], raw[5]]),
                brightness: brightness(raw[7]),
            },
            0x03 if zero(1..6) && zero(9..LED_LEN) => Self::Cycle {
                period_ms: u16::from_be_bytes([raw[6], raw[7]]),
                brightness: brightness(raw[8]),
            },
            _ => Self::Raw { bytes: raw },
        }
    }

    pub fn encode(&self) -> [u8; LED_LEN] {
        // 100% si codifica come 0, come fa il firmware.
        let intensity = |v: u8| if v >= 100 { 0 } else { v.max(1) };
        let mut out = [0u8; LED_LEN];
        match *self {
            Self::Off => {}
            Self::Fixed { color } => {
                out[0] = 0x01;
                out[1..4].copy_from_slice(&color.to_bytes());
            }
            Self::Breathing { color, period_ms, brightness } => {
                out[0] = 0x0A;
                out[1..4].copy_from_slice(&color.to_bytes());
                out[4..6].copy_from_slice(&period_ms.to_be_bytes());
                out[7] = intensity(brightness);
            }
            Self::Cycle { period_ms, brightness } => {
                out[0] = 0x03;
                out[6..8].copy_from_slice(&period_ms.to_be_bytes());
                out[8] = intensity(brightness);
            }
            Self::Raw { bytes } => out = bytes,
        }
        out
    }
}

/// Impostazioni di un profilo che l'interfaccia può modificare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSettings {
    /// Intervallo di polling in ms (1 = 1000 Hz, 2 = 500 Hz, 4 = 250 Hz, 8 = 125 Hz).
    pub report_rate_ms: u8,
    /// Livelli DPI attivi (da 1 a 5).
    pub dpi_levels: Vec<u16>,
    pub default_dpi_index: u8,
    /// Livello usato mentre si tiene premuto il pulsante "DPI Shift" (sniper).
    pub shift_dpi_index: u8,
    pub buttons: Vec<ButtonAction>,
    /// Funzioni dei pulsanti mentre G-Shift è premuto.
    pub gshift_buttons: Vec<ButtonAction>,
    pub leds: Vec<LedEffect>,
}

impl ProfileSettings {
    pub fn parse(sector: &[u8], button_count: usize) -> Result<Self> {
        check_layout(sector, button_count)?;
        let dpi_levels: Vec<u16> = (0..DPI_SLOTS)
            .map(|i| u16::from_le_bytes([sector[OFF_DPI + 2 * i], sector[OFF_DPI + 2 * i + 1]]))
            .take_while(|&dpi| dpi != 0 && dpi != 0xFFFF)
            .collect();
        let button = |base: usize, i: usize| {
            ButtonAction::decode(sector[base + 4 * i..base + 4 * i + 4].try_into().unwrap())
        };
        Ok(Self {
            report_rate_ms: sector[OFF_REPORT_RATE],
            dpi_levels,
            default_dpi_index: sector[OFF_DEFAULT_DPI],
            shift_dpi_index: sector[OFF_SHIFT_DPI],
            buttons: (0..button_count).map(|i| button(OFF_BUTTONS, i)).collect(),
            gshift_buttons: (0..button_count).map(|i| button(OFF_GSHIFT_BUTTONS, i)).collect(),
            leds: (0..LED_SLOTS)
                .map(|i| LedEffect::decode(&sector[OFF_LEDS + LED_LEN * i..]))
                .collect(),
        })
    }

    /// Controlla che i valori siano accettabili per il mouse.
    pub fn validate(&self, button_count: usize, dpi_min: u16, dpi_max: u16, rates_ms: &[u8]) -> Result<()> {
        if !rates_ms.contains(&self.report_rate_ms) {
            return Err(Error::Invalid(format!("polling interval {} ms", self.report_rate_ms)));
        }
        if self.dpi_levels.is_empty() || self.dpi_levels.len() > DPI_SLOTS {
            return Err(Error::Invalid("between 1 and 5 DPI levels are required".into()));
        }
        if let Some(dpi) = self.dpi_levels.iter().find(|&&d| d < dpi_min || d > dpi_max) {
            return Err(Error::Invalid(format!("{dpi} DPI is outside the {dpi_min}–{dpi_max} range")));
        }
        let levels = self.dpi_levels.len() as u8;
        if self.default_dpi_index >= levels || self.shift_dpi_index >= levels {
            return Err(Error::Invalid("default or shift DPI level does not exist".into()));
        }
        if self.buttons.len() != button_count || self.gshift_buttons.len() != button_count {
            return Err(Error::Invalid("wrong number of buttons".into()));
        }
        if self.leds.len() != LED_SLOTS {
            return Err(Error::Invalid("wrong number of LED zones".into()));
        }
        for action in self.buttons.iter().chain(&self.gshift_buttons) {
            action.encode()?;
        }
        Ok(())
    }

    /// Scrive le impostazioni sopra un settore esistente e ricalcola il CRC.
    pub fn apply(&self, sector: &mut [u8]) -> Result<()> {
        let button_count = self.buttons.len();
        check_layout(sector, button_count)?;
        if self.dpi_levels.len() > DPI_SLOTS || self.gshift_buttons.len() != button_count || self.leds.len() != LED_SLOTS {
            return Err(Error::Invalid("incomplete profile".into()));
        }
        sector[OFF_REPORT_RATE] = self.report_rate_ms;
        sector[OFF_DEFAULT_DPI] = self.default_dpi_index;
        sector[OFF_SHIFT_DPI] = self.shift_dpi_index;
        for i in 0..DPI_SLOTS {
            let dpi = self.dpi_levels.get(i).copied().unwrap_or(0);
            sector[OFF_DPI + 2 * i..OFF_DPI + 2 * i + 2].copy_from_slice(&dpi.to_le_bytes());
        }
        for (i, action) in self.buttons.iter().enumerate() {
            sector[OFF_BUTTONS + 4 * i..OFF_BUTTONS + 4 * i + 4].copy_from_slice(&action.encode()?);
        }
        for (i, action) in self.gshift_buttons.iter().enumerate() {
            sector[OFF_GSHIFT_BUTTONS + 4 * i..OFF_GSHIFT_BUTTONS + 4 * i + 4].copy_from_slice(&action.encode()?);
        }
        for (i, led) in self.leds.iter().enumerate() {
            sector[OFF_LEDS + LED_LEN * i..OFF_LEDS + LED_LEN * (i + 1)].copy_from_slice(&led.encode());
        }
        write_crc(sector);
        Ok(())
    }
}

fn check_layout(sector: &[u8], button_count: usize) -> Result<()> {
    if sector.len() < MIN_SECTOR_SIZE || button_count > BUTTON_SLOTS {
        return Err(Error::Invalid("unsupported profile format".into()));
    }
    Ok(())
}

/// Elenco dei profili abilitati, letto dal settore 0x0000.
pub fn parse_directory(sector: &[u8], profile_count: usize) -> Vec<bool> {
    let mut enabled = vec![false; profile_count];
    for entry in sector.as_chunks::<4>().0.iter().take(profile_count) {
        let address = u16::from_be_bytes([entry[0], entry[1]]);
        if address == 0xFFFF {
            break;
        }
        if let Some(slot) = (address as usize).checked_sub(1).filter(|&i| i < profile_count) {
            enabled[slot] = entry[2] != 0;
        }
    }
    enabled
}

pub fn build_directory(enabled: &[bool], sector_size: usize) -> Vec<u8> {
    let mut data = vec![0xFF; sector_size];
    let mut at = 0;
    for (i, &on) in enabled.iter().enumerate() {
        data[at..at + 4].copy_from_slice(&[0x00, i as u8 + 1, on as u8, 0x00]);
        at += 4;
    }
    data[at..at + 4].copy_from_slice(&[0xFF, 0xFF, 0x00, 0x00]);
    write_crc(&mut data);
    data
}
