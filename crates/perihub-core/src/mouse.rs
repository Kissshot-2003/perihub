//! Stato e operazioni del mouse Logitech, eseguite nel thread dedicato al dispositivo.

use std::time::Duration;

use serde::Serialize;

use crate::config::write_backup;
use crate::error::{Error, Result};
use crate::hidpp::profile::{self, LedEffect, ProfileSettings};
use crate::hidpp::{DpiCapabilities, FirmwareEntity, HidppDevice, LedZone, OnboardInfo, ROM_SECTOR_BASE};
use crate::worker::DeviceTask;

/// Formati di profilo verificati byte per byte su un dispositivo reale. Gli altri si possono
/// leggere ma non scrivere.
const WRITABLE_PROFILE_FORMATS: &[u8] = &[2];
const ONBOARD_MODE: u8 = 1;

#[derive(Debug, Clone, Serialize)]
pub struct MouseInfo {
    pub name: String,
    pub firmware: Vec<FirmwareEntity>,
    pub dpi: DpiCapabilities,
    pub report_rates_ms: Vec<u8>,
    pub led_zones: Vec<LedZone>,
    pub onboard: OnboardInfo,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MouseProfile {
    pub number: u8,
    pub enabled: bool,
    pub settings: ProfileSettings,
}

#[derive(Debug, Clone, Serialize)]
pub struct MouseState {
    pub active_profile: u8,
    pub active_dpi_index: u8,
    pub current_dpi: u16,
    pub profiles: Vec<MouseProfile>,
}

pub struct MouseTask {
    dev: HidppDevice,
    pub info: MouseInfo,
}

impl MouseTask {
    pub fn open(mut dev: HidppDevice) -> Result<Self> {
        let onboard = dev.onboard_info()?;
        if onboard.memory_model != 1 || onboard.macro_format != 1 {
            return Err(Error::Protocol("onboard profile memory uses an unknown format".into()));
        }
        // Se un altro programma ha lasciato il mouse in modalità software, torniamo ai profili in memoria.
        if dev.onboard_mode()? != ONBOARD_MODE {
            log::info!("mouse in host mode: switching to onboard profiles");
            dev.set_onboard_mode(ONBOARD_MODE)?;
        }
        let info = MouseInfo {
            name: dev.device_name()?,
            firmware: dev.firmware()?,
            dpi: dev.dpi_capabilities(0)?,
            report_rates_ms: dev.report_rates()?,
            led_zones: dev.led_zones()?,
            writable: WRITABLE_PROFILE_FORMATS.contains(&onboard.profile_format),
            onboard,
        };
        Ok(Self { dev, info })
    }

    fn sector_size(&self) -> u16 {
        self.info.onboard.sector_size
    }

    fn profile_count(&self) -> u8 {
        self.info.onboard.profile_count
    }

    fn check_number(&self, number: u8) -> Result<()> {
        if (1..=self.profile_count()).contains(&number) {
            Ok(())
        } else {
            Err(Error::Invalid(format!("profile {number}")))
        }
    }

    fn check_writable(&self) -> Result<()> {
        if self.info.writable {
            Ok(())
        } else {
            Err(Error::Invalid("saving is not supported on this mouse model yet".into()))
        }
    }

    pub fn state(&mut self) -> Result<MouseState> {
        let directory = self.dev.read_sector(0, self.sector_size())?;
        let enabled = if profile::crc_is_valid(&directory) {
            profile::parse_directory(&directory, self.profile_count() as usize)
        } else {
            vec![true; self.profile_count() as usize]
        };
        let mut profiles = Vec::new();
        for number in 1..=self.profile_count() {
            let sector = self.read_profile_sector(number)?;
            profiles.push(MouseProfile {
                number,
                enabled: enabled[number as usize - 1],
                settings: ProfileSettings::parse(&sector, self.info.onboard.button_count as usize)?,
            });
        }
        Ok(MouseState {
            active_profile: self.dev.active_profile()?,
            active_dpi_index: self.dev.active_dpi_index()?,
            current_dpi: self.dev.sensor_dpi(0)?.0,
            profiles,
        })
    }

    /// Settore del profilo utente; se non è mai stato scritto (CRC non valido) vale quello di fabbrica.
    fn read_profile_sector(&mut self, number: u8) -> Result<Vec<u8>> {
        let sector = self.dev.read_sector(number as u16, self.sector_size())?;
        if profile::crc_is_valid(&sector) {
            return Ok(sector);
        }
        self.read_factory_sector(number)
    }

    fn read_factory_sector(&mut self, number: u8) -> Result<Vec<u8>> {
        let rom = if number <= self.info.onboard.rom_profile_count { number } else { 1 };
        let mut sector = self.dev.read_sector(ROM_SECTOR_BASE + rom as u16, self.sector_size())?;
        profile::write_crc(&mut sector);
        Ok(sector)
    }

    fn backup_all(&mut self, label: &str) -> Result<()> {
        let mut files = Vec::new();
        for sector in 0..=self.profile_count() as u16 {
            files.push((format!("sector_{sector:04x}.bin"), self.dev.read_sector(sector, self.sector_size())?));
        }
        let refs: Vec<(&str, &[u8])> = files.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
        let dir = write_backup("g502", label, &refs)?;
        log::info!("mouse backup saved to {}", dir.display());
        Ok(())
    }

