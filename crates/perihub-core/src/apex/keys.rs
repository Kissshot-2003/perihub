//! Tasti della SteelSeries Apex 3 TKL (usage HID della pagina tastiera).

use serde::Serialize;

/// Tasto SteelSeries (selettore del secondo livello): non si può rimappare.
pub const STEELSERIES_KEY: u8 = 0xF0;

/// Gli unici usage che hanno uno slot nella mappa del firmware 3.0.4 (letti dal dispositivo:
/// 114 slot). Gli altri usage leggono memoria fuori dalla tabella e non vanno mai scritti.
pub fn is_mappable_slot(usage: u8) -> bool {
    matches!(usage, 0x04..=0x64 | 0x85 | 0x87..=0x8B | 0x90 | 0x91 | 0xE0..=0xE7 | STEELSERIES_KEY)
}

/// Tasti che si possono rimappare (tutti gli slot tranne il tasto SteelSeries).
pub fn is_remappable_source(usage: u8) -> bool {
    usage != STEELSERIES_KEY && is_mappable_slot(usage)
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct KeyInfo {
    pub usage: u8,
    pub name: &'static str,
}

/// Tasti che il firmware può emettere come destinazione di una rimappatura.
pub const TARGET_KEYS: &[KeyInfo] = &[
    k(0x04, "A"), k(0x05, "B"), k(0x06, "C"), k(0x07, "D"), k(0x08, "E"), k(0x09, "F"),
    k(0x0A, "G"), k(0x0B, "H"), k(0x0C, "I"), k(0x0D, "J"), k(0x0E, "K"), k(0x0F, "L"),
    k(0x10, "M"), k(0x11, "N"), k(0x12, "O"), k(0x13, "P"), k(0x14, "Q"), k(0x15, "R"),
    k(0x16, "S"), k(0x17, "T"), k(0x18, "U"), k(0x19, "V"), k(0x1A, "W"), k(0x1B, "X"),
    k(0x1C, "Y"), k(0x1D, "Z"),
    k(0x1E, "1"), k(0x1F, "2"), k(0x20, "3"), k(0x21, "4"), k(0x22, "5"), k(0x23, "6"),
    k(0x24, "7"), k(0x25, "8"), k(0x26, "9"), k(0x27, "0"),
    k(0x28, "Enter"), k(0x29, "Esc"), k(0x2A, "Backspace"), k(0x2B, "Tab"), k(0x2C, "Space"),
    k(0x2D, "Minus"), k(0x2E, "Equal"), k(0x2F, "LeftBracket"), k(0x30, "RightBracket"),
    k(0x31, "Backslash"), k(0x32, "NonUsHash"), k(0x33, "Semicolon"), k(0x34, "Quote"),
    k(0x35, "Grave"), k(0x36, "Comma"), k(0x37, "Period"), k(0x38, "Slash"), k(0x39, "CapsLock"),
    k(0x3A, "F1"), k(0x3B, "F2"), k(0x3C, "F3"), k(0x3D, "F4"), k(0x3E, "F5"), k(0x3F, "F6"),
    k(0x40, "F7"), k(0x41, "F8"), k(0x42, "F9"), k(0x43, "F10"), k(0x44, "F11"), k(0x45, "F12"),
    k(0x46, "PrintScreen"), k(0x47, "ScrollLock"), k(0x48, "Pause"), k(0x49, "Insert"),
    k(0x4A, "Home"), k(0x4B, "PageUp"), k(0x4C, "Delete"), k(0x4D, "End"), k(0x4E, "PageDown"),
    k(0x4F, "Right"), k(0x50, "Left"), k(0x51, "Down"), k(0x52, "Up"), k(0x53, "NumLock"),
    k(0x54, "KpDivide"), k(0x55, "KpMultiply"), k(0x56, "KpMinus"), k(0x57, "KpPlus"),
    k(0x58, "KpEnter"), k(0x59, "Kp1"), k(0x5A, "Kp2"), k(0x5B, "Kp3"), k(0x5C, "Kp4"),
    k(0x5D, "Kp5"), k(0x5E, "Kp6"), k(0x5F, "Kp7"), k(0x60, "Kp8"), k(0x61, "Kp9"),
    k(0x62, "Kp0"), k(0x63, "KpPeriod"), k(0x64, "NonUsBackslash"), k(0x65, "Menu"),
    k(0x67, "KpEqual"),
    k(0x68, "F13"), k(0x69, "F14"), k(0x6A, "F15"), k(0x6B, "F16"), k(0x6C, "F17"), k(0x6D, "F18"),
    k(0x6E, "F19"), k(0x6F, "F20"), k(0x70, "F21"), k(0x71, "F22"), k(0x72, "F23"), k(0x73, "F24"),
    k(0x85, "KpComma"), k(0x87, "Intl1"), k(0x88, "Intl2"), k(0x89, "Intl3"), k(0x8A, "Intl4"),
    k(0x8B, "Intl5"), k(0x90, "Lang1"), k(0x91, "Lang2"),
    k(0xE0, "LeftCtrl"), k(0xE1, "LeftShift"), k(0xE2, "LeftAlt"), k(0xE3, "LeftWin"),
    k(0xE4, "RightCtrl"), k(0xE5, "RightShift"), k(0xE6, "RightAlt"), k(0xE7, "RightWin"),
];

const fn k(usage: u8, name: &'static str) -> KeyInfo {
    KeyInfo { usage, name }
}

pub fn is_valid_target(usage: u8) -> bool {
    TARGET_KEYS.iter().any(|key| key.usage == usage)
}

/// Funzioni di fabbrica del livello SteelSeries (tasto SteelSeries + tasto).
pub fn factory_function(source: u8) -> Option<[u8; 5]> {
    Some(match source {
        0x42 => [0x62, 0x01, 0x00, 0x00, 0x00],
        0x43 => [0x62, 0x02, 0x01, 0x00, 0x00],
        0x44 => [0x62, 0x03, 0x01, 0x00, 0x00],
        0x45 => [0x62, 0x04, 0x01, 0x00, 0x00],
        0xE3 => [0x62, 0x05, 0x00, 0x00, 0x00],
        _ => return None,
    })
}
