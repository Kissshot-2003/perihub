//! Impostazioni salvate su disco e copie di sicurezza.

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
    /// Luminosità 0–16 (scala della tastiera).
    pub brightness: u8,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self { lighting: KeyboardLighting::default(), brightness: crate::apex::MAX_BRIGHTNESS }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub keyboard: KeyboardConfig,
    /// Avvia nascosto nell'area di notifica.
    pub start_hidden: bool,
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
        // Scrittura atomica: un crash a metà non lascia un file troncato.
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_string_pretty(self)?)?;
        fs::rename(temp, path)?;
        Ok(())
    }
}

/// Salva una copia di sicurezza prima di ogni scrittura nella memoria di un dispositivo.
pub fn write_backup(device: &str, label: &str, files: &[(&str, &[u8])]) -> Result<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let dir = config_dir().join("backups").join(device).join(format!("{stamp}-{label}"));
    fs::create_dir_all(&dir)?;
    for (name, data) in files {
        fs::write(dir.join(name), data)?;
    }
    Ok(dir)
}
