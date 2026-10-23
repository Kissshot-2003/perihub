//! Starting with Windows, which PeriHub does in the tray (`--hidden`).
//!
//! On Windows PeriHub keeps the autostart entry in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! and manages it itself. auto-launch 0.6 (through tauri-plugin-autostart) prefers `HKLM` in its
//! `Dynamic` mode and succeeds there whenever it can write, so 0.4.2 left machine-wide entries:
//! they would also start PeriHub for other accounts, pointing at this user's folder, which does
//! not exist for them. Entries that those versions left in `HKLM` are moved to `HKCU` at startup.
//!
//! The Run value is written with quotes around the exe path: without them, with a space in the
//! path like `C:\Users\Mario Rossi\…`, Windows may split the command at the first space. The
//! value in `Explorer\StartupApproved\Run` that Task Manager needs to show the entry as enabled
//! is written too, like auto-launch does. Linux keeps tauri-plugin-autostart.

use tauri::AppHandle;

#[cfg(windows)]
const E_ACCESSDENIED: windows_result::HRESULT = windows_result::HRESULT::from_win32(0x80070005_u32);
#[cfg(windows)]
const E_FILENOTFOUND: windows_result::HRESULT = windows_result::HRESULT::from_win32(0x80070002_u32);

#[cfg(windows)]
fn run_path() -> &'static str {
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
}

#[cfg(windows)]
fn task_manager_path() -> &'static str {
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
}

/// The command line the entry runs, for example `"C:\PeriHub\perihub.exe" --hidden`.
#[cfg(windows)]
fn command_line() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\" {}", exe.display(), crate::HIDDEN_ARG))
}

/// Task Manager can suppress a Run entry. Without a value there, Task Manager shows the entry
/// as enabled, and this value does the same: the flag 2, then 8 zero bytes.
#[cfg(windows)]
const TASK_MANAGER_ENABLED: [u8; 12] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

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

/// Whether PeriHub starts on its own at sign-in.
#[cfg(windows)]
pub fn is_enabled(app: &AppHandle) -> bool {
    use windows_registry::CURRENT_USER;

    let name = &app.package_info().name;
    let Ok(run) = CURRENT_USER.open(run_path()) else {
        return false;
    };
    if run.get_string(name).is_err() {
        return false;
    }
    match CURRENT_USER.open(task_manager_path()).and_then(|key| key.get_value(name)) {
        Ok(value) => {
            // The last 8 bytes hold the time the entry was enabled: when they are not all zero,
            // Task Manager does not start the entry.
            value.len() < 8 || value.iter().rev().take(8).all(|&byte| byte == 0)
        }
        // Without a value there, the entry counts as enabled.
        Err(_) => true,
    }
}

#[cfg(not(windows))]
pub fn is_enabled(app: &AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;

    app.autolaunch().is_enabled().unwrap_or(false)
}

/// Starts or stops PeriHub starting on its own at sign-in. When `enabled` already holds, both
/// sides still write the entries again with the current exe path and arguments.
#[cfg(windows)]
pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if enabled { enable(app) } else { disable(app) }
}

#[cfg(not(windows))]
pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| e.to_string())
}

#[cfg(windows)]
pub fn startup(app: &AppHandle) {
    move_from_local_machine(app);
    // Entries written by 0.3.0 and earlier are not quoted.
    if is_enabled(app) {
        quote_entry(app);
    }
}

#[cfg(not(windows))]
pub fn startup(_app: &AppHandle) {}

