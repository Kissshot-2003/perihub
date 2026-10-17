use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("errore HID: {0}")]
    Hid(#[from] hidapi::HidError),
    #[error("il mouse ha rifiutato il comando: {0}")]
    Hidpp(HidppErrorCode),
    #[error("il dispositivo non ha risposto in tempo")]
    Timeout,
    #[error("funzione {0:#06x} non disponibile su questo dispositivo")]
    FeatureMissing(u16),
    #[error("risposta inattesa dal dispositivo: {0}")]
    Protocol(String),
    #[error("valore non valido: {0}")]
    Invalid(String),
    #[error("dispositivo non collegato")]
    NotConnected,
    #[error("errore di file: {0}")]
    Io(#[from] std::io::Error),
    #[error("errore di formato: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HidppErrorCode(pub u8);

impl std::fmt::Display for HidppErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.0 {
            0x01 => "errore sconosciuto",
            0x02 => "argomento non valido",
            0x03 => "valore fuori intervallo",
            0x04 => "errore hardware",
            0x05 => "errore interno Logitech",
            0x06 => "indice funzione non valido",
            0x07 => "funzione non valida",
            0x08 => "dispositivo occupato",
            0x09 => "non supportato",
            _ => "codice non documentato",
        };
        write!(f, "{name} ({:#04x})", self.0)
    }
}
