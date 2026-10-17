//! PeriHub: gestione di SteelSeries Apex 3 TKL e Logitech G502 HERO senza GG né G HUB.

pub mod apex;
pub mod color;
pub mod config;
pub mod effects;
pub mod error;
pub mod hidpp;
pub mod keyboard;
pub mod manager;
pub mod mouse;
pub mod worker;

pub use color::Rgb;
pub use error::{Error, Result};
pub use manager::{DeviceStatus, Manager};
