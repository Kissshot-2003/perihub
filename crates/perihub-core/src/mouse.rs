//! Logitech mouse state and operations, run on the device's dedicated thread.

use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;

use crate::config::write_backup;
use crate::error::{Error, Result};
use crate::hidpp::macros;
use crate::hidpp::models::MouseModel;
use crate::hidpp::profile::{self, ButtonAction, LedEffect, ProfileSettings};
use crate::hidpp::{DpiCapabilities, FirmwareEntity, HidppDevice, LedZone, OnboardInfo, ROM_SECTOR_BASE};
use crate::worker::DeviceTask;

/// Profile formats verified byte by byte on a real device. Others can be read
/// but not written.
const WRITABLE_PROFILE_FORMATS: &[u8] = &[2];
const ONBOARD_MODE: u8 = 1;

#[derive(Debug, Clone, Serialize)]
pub struct MouseInfo {
    pub product_id: u16,
    /// Model name from PeriHub's list, e.g. "G502 HERO".
    pub model: String,
    /// Name reported by the mouse itself.
    pub name: String,
    /// Tested with PeriHub: unverified models are read-only.
    pub verified: bool,
    /// Another program left the mouse under software control, so its onboard profiles are not in use.
    pub host_mode: bool,
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
    model: &'static MouseModel,
    pub info: MouseInfo,
}

/// Features that some mice lack (no LEDs, a different polling rate feature) leave an empty list.
fn missing_as_empty<T>(result: Result<Vec<T>>) -> Result<Vec<T>> {
    match result {
        Err(Error::FeatureMissing(_)) => Ok(Vec::new()),
        other => other,
    }
}

impl MouseTask {
    pub fn open(mut dev: HidppDevice, model: &'static MouseModel) -> Result<Self> {
        let onboard = dev.onboard_info()?;
        if onboard.memory_model != 1 || onboard.macro_format != 1 {
            return Err(Error::Protocol("onboard profile memory uses an unknown format".into()));
        }
        // If another program left the mouse in host mode, switch back to the onboard profiles.
        // Unverified models are only read: the interface explains the situation instead.
        let mut host_mode = dev.onboard_mode()? != ONBOARD_MODE;
        if host_mode && model.verified {
            log::info!("mouse in host mode: switching to onboard profiles");
            dev.set_onboard_mode(ONBOARD_MODE)?;
            host_mode = false;
        }
        let info = MouseInfo {
            product_id: model.product_id,
            model: model.name.into(),
            name: dev.device_name()?,
            verified: model.verified,
            host_mode,
            firmware: dev.firmware()?,
            dpi: dev.dpi_capabilities(0)?,
            report_rates_ms: missing_as_empty(dev.report_rates())?,
            led_zones: missing_as_empty(dev.led_zones())?,
            writable: model.verified && WRITABLE_PROFILE_FORMATS.contains(&onboard.profile_format),
            onboard,
        };
        Ok(Self { dev, model, info })
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
        let mut macro_sectors = HashMap::new();
        for number in 1..=self.profile_count() {
            let sector = self.read_profile_sector(number)?;
            let mut settings = ProfileSettings::parse(&sector, self.info.onboard.button_count as usize)?;
            self.read_macros(&mut settings, &mut macro_sectors)?;
            profiles.push(MouseProfile { number, enabled: enabled[number as usize - 1], settings });
        }
        Ok(MouseState {
            active_profile: self.dev.active_profile()?,
            active_dpi_index: self.dev.active_dpi_index()?,
            current_dpi: self.dev.sensor_dpi(0)?.0,
            profiles,
        })
    }

    /// User profile sector; if it was never written (invalid CRC), the factory one applies.
    fn read_profile_sector(&mut self, number: u8) -> Result<Vec<u8>> {
        let sector = self.dev.read_sector(number as u16, self.sector_size())?;
        if profile::crc_is_valid(&sector) {
            return Ok(sector);
        }
        self.read_factory_sector(number)
    }

    /// Fills in the steps of the macros bound to the profile's buttons. Macros PeriHub cannot read
    /// stay without steps; errors talking to the mouse are returned. `cache` keeps the sectors
    /// already read, since profiles can share macro sectors.
    fn read_macros(&mut self, settings: &mut ProfileSettings, cache: &mut HashMap<u16, Vec<u8>>) -> Result<()> {
        let size = self.sector_size();
        for action in settings.buttons.iter_mut().chain(settings.gshift_buttons.iter_mut()) {
            let ButtonAction::Macro { sector, offset, steps } = action else { continue };
            let read = |id: u16| -> Result<Vec<u8>> {
                if let Some(data) = cache.get(&id) {
                    return Ok(data.clone());
                }
                let data = self.dev.read_sector(id, size)?;
                cache.insert(id, data.clone());
                Ok(data)
            };
            *steps = match macros::parse(read, *sector, *offset) {
                Ok(parsed) => Some(parsed),
                Err(e @ (Error::Hid(_) | Error::Timeout)) => return Err(e),
                Err(e) => {
                    log::warn!("macro at sector {sector:#04x} offset {offset:#04x}: {e}");
                    None
                }
            };
        }
        Ok(())
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
        let dir = write_backup(&self.model.backup_dir(), label, &refs)?;
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

    /// Reloads the active profile so changes take effect immediately. Reselecting the same
    /// profile is not enough (verified on the G502 HERO): you have to switch to another one and back.
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
        // Better to go through an enabled profile: the behavior with disabled ones is not verified.
        let others = || (1..=self.profile_count()).filter(|&n| n != active);
        let other = others()
            .find(|&n| enabled[n as usize - 1])
            .or_else(|| others().next())
            .ok_or_else(|| Error::Invalid("a second profile is needed to reload settings".into()))?;
        self.dev.set_active_profile(other)?;
        self.dev.set_active_profile(active)
    }

    /// Raw read of a sector (diagnostics).
    pub fn read_raw_sector(&mut self, sector: u16) -> Result<Vec<u8>> {
        self.dev.read_sector(sector, self.sector_size())
    }

    /// Cancels DPI and LED previews by going back to the saved settings.
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
        // Writing macros is not supported yet: a button can keep a macro the profile already has
        // (or take it from another button), but cannot point to new memory.
        let known = ProfileSettings::parse(&sector, self.info.onboard.button_count as usize)?.macro_addresses();
        if settings.macro_addresses().iter().any(|address| !known.contains(address)) {
            return Err(Error::Invalid("new macros cannot be saved yet".into()));
        }
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

    pub fn active_profile(&mut self) -> Result<u8> {
        self.dev.active_profile()
    }

    pub fn set_active_profile(&mut self, number: u8) -> Result<()> {
        self.check_number(number)?;
        self.dev.set_active_profile(number)
    }

    pub fn set_active_dpi_index(&mut self, index: u8) -> Result<()> {
        self.dev.set_active_dpi_index(index)
    }

    /// DPI preview (not saved: the mouse goes back to the profile at the next level change).
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