/// Moves the Run entry that 0.4.2 and earlier wrote under `HKLM` back to `HKCU`, where a
/// per-user app belongs. Only an entry pointing at PeriHub's own exe is moved. It stays in
/// `HKLM` when Windows does not let us delete it, which PeriHub logs.
#[cfg(windows)]
fn move_from_local_machine(app: &AppHandle) {
    use windows_registry::{CURRENT_USER, LOCAL_MACHINE};

    let name = &app.package_info().name;
    let Some(value) = command_line() else { return };
    let Ok(run) = LOCAL_MACHINE.options().read().open(run_path()) else { return };
    let Ok(entry) = run.get_string(name) else { return };
    // Not a path of our own exe, for example written for another install location: leave it.
    let Some(exe) = std::env::current_exe().ok() else { return };
    let exe = exe.display().to_string();
    let entry = entry.replace('"', "");
    let ours = entry.get(..exe.len()).is_some_and(|start| start.eq_ignore_ascii_case(&exe));
    if !ours {
        return;
    }

    // The entry stays enabled for this user even when Windows does not let us delete the
    // machine-wide one.
    if let Err(error) = CURRENT_USER.create(run_path()).and_then(|key| key.set_string(name, &value)) {
        log::warn!("could not write the HKCU autostart entry that replaces the HKLM one: {error}");
        return;
    }
    match LOCAL_MACHINE.options().read().write().open(run_path()).and_then(|key| key.remove_value(name)) {
        Ok(_) => {
            log::info!("moved the autostart entry from HKLM to HKCU");
            // The Task Manager value auto-launch wrote for the HKLM entry would stay behind.
            if let Ok(key) = LOCAL_MACHINE.options().read().write().open(task_manager_path())
                && let Err(error) = key.remove_value(name)
                && error.code() != E_FILENOTFOUND
            {
                log::warn!("could not remove the HKLM Task Manager value of the moved entry: {error}");
            }
        }
        Err(error) if error.code() == E_ACCESSDENIED => {
            log::warn!("autostart entry left in HKLM: not allowed to delete it");
        }
        Err(error) => log::warn!("could not delete the HKLM autostart entry: {error}"),
    }
}

/// Rewrites the entry with the current quoted command line, both to quote the unquoted entries
/// 0.3.0 and earlier wrote and to follow the exe after an update moved it.
#[cfg(windows)]
fn quote_entry(app: &AppHandle) {
    use windows_registry::CURRENT_USER;

    let name = &app.package_info().name;
    let Some(value) = command_line() else { return };
    let Ok(key) = CURRENT_USER.options().read().write().open(run_path()) else { return };
    // auto-launch only checks that the entry exists, so a wrong value still counts as enabled.
    if key.get_string(name).is_ok_and(|current| current != value)
        && let Err(error) = key.set_string(name, &value)
    {
        log::warn!("autostart entry not quoted: {error}");
    }
}

#[cfg(not(windows))]
fn quote_entry(_app: &AppHandle) {}

#[cfg(windows)]
fn enable(app: &AppHandle) -> Result<(), String> {
    use windows_registry::{CURRENT_USER, Type};

    let name = &app.package_info().name;
    let Some(value) = command_line() else { return Err("could not find PeriHub's own path".to_string()) };
    // create opens the key when it already exists.
    CURRENT_USER.create(run_path()).and_then(|key| key.set_string(name, &value)).map_err(|error| format!("{error}"))?;
    match CURRENT_USER.options().write().open(task_manager_path()) {
        Ok(key) => {
            key.set_bytes(name, Type::Bytes, &TASK_MANAGER_ENABLED).map_err(|error| format!("{error}"))?;
        }
        Err(error) if error.code() == E_FILENOTFOUND => {}
        Err(error) => return Err(format!("{error}")),
    }
    Ok(())
}

#[cfg(windows)]
fn disable(app: &AppHandle) -> Result<(), String> {
    use windows_registry::CURRENT_USER;

    let name = &app.package_info().name;
    // Like auto-launch, only the Run value is removed: the Task Manager value only matters
    // while the Run entry exists.
    match CURRENT_USER.options().write().open(run_path()).and_then(|key| key.remove_value(name)) {
        Ok(_) => Ok(()),
        Err(error) if error.code() == E_FILENOTFOUND => Ok(()),
        Err(error) => Err(format!("{error}")),
    }
}
