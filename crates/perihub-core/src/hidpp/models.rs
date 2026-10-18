//! Logitech mice that PeriHub recognizes when connected with a USB cable.
//!
//! The list comes from libratbag's device files (MIT, commit 8235b5b): only G-series
//! mice and the MX518, which store their settings in onboard profiles (HID++ 0x8100).
//! Models that have not been tested on real hardware are read-only: see [`MouseModel::verified`].

/// HID++ device index for devices connected directly over USB (no receiver).
pub const WIRED_INDEX: u8 = 0xFF;

const G502_HERO: u16 = 0xC08B;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseModel {
    pub product_id: u16,
    pub name: &'static str,
    /// Device index the mouse answers on (libratbag's `DeviceIndex`).
    pub device_index: u8,
    /// Tested with PeriHub on a real device. Unverified models only get read commands.
    pub verified: bool,
}

impl MouseModel {
    /// Folder name for backups. The G502 HERO keeps the one used before other models existed.
    pub fn backup_dir(&self) -> String {
        if self.product_id == G502_HERO { "g502".into() } else { format!("logitech-{:04x}", self.product_id) }
    }
}

const fn model(product_id: u16, name: &'static str) -> MouseModel {
    MouseModel { product_id, name, device_index: WIRED_INDEX, verified: false }
}

pub const MODELS: &[MouseModel] = &[
    MouseModel { verified: true, ..model(G502_HERO, "G502 HERO") },
    model(0xC07D, "G502 Proteus Core"),
    model(0xC332, "G502 Proteus Spectrum"),
    model(0xC08D, "G502 HERO Wireless"),
    model(0xC099, "G502 X"),
    model(0xC098, "G502 X Wireless"),
    model(0xC095, "G502 X PLUS"),
    model(0xC085, "G Pro"),
    model(0xC08C, "G Pro"),
    MouseModel { device_index: 1, ..model(0xC088, "G Pro Wireless") },
    model(0xC094, "G Pro X Superlight"),
    model(0xC084, "G102/G103/G203"),
    model(0xC092, "G102/G103/G203"),
    model(0xC09D, "G102/G103/G203"),
    model(0xC07F, "G302"),
    model(0xC080, "G303"),
    model(0xC097, "G303 Shroud Edition"),
    model(0xC07E, "G402"),
    model(0xC083, "G403"),
    model(0xC08F, "G403 HERO"),
    model(0xC082, "G403 Wireless"),
    model(0xC087, "G703"),
    model(0xC090, "G703 HERO"),
    model(0xC096, "G705"),
    model(0xC081, "G900"),
    model(0xC086, "G903"),
    model(0xC091, "G903 HERO"),
    model(0xC08E, "MX518"),
];

pub fn find(product_id: u16) -> Option<&'static MouseModel> {
    MODELS.iter().find(|m| m.product_id == product_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_ids_are_unique() {
        for (i, a) in MODELS.iter().enumerate() {
            assert!(MODELS[i + 1..].iter().all(|b| b.product_id != a.product_id), "{:04x}", a.product_id);
        }
    }

    #[test]
    fn only_the_g502_hero_is_verified() {
        let verified: Vec<_> = MODELS.iter().filter(|m| m.verified).map(|m| m.product_id).collect();
        assert_eq!(verified, [G502_HERO]);
        assert_eq!(find(G502_HERO).unwrap().backup_dir(), "g502");
        assert_eq!(find(0xC094).unwrap().backup_dir(), "logitech-c094");
    }
}
