//! Starting with Windows, which PeriHub does in the tray (`--hidden`).
//!
//! Autostart fix: tauri-plugin-autostart (through auto-launch 0.6) writes the Run
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

/// Windows reopens the apps that were open at sign-out ("Restart apps" in the sign-in options).
/// On Windows 11 with 0.4.1 PeriHub came back without arguments, so with its window, before
/// autostart started it hidden. Registering the restart with `--hidden` makes it come back in the
/// tray. It runs again when the window is created, in case the web view registers its own restart,
/// and logs a registration it replaces, to find out who made it.
#[cfg(windows)]
pub fn restart_hidden() {
    use windows_sys::Win32::System::Recovery::{
        GetApplicationRestartSettings, RESTART_NO_CRASH, RESTART_NO_HANG, RegisterApplicationRestart,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // Only restarts after a sign-out, a reboot or an update: not after a crash or a hang.
    const FLAGS: u32 = RESTART_NO_CRASH | RESTART_NO_HANG;
    // RESTART_MAX_CMD_LINE.
    let mut current = [0u16; 1024];
    let (mut len, mut flags) = (current.len() as u32, 0);
    // SAFETY: the buffer holds `len` characters; the process handle is the current process.
    let registered =
        unsafe { GetApplicationRestartSettings(GetCurrentProcess(), current.as_mut_ptr(), &mut len, &mut flags) } == 0;
    if registered {
        let used = &current[..(len as usize).min(current.len())];
        let command = String::from_utf16_lossy(&used[..used.iter().position(|&c| c == 0).unwrap_or(used.len())]);
        if command == crate::HIDDEN_ARG && flags == FLAGS {
            return;
        }
        log::info!("restart after sign-in was registered as {command:?} (flags {flags}): registering it hidden");
    }
    let command: Vec<u16> = crate::HIDDEN_ARG.encode_utf16().chain([0]).collect();
    // SAFETY: `command` is a NUL-terminated UTF-16 string that outlives the call.
    let result = unsafe { RegisterApplicationRestart(command.as_ptr(), FLAGS) };
    if result != 0 {
        log::warn!("restart after sign-in not registered: {result:#x}");
    }
}

#[cfg(not(windows))]
pub fn restart_hidden() {}
