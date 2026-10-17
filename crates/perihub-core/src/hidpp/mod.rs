//! Protocollo Logitech HID++ 2.0 (mouse cablati come il G502 HERO).
//!
//! Riferimenti: libratbag (MIT) e i dati letti dal dispositivo reale.

pub mod profile;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use hidapi::HidDevice;
use serde::Serialize;

use crate::error::{Error, HidppErrorCode, Result};
use profile::LedEffect;

const REPORT_SHORT: u8 = 0x10;
const REPORT_LONG: u8 = 0x11;
const LONG_LEN: usize = 20;
/// Indice usato dai dispositivi collegati direttamente via USB (senza ricevitore).
const DEVICE_INDEX: u8 = 0xFF;
const SOFTWARE_ID: u8 = 0x0B;
const TIMEOUT: Duration = Duration::from_millis(1000);

pub mod feature {
    pub const ROOT: u16 = 0x0000;
    pub const FEATURE_SET: u16 = 0x0001;
    pub const DEVICE_INFO: u16 = 0x0003;
    pub const DEVICE_NAME: u16 = 0x0005;
    pub const ADJUSTABLE_DPI: u16 = 0x2201;
    pub const REPORT_RATE: u16 = 0x8060;
    pub const COLOR_LED_EFFECTS: u16 = 0x8070;
    pub const ONBOARD_PROFILES: u16 = 0x8100;
}

/// Le sole funzioni HID++ che il programma può usare. Le altre (DFU, funzioni interne
/// Logitech 0x18xx, 0x00c2…) non vanno mai toccate.
const ALLOWED_FEATURES: &[u16] = &[
    feature::ROOT,
    feature::FEATURE_SET,
    feature::DEVICE_INFO,
    feature::DEVICE_NAME,
    feature::ADJUSTABLE_DPI,
    feature::REPORT_RATE,
    feature::COLOR_LED_EFFECTS,
    feature::ONBOARD_PROFILES,
];

/// Settori della memoria del mouse riservati ai profili di fabbrica (sola lettura).
pub const ROM_SECTOR_BASE: u16 = 0x0100;

#[derive(Debug, Clone, Serialize)]
pub struct DpiCapabilities {
    pub min: u16,
    pub max: u16,
    pub step: u16,
    /// Valori ammessi, se il sensore ne espone un elenco invece di un intervallo.
    pub list: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LedZone {
    pub index: u8,
    /// 1 = principale, 2 = logo (secondo la specifica HID++ 0x8070).
    pub location: u16,
    pub effects: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnboardInfo {
    pub memory_model: u8,
    pub profile_format: u8,
    pub macro_format: u8,
    pub profile_count: u8,
    pub rom_profile_count: u8,
    pub button_count: u8,
    pub sector_count: u8,
    pub sector_size: u16,
    pub has_gshift: bool,
    pub has_dpi_shift: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FirmwareEntity {
    pub kind: u8,
    pub name: String,
    pub version: String,
}

pub struct HidppDevice {
    dev: HidDevice,
    features: HashMap<u16, u8>,
}

impl HidppDevice {
    pub fn new(dev: HidDevice) -> Result<Self> {
        let mut device = Self { dev, features: HashMap::new() };
        let (major, _) = device.protocol_version()?;
        if major < 2 {
            return Err(Error::Protocol(format!("HID++ {major}.x non supportato")));
        }
        Ok(device)
    }

    fn transact(&mut self, index: u8, function: u8, params: &[u8]) -> Result<[u8; 16]> {
        if params.len() > LONG_LEN - 4 {
            return Err(Error::Invalid("troppi parametri HID++".into()));
        }
        let address = (function << 4) | SOFTWARE_ID;
        let mut packet = [0u8; LONG_LEN];
        packet[0] = REPORT_LONG;
        packet[1] = DEVICE_INDEX;
        packet[2] = index;
        packet[3] = address;
        packet[4..4 + params.len()].copy_from_slice(params);
        self.dev.write(&packet)?;

        let deadline = Instant::now() + TIMEOUT;
        let mut buf = [0u8; 64];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::Timeout);
            }
            let n = self.dev.read_timeout(&mut buf, remaining.as_millis().max(1) as i32)?;
            // Sulla stessa interfaccia arrivano anche i report tastiera/multimediali del mouse: li ignoriamo.
            if n < 7 || !(buf[0] == REPORT_SHORT || buf[0] == REPORT_LONG) || buf[1] != DEVICE_INDEX {
                continue;
            }
            if buf[2] == index && buf[3] == address {
                let mut out = [0u8; 16];
                let len = (n - 4).min(16);
                out[..len].copy_from_slice(&buf[4..4 + len]);
                return Ok(out);
            }
            if buf[2] == 0xFF && buf[3] == index && buf[4] == address {
                return Err(Error::Hidpp(HidppErrorCode(buf[5])));
            }
        }
    }

