//! SteelSeries Apex 3 TKL (USB 1038:1622), interfaccia di configurazione MI_01 (usage page 0xFFC0).
//!
//! Protocollo RGB ricostruito dalla documentazione di OpenRGB; mappa tasti dal progetto
//! gorkemgun/apex-3-tkl (MIT). Il codice è riscritto da zero.

pub mod keys;

use std::collections::BTreeMap;
use std::thread::sleep;
use std::time::Duration;

use hidapi::HidDevice;
use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::error::{Error, Result};

pub const ZONES: usize = 8;
pub const MAX_BRIGHTNESS: u8 = 0x10;
pub const LAYERS: [u8; 2] = [0, 1];

const OUTPUT_LEN: usize = 65;
const FEATURE_LEN: usize = 257;
const READ_DELAY: Duration = Duration::from_millis(80);
const WRITE_DELAY: Duration = Duration::from_millis(250);
const BINDINGS_PER_QUERY: usize = 30;

/// Comandi inviati come report di output (illuminazione, volatili).
enum OutputCommand<'a> {
    Colors(&'a [Rgb; ZONES]),
    Rainbow,
    Brightness(u8),
}

/// Comandi inviati come feature report. Sono gli unici consentiti: in particolare il comando
/// che riavvia la tastiera nel bootloader non è rappresentabile.
enum FeatureCommand<'a> {
    Version,
    Build,
    Brightness,
    Bindings { layer: u8, keys: &'a [u8] },
    SetBinding { layer: u8, source: u8, binding: [u8; 5] },
    Save,
}

impl FeatureCommand<'_> {
    fn encode(&self) -> Vec<u8> {
        match *self {
            Self::Version => vec![0x90, 0],
            Self::Build => vec![0x90, 1],
            Self::Brightness => vec![0xA3],
            Self::Bindings { layer, keys } => {
                let mut out = vec![0xAA, layer, keys.len() as u8];
                for &key in keys {
                    out.extend_from_slice(&[key, 0, 0, 0, 0, 0]);
                }
                out
            }
            Self::SetBinding { layer, source, binding } => {
                let mut out = vec![0x2A, layer, 1, source];
                out.extend_from_slice(&binding);
                out
            }
            Self::Save => vec![0x11],
        }
    }
}

/// Funzione di un tasto in uno dei due livelli.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyBinding {
    /// Emette questo tasto (usage HID).
    Key { usage: u8 },
    /// Funzione interna di fabbrica del livello SteelSeries (luminosità, blocco Win…).
    Function { code: u8, param: u8 },
    /// Nessuna funzione: sul livello SteelSeries vale il tasto normale.
    None,
    /// Valore sconosciuto, conservato com'è.
    Raw { bytes: [u8; 5] },
}

impl KeyBinding {
    pub fn decode(b: [u8; 5]) -> Self {
        match b {
            [0x51, usage, 0, 0, 0] => Self::Key { usage },
            [0x62, code, param, 0, 0] => Self::Function { code, param },
            [0, 0, 0, 0, 0] => Self::None,
            bytes => Self::Raw { bytes },
        }
    }

    pub fn encode(self) -> [u8; 5] {
        match self {
            Self::Key { usage } => [0x51, usage, 0, 0, 0],
            Self::Function { code, param } => [0x62, code, param, 0, 0],
            Self::None => [0; 5],
            Self::Raw { bytes } => bytes,
        }
    }

    pub fn factory(layer: u8, source: u8) -> Self {
        if layer == 0 {
            Self::Key { usage: source }
        } else {
            keys::factory_function(source).map(Self::decode).unwrap_or(Self::None)
        }
    }
}

/// Mappa completa di un livello: usage sorgente → funzione.
pub type Layer = BTreeMap<u8, KeyBinding>;

pub struct Apex3Tkl {
    dev: HidDevice,
}

impl Apex3Tkl {
    pub fn new(dev: HidDevice) -> Self {
        Self { dev }
    }

    fn output(&self, command: OutputCommand) -> Result<()> {
        let mut packet = [0u8; OUTPUT_LEN];
        match command {
            OutputCommand::Colors(colors) => {
                packet[1] = 0x21;
                packet[2] = 0xFF;
                for (i, color) in colors.iter().enumerate() {
                    packet[3 + 3 * i..6 + 3 * i].copy_from_slice(&color.to_bytes());
                }
            }
            OutputCommand::Rainbow => {
                packet[1] = 0x22;
                packet[2] = 0xFF;
            }
            OutputCommand::Brightness(level) => {
                packet[1] = 0x23;
                packet[2] = level.min(MAX_BRIGHTNESS);
            }
        }
        self.dev.write(&packet)?;
        Ok(())
    }

