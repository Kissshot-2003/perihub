//! Autostart fix for Windows. tauri-plugin-autostart (through auto-launch 0.6) writes the Run
//! entry as `C:\…\perihub.exe --hidden`, without quotes: with a space in the path, for example
//! `C:\Users\Mario Rossi\…`, Windows may split the command at the first space. The entry is
//! rewritten with quotes in the key where auto-launch put it.

use tauri::AppHandle;

#[cfg(windows)]
pub fn quote_entry(app: &AppHandle) {
    use windows_registry::{CURRENT_USER, LOCAL_MACHINE};

    const RUN: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    let name = &app.package_info().name;
    let Ok(exe) = std::env::current_exe() else { return };
    let value = format!("\"{}\" {}", exe.display(), crate::HIDDEN_ARG);
    for root in [CURRENT_USER, LOCAL_MACHINE] {
        let Ok(key) = root.options().read().write().open(RUN) else { continue };
        // auto-launch only checks that the entry exists, so it still counts as enabled.
        if key.get_string(name).is_ok_and(|current| current != value)
            && let Err(error) = key.set_string(name, &value)
        {
            log::warn!("autostart entry not quoted: {error}");
        }
    }
}

#[cfg(not(windows))]
pub fn quote_entry(_app: &AppHandle) {}