    fn feature_index(&mut self, id: u16) -> Result<u8> {
        if !ALLOWED_FEATURES.contains(&id) {
            return Err(Error::Invalid(format!("funzione HID++ {id:#06x} non consentita")));
        }
        if id == feature::ROOT {
            return Ok(0);
        }
        if let Some(&index) = self.features.get(&id) {
            return Ok(index);
        }
        let reply = self.transact(0, 0, &id.to_be_bytes())?;
        if reply[0] == 0 {
            return Err(Error::FeatureMissing(id));
        }
        self.features.insert(id, reply[0]);
        Ok(reply[0])
    }

    fn call(&mut self, feature: u16, function: u8, params: &[u8]) -> Result<[u8; 16]> {
        let index = self.feature_index(feature)?;
        self.transact(index, function, params)
    }

    pub fn has_feature(&mut self, id: u16) -> bool {
        self.feature_index(id).is_ok()
    }

    pub fn protocol_version(&mut self) -> Result<(u8, u8)> {
        let reply = self.transact(0, 1, &[0, 0, 0x5A])?;
        Ok((reply[0], reply[1]))
    }

    pub fn device_name(&mut self) -> Result<String> {
        let length = self.call(feature::DEVICE_NAME, 0, &[])?[0] as usize;
        let mut name = Vec::with_capacity(length);
        while name.len() < length {
            let chunk = self.call(feature::DEVICE_NAME, 1, &[name.len() as u8])?;
            let take = (length - name.len()).min(16);
            name.extend_from_slice(&chunk[..take]);
        }
        Ok(String::from_utf8_lossy(&name).trim_end_matches('\0').to_string())
    }

    pub fn firmware(&mut self) -> Result<Vec<FirmwareEntity>> {
        let count = self.call(feature::DEVICE_INFO, 0, &[])?[0];
        let mut out = Vec::new();
        for entity in 0..count {
            let r = self.call(feature::DEVICE_INFO, 1, &[entity])?;
            out.push(FirmwareEntity {
                kind: r[0],
                name: String::from_utf8_lossy(&r[1..4]).trim().to_string(),
                version: format!("{:02x}.{:02x}.{:02x}{:02x}", r[4], r[5], r[6], r[7]),
            });
        }
        Ok(out)
    }

    // ---- 0x2201 DPI regolabili ----

    pub fn dpi_capabilities(&mut self, sensor: u8) -> Result<DpiCapabilities> {
        let r = self.call(feature::ADJUSTABLE_DPI, 1, &[sensor])?;
        let mut caps = DpiCapabilities { min: u16::MAX, max: 0, step: 0, list: Vec::new() };
        for pair in r[1..].as_chunks::<2>().0 {
            let value = u16::from_be_bytes([pair[0], pair[1]]);
            if value == 0 {
                break;
            }
            if value > 0xE000 {
                caps.step = value - 0xE000;
            } else {
                caps.min = caps.min.min(value);
                caps.max = caps.max.max(value);
                caps.list.push(value);
            }
        }
        if caps.step > 0 {
            caps.list.clear();
        }
        if caps.max == 0 {
            return Err(Error::Protocol("elenco DPI vuoto".into()));
        }
        Ok(caps)
    }

    /// Restituisce (DPI attuali, DPI predefiniti).
    pub fn sensor_dpi(&mut self, sensor: u8) -> Result<(u16, u16)> {
        let r = self.call(feature::ADJUSTABLE_DPI, 2, &[sensor])?;
        Ok((u16::from_be_bytes([r[1], r[2]]), u16::from_be_bytes([r[3], r[4]])))
    }

    /// Cambia i DPI subito, senza scrivere nella memoria del mouse.
    pub fn set_sensor_dpi(&mut self, sensor: u8, dpi: u16) -> Result<()> {
        let [hi, lo] = dpi.to_be_bytes();
        self.call(feature::ADJUSTABLE_DPI, 3, &[sensor, hi, lo])?;
        Ok(())
    }

    // ---- 0x8060 frequenza di polling ----

    /// Intervalli supportati in millisecondi (1 ms = 1000 Hz).
    pub fn report_rates(&mut self) -> Result<Vec<u8>> {
        let mask = self.call(feature::REPORT_RATE, 0, &[])?[0];
        Ok((0..8).filter(|bit| mask & (1 << bit) != 0).map(|bit| bit + 1).collect())
    }

    pub fn report_rate(&mut self) -> Result<u8> {
        Ok(self.call(feature::REPORT_RATE, 1, &[])?[0])
    }

    // ---- 0x8070 effetti LED ----

    pub fn led_zones(&mut self) -> Result<Vec<LedZone>> {
        let count = self.call(feature::COLOR_LED_EFFECTS, 0, &[])?[0];
        let mut zones = Vec::new();
        for zone in 0..count {
            let info = self.call(feature::COLOR_LED_EFFECTS, 1, &[zone])?;
            let mut effects = Vec::new();
            for effect in 0..info[3] {
                let e = self.call(feature::COLOR_LED_EFFECTS, 2, &[zone, effect])?;
                effects.push(u16::from_be_bytes([e[2], e[3]]));
            }
            zones.push(LedZone { index: zone, location: u16::from_be_bytes([info[1], info[2]]), effects });
        }
        Ok(zones)
    }

