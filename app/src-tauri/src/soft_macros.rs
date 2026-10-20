//! Commands for software macros (see `perihub_core::softmacro`), and on Linux the optional rule
//! that lets PeriHub simulate a keyboard and mouse through /dev/uinput.

use perihub_core::softmacro::{SoftMacroStatus, SoftwareMacro, TRIGGER_KEYS};
use serde::Serialize;
use tauri::State;

use crate::AppState;
use crate::key_access::as_root;

const RULE: &str = include_str!("../../../packaging/linux/72-perihub-uinput.rules");
const RULE_PATH: &str = "/etc/udev/rules.d/72-perihub-uinput.rules";
const RELOAD: &str = "udevadm control --reload && udevadm trigger --subsystem-match=misc --sysname-match=uinput";

#[derive(Serialize)]
pub struct SoftMacrosInfo {
    macros: Vec<SoftwareMacro>,
    status: SoftMacroStatus,
    /// Trigger keys not used by a macro yet, in the order they are given out.
    free_keys: Vec<u8>,
    /// Linux: PeriHub's optional rule is installed.
    rule_installed: bool,
    /// The rule can be installed from PeriHub (Linux only).
    rule_supported: bool,
}

#[tauri::command]
pub fn soft_get(state: State<'_, AppState>) -> SoftMacrosInfo {
    let macros = state.soft.macros();
    let free_keys = TRIGGER_KEYS.iter().copied().filter(|k| !macros.iter().any(|m| m.key == *k)).collect();
    SoftMacrosInfo {
        status: state.soft.status(),
        macros,
        free_keys,
        rule_installed: std::path::Path::new(RULE_PATH).exists(),
        rule_supported: cfg!(target_os = "linux"),
    }
}

fn store(state: &AppState, change: impl FnOnce(&mut Vec<SoftwareMacro>) -> Result<(), String>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    change(&mut config.software_macros)?;
    config.save().map_err(|e| e.to_string())?;
    state.soft.set_macros(config.software_macros.clone());
    // Reading the mouse's trigger keys starts with the first macro and stops with the last one.
    state.soft.watch_mouse(state.manager.mouse_path());
    Ok(())
}

/// Saves a macro. A new one (key 0) gets the first free trigger key, which its buttons must send.
#[tauri::command]
pub fn soft_save(state: State<'_, AppState>, mut item: SoftwareMacro) -> Result<SoftwareMacro, String> {
    store(&state, |macros| {
        if item.key == 0 {
            item.key = TRIGGER_KEYS
                .iter()
                .copied()
                .find(|k| !macros.iter().any(|m| m.key == *k))
                .ok_or("all 8 software macros are in use: delete one first")?;
        }
        item.name = item.name.trim().to_string();
        item.validate().map_err(|e| e.to_string())?;
        match macros.iter_mut().find(|m| m.key == item.key) {
            Some(existing) => *existing = item.clone(),
            None => macros.push(item.clone()),
        }
        Ok(())
    })?;
    Ok(item)
}

/// Deletes a macro. Buttons that send its key keep sending it, but nothing runs any more.
#[tauri::command]
pub fn soft_delete(state: State<'_, AppState>, key: u8) -> Result<(), String> {
    store(&state, |macros| {
        macros.retain(|m| m.key != key);
        Ok(())
    })
}

#[tauri::command]
pub fn soft_stop_all(state: State<'_, AppState>) {
    state.soft.stop_all();
}

#[tauri::command]
pub async fn soft_grant_access() -> Result<(), String> {
    as_root(format!("cat > {RULE_PATH} && {RELOAD}"), RULE).await
}

#[tauri::command]
pub async fn soft_revoke_access() -> Result<(), String> {
    as_root(format!("rm -f {RULE_PATH} && {RELOAD}"), "").await
}