    fn write_verified(&mut self, sector: u16, data: &[u8]) -> Result<()> {
        self.dev.write_sector(sector, data, &self.info.onboard)?;
        let back = self.dev.read_sector(sector, self.sector_size())?;
        if back != data {
            return Err(Error::Protocol(format!(
                "sector {sector:#06x} read back differs from what was written (backup in {})",
                crate::config::config_dir().join("backups").display()
            )));
        }
        Ok(())
    }

    /// Ricarica il profilo attivo, così le modifiche hanno effetto subito. Riselezionare lo stesso
    /// profilo non basta (verificato sul G502 HERO): bisogna passare a un altro e tornare indietro.
    fn reload_if_active(&mut self, number: u8) -> Result<()> {
        if self.dev.active_profile()? == number {
            self.reload_active()?;
        }
        Ok(())
    }

    fn reload_active(&mut self) -> Result<()> {
        let active = self.dev.active_profile()?;
        let directory = self.dev.read_sector(0, self.sector_size())?;
        let enabled = if profile::crc_is_valid(&directory) {
            profile::parse_directory(&directory, self.profile_count() as usize)
        } else {
            vec![true; self.profile_count() as usize]
        };
        // Meglio passare per un profilo abilitato: il comportamento con quelli disattivati non è verificato.
        let others = || (1..=self.profile_count()).filter(|&n| n != active);
        let other = others()
            .find(|&n| enabled[n as usize - 1])
            .or_else(|| others().next())
            .ok_or_else(|| Error::Invalid("a second profile is needed to reload settings".into()))?;
        self.dev.set_active_profile(other)?;
        self.dev.set_active_profile(active)
    }

    /// Lettura grezza di un settore (diagnostica).
    pub fn read_raw_sector(&mut self, sector: u16) -> Result<Vec<u8>> {
        self.dev.read_sector(sector, self.sector_size())
    }

    /// Annulla le anteprime di DPI e LED tornando alle impostazioni salvate.
    pub fn revert_preview(&mut self) -> Result<()> {
        self.reload_active()
    }

    pub fn save_profile(&mut self, number: u8, settings: &ProfileSettings) -> Result<MouseState> {
        self.check_writable()?;
        self.check_number(number)?;
        let caps = &self.info.dpi;
        settings.validate(self.info.onboard.button_count as usize, caps.min, caps.max, &self.info.report_rates_ms)?;
        if caps.step > 0 && settings.dpi_levels.iter().any(|d| !(d - caps.min).is_multiple_of(caps.step)) {
            return Err(Error::Invalid(format!("DPI values must be multiples of {}", caps.step)));
        }
        let mut sector = self.read_profile_sector(number)?;
        settings.apply(&mut sector)?;
        self.backup_all(&format!("profile{number}"))?;
        self.write_verified(number as u16, &sector)?;
        self.reload_if_active(number)?;
        self.state()
    }

    pub fn restore_factory_profile(&mut self, number: u8) -> Result<MouseState> {
        self.check_writable()?;
        self.check_number(number)?;
        let sector = self.read_factory_sector(number)?;
        self.backup_all(&format!("factory{number}"))?;
        self.write_verified(number as u16, &sector)?;
        self.reload_if_active(number)?;
        self.state()
    }

    pub fn set_profile_enabled(&mut self, number: u8, enabled: bool) -> Result<MouseState> {
        self.check_writable()?;
        self.check_number(number)?;
        let mut flags = self.state()?.profiles.iter().map(|p| p.enabled).collect::<Vec<_>>();
        flags[number as usize - 1] = enabled;
        if !flags.iter().any(|&f| f) {
            return Err(Error::Invalid("at least one profile must stay enabled".into()));
        }
        let directory = profile::build_directory(&flags, self.sector_size() as usize);
        self.backup_all("directory")?;
        self.write_verified(0, &directory)?;
        self.state()
    }

    pub fn set_active_profile(&mut self, number: u8) -> Result<()> {
        self.check_number(number)?;
        self.dev.set_active_profile(number)
    }

    pub fn set_active_dpi_index(&mut self, index: u8) -> Result<()> {
        self.dev.set_active_dpi_index(index)
    }

    /// Anteprima dei DPI (non salvata: il mouse torna al profilo al prossimo cambio livello).
    pub fn preview_dpi(&mut self, dpi: u16) -> Result<()> {
        let caps = &self.info.dpi;
        if dpi < caps.min || dpi > caps.max {
            return Err(Error::Invalid(format!("{dpi} DPI")));
        }
        self.dev.set_sensor_dpi(0, dpi)
    }

    pub fn preview_led(&mut self, zone: u8, effect: &LedEffect) -> Result<()> {
        let zone = self
            .info
            .led_zones
            .iter()
            .find(|z| z.index == zone)
            .cloned()
            .ok_or_else(|| Error::Invalid(format!("LED zone {zone}")))?;
        self.dev.preview_led_effect(&zone, effect)
    }
}

impl DeviceTask for MouseTask {
    fn tick(&mut self) -> Result<Duration> {
        Ok(Duration::from_secs(3600))
    }
}