    /// Applica un effetto LED subito, solo in RAM (anteprima): non tocca i profili salvati.
    /// `zone.effects` serve perché qui il mouse vuole la posizione dell'effetto nel suo elenco,
    /// mentre nei profili salvati si usa il codice dell'effetto.
    pub fn preview_led_effect(&mut self, zone: &LedZone, effect: &LedEffect) -> Result<()> {
        let mut encoded = effect.encode();
        let position = zone
            .effects
            .iter()
            .position(|&id| id == encoded[0] as u16)
            .ok_or_else(|| Error::Invalid("effetto non supportato da questa zona".into()))?;
        encoded[0] = position as u8;
        let mut params = [0u8; 13];
        params[0] = zone.index;
        params[1..12].copy_from_slice(&encoded);
        params[12] = 0;
        self.call(feature::COLOR_LED_EFFECTS, 3, &params)?;
        Ok(())
    }

    // ---- 0x8100 profili in memoria ----

    pub fn onboard_info(&mut self) -> Result<OnboardInfo> {
        let r = self.call(feature::ONBOARD_PROFILES, 0, &[])?;
        Ok(OnboardInfo {
            memory_model: r[0],
            profile_format: r[1],
            macro_format: r[2],
            profile_count: r[3],
            rom_profile_count: r[4],
            button_count: r[5],
            sector_count: r[6],
            sector_size: u16::from_be_bytes([r[7], r[8]]),
            has_gshift: r[9] & 0x03 == 0x02,
            has_dpi_shift: (r[9] & 0x0C) >> 2 == 0x02,
        })
    }

    /// 1 = profili in memoria del mouse, 2 = controllo da software.
    pub fn onboard_mode(&mut self) -> Result<u8> {
        Ok(self.call(feature::ONBOARD_PROFILES, 2, &[])?[0])
    }

    pub fn set_onboard_mode(&mut self, mode: u8) -> Result<()> {
        if mode != 1 && mode != 2 {
            return Err(Error::Invalid(format!("modalità {mode}")));
        }
        self.call(feature::ONBOARD_PROFILES, 1, &[mode])?;
        Ok(())
    }

    /// Profilo attivo, numerato da 1.
    pub fn active_profile(&mut self) -> Result<u8> {
        Ok(self.call(feature::ONBOARD_PROFILES, 4, &[])?[1])
    }

    pub fn set_active_profile(&mut self, number: u8) -> Result<()> {
        self.call(feature::ONBOARD_PROFILES, 3, &[0, number])?;
        Ok(())
    }

    pub fn active_dpi_index(&mut self) -> Result<u8> {
        Ok(self.call(feature::ONBOARD_PROFILES, 0x0B, &[])?[0])
    }

    pub fn set_active_dpi_index(&mut self, index: u8) -> Result<()> {
        if index > 4 {
            return Err(Error::Invalid(format!("livello DPI {index}")));
        }
        self.call(feature::ONBOARD_PROFILES, 0x0C, &[index])?;
        Ok(())
    }

    pub fn read_sector(&mut self, sector: u16, size: u16) -> Result<Vec<u8>> {
        let size = size as usize;
        let mut data = vec![0u8; size];
        let mut offset = 0usize;
        while offset < size {
            // Il firmware rifiuta letture oltre la fine del settore: l'ultimo blocco si allinea a size-16.
            let at = if size - offset < 16 { size - 16 } else { offset };
            let [s_hi, s_lo] = sector.to_be_bytes();
            let [o_hi, o_lo] = (at as u16).to_be_bytes();
            let chunk = self.call(feature::ONBOARD_PROFILES, 5, &[s_hi, s_lo, o_hi, o_lo])?;
            data[at..at + 16].copy_from_slice(&chunk);
            offset = at + 16;
        }
        Ok(data)
    }

    /// Scrive un settore utente (directory o profilo). I settori di fabbrica non sono scrivibili.
    pub fn write_sector(&mut self, sector: u16, data: &[u8], info: &OnboardInfo) -> Result<()> {
        if sector > info.profile_count as u16 {
            return Err(Error::Invalid(format!("settore {sector:#06x} non scrivibile")));
        }
        if data.len() != info.sector_size as usize || !data.len().is_multiple_of(16) {
            return Err(Error::Invalid("dimensione del settore errata".into()));
        }
        if !profile::crc_is_valid(data) {
            return Err(Error::Invalid("CRC del settore non valido".into()));
        }
        let [s_hi, s_lo] = sector.to_be_bytes();
        let [l_hi, l_lo] = (data.len() as u16).to_be_bytes();
        self.call(feature::ONBOARD_PROFILES, 6, &[s_hi, s_lo, 0, 0, l_hi, l_lo])?;
        for chunk in data.as_chunks::<16>().0 {
            self.call(feature::ONBOARD_PROFILES, 7, chunk)?;
        }
        self.call(feature::ONBOARD_PROFILES, 8, &[])?;
        Ok(())
    }
}
