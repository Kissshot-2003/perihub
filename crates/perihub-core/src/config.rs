//! Settings saved to disk, and backups.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::effects::KeyboardLighting;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardConfig {
    pub lighting: KeyboardLighting,
    /// Brightness 0–16 (the keyboard's scale).
    pub brightness: u8,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self { lighting: KeyboardLighting::default(), brightness: crate::apex::MAX_BRIGHTNESS }
    }
}

/// A keyboard lighting saved by name, so app profiles can use it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightingPreset {
    pub name: String,
    pub lighting: KeyboardLighting,
}

/// What to change while an app is in focus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppRule {
    /// Window class on Linux, executable name on Windows (compared ignoring case).
    pub app: String,
    /// Name of a lighting preset, or `None` to keep the keyboard as it is.
    pub lighting: Option<String>,
    /// Onboard profile to activate on the mouse, or `None` to keep it as it is.
    pub mouse_profile: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppProfiles {
    pub enabled: bool,
    pub rules: Vec<AppRule>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub keyboard: KeyboardConfig,
    /// Start hidden in the system tray.
    pub start_hidden: bool,
    pub lighting_presets: Vec<LightingPreset>,
    pub app_profiles: AppProfiles,
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("perihub")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

impl AppConfig {
    pub fn load() -> Self {
        match fs::read_to_string(config_path()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
                log::warn!("invalid config.json, using defaults: {error}");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        fs::create_dir_all(path.parent().unwrap())?;
        // Atomic write: a crash halfway through does not leave a truncated file.
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_string_pretty(self)?)?;
        fs::rename(temp, path)?;
        Ok(())
    }
}

/// Saves a backup before every write to a device's memory.
pub fn write_backup(device: &str, label: &str, files: &[(&str, &[u8])]) -> Result<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let dir = config_dir().join("backups").join(device).join(format!("{stamp}-{label}"));
    fs::create_dir_all(&dir)?;
    for (name, data) in files {
        fs::write(dir.join(name), data)?;
    }
    Ok(dir)
}