    fn send_feature(&self, command: &FeatureCommand, delay: Duration) -> Result<[u8; FEATURE_LEN]> {
        let request = command.encode();
        let mut packet = [0u8; FEATURE_LEN];
        packet[1..1 + request.len()].copy_from_slice(&request);
        self.dev.send_feature_report(&packet)?;
        sleep(delay);
        let mut reply = [0u8; FEATURE_LEN];
        let n = self.dev.get_feature_report(&mut reply)?;
        if n < 3 || reply[1] != request[0] {
            return Err(Error::Protocol(format!(
                "reply {:#04x} to command {:#04x} (is SteelSeries GG running?)",
                reply[1], request[0]
            )));
        }
        Ok(reply)
    }

    fn read(&self, command: &FeatureCommand) -> Result<[u8; FEATURE_LEN]> {
        // Le risposte a volte arrivano in ritardo: riproviamo come fa il software ufficiale.
        let mut last = Error::Timeout;
        for _ in 0..3 {
            match self.send_feature(command, READ_DELAY) {
                Ok(reply) => return Ok(reply),
                Err(error) => last = error,
            }
        }
        Err(last)
    }

    fn write(&self, command: &FeatureCommand) -> Result<()> {
        let reply = self.send_feature(command, WRITE_DELAY)?;
        if reply[2] != 0 {
            return Err(Error::Protocol(format!("command rejected (status {:#04x})", reply[2])));
        }
        Ok(())
    }

    fn read_string(&self, command: &FeatureCommand) -> Result<String> {
        let reply = self.read(command)?;
        let text = &reply[2..];
        let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
        Ok(String::from_utf8_lossy(&text[..end]).to_string())
    }

    pub fn firmware_version(&self) -> Result<String> {
        let version = self.read_string(&FeatureCommand::Version)?;
        let build = self.read_string(&FeatureCommand::Build)?;
        Ok(format!("{version}{build}"))
    }

    // ---- Illuminazione (volatile: si perde scollegando la tastiera) ----

    pub fn set_colors(&self, colors: &[Rgb; ZONES]) -> Result<()> {
        self.output(OutputCommand::Colors(colors))
    }

    pub fn set_hardware_rainbow(&self) -> Result<()> {
        self.output(OutputCommand::Rainbow)
    }

    pub fn set_brightness(&self, level: u8) -> Result<()> {
        self.output(OutputCommand::Brightness(level))
    }

    pub fn brightness(&self) -> Result<u8> {
        Ok(self.read(&FeatureCommand::Brightness)?[2].min(MAX_BRIGHTNESS))
    }

    // ---- Mappa dei tasti (salvata nella tastiera) ----

    pub fn read_layer(&self, layer: u8) -> Result<Layer> {
        check_layer(layer)?;
        let slots: Vec<u8> = (0..=255u8).filter(|&u| keys::is_mappable_slot(u)).collect();
        let mut map = Layer::new();
        for chunk in slots.chunks(BINDINGS_PER_QUERY) {
            let mut attempt = 0;
            loop {
                let reply = self.read(&FeatureCommand::Bindings { layer, keys: chunk })?;
                // reply[2] è un byte non inizializzato dal firmware; le voci partono da reply[3].
                let entries = &reply[3..];
                let ok = chunk.iter().enumerate().all(|(i, &key)| entries[6 * i] == key);
                if ok {
                    for (i, &key) in chunk.iter().enumerate() {
                        let raw: [u8; 5] = entries[6 * i + 1..6 * i + 6].try_into().unwrap();
                        map.insert(key, KeyBinding::decode(raw));
                    }
                    break;
                }
                attempt += 1;
                if attempt >= 4 {
                    return Err(Error::Protocol("inconsistent keymap response".into()));
                }
                sleep(Duration::from_millis(150));
            }
        }
        Ok(map)
    }

    /// Imposta un tasto solo in RAM. Va confermato con [`Apex3Tkl::save`].
    fn set_binding_volatile(&self, layer: u8, source: u8, binding: KeyBinding) -> Result<()> {
        check_layer(layer)?;
        validate_binding(layer, source, binding)?;
        self.write(&FeatureCommand::SetBinding { layer, source, binding: binding.encode() })
    }

    fn save(&self) -> Result<()> {
        self.write(&FeatureCommand::Save)
    }

