use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 8-bit RGB color, serialized as a `#rrggbb` string.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self::new(bytes[0], bytes[1], bytes[2])
    }

    pub fn to_bytes(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }

    pub fn from_hex(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#').unwrap_or(text);
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Multiplies the channels by `factor` (0.0–1.0).
    pub fn scale(self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        let c = |v: u8| (v as f32 * f).round() as u8;
        Self::new(c(self.r), c(self.g), c(self.b))
    }

    pub fn lerp(self, other: Rgb, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let c = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Self::new(c(self.r, other.r), c(self.g, other.g), c(self.b, other.b))
    }

    /// `hue` in turns (0.0–1.0), `sat` and `val` in 0.0–1.0.
    pub fn from_hsv(hue: f32, sat: f32, val: f32) -> Self {
        let h = hue.rem_euclid(1.0) * 6.0;
        let c = val * sat;
        let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = val - c;
        let ch = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        Self::new(ch(r), ch(g), ch(b))
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Rgb::from_hex(&text).ok_or_else(|| serde::de::Error::custom(format!("invalid color: {text}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c = Rgb::new(0x00, 0xdc, 0xff);
        assert_eq!(c.to_hex(), "#00dcff");
        assert_eq!(Rgb::from_hex("#00DCFF"), Some(c));
        assert_eq!(Rgb::from_hex("00dcff"), Some(c));
        assert_eq!(Rgb::from_hex("#00dcf"), None);
    }

    #[test]
    fn hsv_primaries() {
        assert_eq!(Rgb::from_hsv(0.0, 1.0, 1.0), Rgb::new(255, 0, 0));
        assert_eq!(Rgb::from_hsv(1.0 / 3.0, 1.0, 1.0), Rgb::new(0, 255, 0));
        assert_eq!(Rgb::from_hsv(2.0 / 3.0, 1.0, 1.0), Rgb::new(0, 0, 255));
    }
}