    /// Applica le modifiche in modo transazionale: scrive in RAM, rilegge e verifica, e solo
    /// allora salva nella memoria della tastiera. Se qualcosa non torna, ripristina i valori precedenti.
    /// Restituisce la mappa letta dopo il salvataggio.
    pub fn apply_changes(&self, changes: &[(u8, u8, KeyBinding)]) -> Result<[Layer; 2]> {
        for &(layer, source, binding) in changes {
            check_layer(layer)?;
            validate_binding(layer, source, binding)?;
        }
        let before = [self.read_layer(0)?, self.read_layer(1)?];
        let mut expected = before.clone();
        let mut written = Vec::new();
        let result = (|| {
            for &(layer, source, binding) in changes {
                let current = before[layer as usize][&source];
                if current == binding {
                    continue;
                }
                // Un valore attuale sconosciuto non saprebbe essere ripristinato: meglio fermarsi.
                validate_binding(layer, source, current)?;
                written.push((layer, source, current));
                self.set_binding_volatile(layer, source, binding)?;
                expected[layer as usize].insert(source, binding);
            }
            let after = [self.read_layer(0)?, self.read_layer(1)?];
            if after != expected {
                return Err(Error::Protocol("the keyboard did not apply the changes as expected".into()));
            }
            Ok(())
        })();
        if let Err(error) = result {
            for &(layer, source, binding) in written.iter().rev() {
                let _ = self.set_binding_volatile(layer, source, binding);
            }
            return Err(error);
        }
        if written.is_empty() {
            return Ok(before);
        }
        self.save()?;
        let saved = [self.read_layer(0)?, self.read_layer(1)?];
        if saved != expected {
            return Err(Error::Protocol("the keymap read back after saving is different".into()));
        }
        Ok(saved)
    }
}

fn check_layer(layer: u8) -> Result<()> {
    if LAYERS.contains(&layer) { Ok(()) } else { Err(Error::Invalid(format!("layer {layer}"))) }
}

/// Valori ammessi in scrittura: un tasto singolo valido, oppure il valore di fabbrica.
pub fn validate_binding(layer: u8, source: u8, binding: KeyBinding) -> Result<()> {
    if !keys::is_remappable_source(source) {
        return Err(Error::Invalid(format!("key {source:#04x} cannot be remapped")));
    }
    if binding == KeyBinding::factory(layer, source) {
        return Ok(());
    }
    match binding {
        KeyBinding::Key { usage } if keys::is_valid_target(usage) => Ok(()),
        _ => Err(Error::Invalid(format!("this function is not allowed for key {source:#04x}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_table_has_114_entries() {
        assert_eq!((0..=255u8).filter(|&u| keys::is_mappable_slot(u)).count(), 114);
        assert!(!keys::is_remappable_source(keys::STEELSERIES_KEY));
    }

    #[test]
    fn bindings_round_trip() {
        for raw in [[0x51, 0x04, 0, 0, 0], [0x62, 0x03, 0x01, 0, 0], [0; 5], [0x7F, 1, 2, 3, 4]] {
            assert_eq!(KeyBinding::decode(raw).encode(), raw);
        }
    }

    #[test]
    fn only_safe_bindings_are_writable() {
        assert!(validate_binding(0, 0x39, KeyBinding::Key { usage: 0x29 }).is_ok());
        assert!(validate_binding(1, 0x44, KeyBinding::factory(1, 0x44)).is_ok());
        assert!(validate_binding(1, 0x04, KeyBinding::None).is_ok());
        assert!(validate_binding(0, 0x04, KeyBinding::None).is_err());
        assert!(validate_binding(0, 0x04, KeyBinding::Function { code: 1, param: 0 }).is_err());
        assert!(validate_binding(0, keys::STEELSERIES_KEY, KeyBinding::Key { usage: 0x04 }).is_err());
        assert!(validate_binding(0, 0x65, KeyBinding::Key { usage: 0x04 }).is_err());
        assert!(validate_binding(0, 0x04, KeyBinding::Key { usage: 0x66 }).is_err());
    }

    #[test]
    fn no_bootloader_command_can_be_built() {
        let commands = [
            FeatureCommand::Version,
            FeatureCommand::Build,
            FeatureCommand::Brightness,
            FeatureCommand::Bindings { layer: 0, keys: &[0x04] },
            FeatureCommand::SetBinding { layer: 0, source: 0x04, binding: [0x51, 0x04, 0, 0, 0] },
            FeatureCommand::Save,
        ];
        for command in commands {
            assert!([0x90, 0xA3, 0xAA, 0x2A, 0x11].contains(&command.encode()[0]));
        }
    }
}
