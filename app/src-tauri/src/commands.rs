//! Commands called by the interface. Device operations can take a few seconds,
//! so they run off the main thread to avoid freezing the window.

use perihub_core::apex::{KeyBinding, keys};
use perihub_core::config::KeyboardConfig;
use perihub_core::effects::KeyboardLighting;
use perihub_core::hidpp::profile::{LedEffect, ProfileSettings};
use perihub_core::keyboard::Keymap;
use perihub_core::mouse::MouseState;
use perihub_core::{DeviceStatus, Error};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::AppState;

type CmdResult<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> perihub_core::Result<T> + Send + 'static) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> DeviceStatus {
    state.manager.status()
}

// ---- Keyboard ----

#[tauri::command]
pub fn kb_get_config(state: State<'_, AppState>) -> KeyboardConfig {
    state.config.lock().unwrap().keyboard.clone()
}

/// Saves the keyboard settings (also while it is unplugged: they are applied when it is connected).
fn update_keyboard_config(state: &AppState, change: impl FnOnce(&mut KeyboardConfig)) -> CmdResult<()> {
    let mut config = state.config.lock().unwrap();
    change(&mut config.keyboard);
    state.manager.set_keyboard_config(config.keyboard.clone());
    config.save().map_err(|e| e.to_string())
}

fn ignore_disconnected(result: perihub_core::Result<()>) -> perihub_core::Result<()> {
    match result {
        Err(Error::NotConnected) => Ok(()),
        other => other,
    }
}

#[tauri::command]
pub async fn kb_set_lighting(state: State<'_, AppState>, lighting: KeyboardLighting) -> CmdResult<()> {
    update_keyboard_config(&state, |c| c.lighting = lighting.clone())?;
    let manager = state.manager.clone();
    blocking(move || ignore_disconnected(manager.keyboard().and_then(|k| k.call(move |k| k.set_lighting(lighting)))))
        .await
}

#[tauri::command]
pub async fn kb_set_brightness(state: State<'_, AppState>, level: u8) -> CmdResult<()> {
    update_keyboard_config(&state, |c| c.brightness = level)?;
    let manager = state.manager.clone();
    blocking(move || ignore_disconnected(manager.keyboard().and_then(|k| k.call(move |k| k.set_brightness(level)))))
        .await
}

#[tauri::command]
pub async fn kb_read_brightness(state: State<'_, AppState>) -> CmdResult<u8> {
    let manager = state.manager.clone();
    blocking(move || manager.keyboard()?.call(|k| k.brightness())).await
}

#[tauri::command]
pub async fn kb_get_keymap(state: State<'_, AppState>) -> CmdResult<Keymap> {
    let manager = state.manager.clone();
    blocking(move || manager.keyboard()?.call(|k| k.keymap())).await
}

#[derive(Debug, Deserialize)]
pub struct KeyChange {
    layer: u8,
    source: u8,
    binding: KeyBinding,
}

#[tauri::command]
pub async fn kb_apply_keymap(state: State<'_, AppState>, changes: Vec<KeyChange>) -> CmdResult<Keymap> {
    let changes: Vec<_> = changes.into_iter().map(|c| (c.layer, c.source, c.binding)).collect();
    let manager = state.manager.clone();
    blocking(move || manager.keyboard()?.call(move |k| k.apply_keymap(&changes))).await
}

#[tauri::command]
pub async fn kb_reset_keymap(state: State<'_, AppState>) -> CmdResult<Keymap> {
    let manager = state.manager.clone();
    blocking(move || manager.keyboard()?.call(|k| k.reset_keymap())).await
}

#[derive(Serialize)]
pub struct KeyCatalog {
    targets: &'static [keys::KeyInfo],
    sources: Vec<u8>,
}

#[tauri::command]
pub fn kb_key_catalog() -> KeyCatalog {
    KeyCatalog { targets: keys::TARGET_KEYS, sources: (0..=255u8).filter(|&u| keys::is_remappable_source(u)).collect() }
}

// ---- Mouse ----

#[tauri::command]
pub async fn mouse_get_state(state: State<'_, AppState>) -> CmdResult<MouseState> {
    let manager = state.manager.clone();
    blocking(move || manager.mouse()?.call(|m| m.state())).await
}

#[tauri::command]
pub async fn mouse_save_profile(
    state: State<'_, AppState>,
    number: u8,
    settings: ProfileSettings,
) -> CmdResult<MouseState> {
    let manager = state.manager.clone();
    blocking(move || manager.mouse()?.call(move |m| m.save_profile(number, &settings))).await
}

#[tauri::command]
pub async fn mouse_restore_factory(state: State<'_, AppState>, number: u8) -> CmdResult<MouseState> {
    let manager = state.manager.clone();
    blocking(move || manager.mouse()?.call(move |m| m.restore_factory_profile(number))).await
}

#[tauri::command]
pub async fn mouse_set_profile_enabled(state: State<'_, AppState>, number: u8, enabled: bool) -> CmdResult<MouseState> {
    let manager = state.manager.clone();
    blocking(move || manager.mouse()?.call(move |m| m.set_profile_enabled(number, enabled))).await
}

#[tauri::command]
pub async fn mouse_set_active_profile(state: State<'_, AppState>, number: u8) -> CmdResult<MouseState> {
    let manager = state.manager.clone();
    blocking(move || {
        manager.mouse()?.call(move |m| {
            m.set_active_profile(number)?;
            m.state()
        })
    })
    .await
}

#[tauri::command]
pub async fn mouse_preview_dpi(state: State<'_, AppState>, dpi: u16) -> CmdResult<()> {
    let manager = state.manager.clone();
    blocking(move || manager.mouse()?.call(move |m| m.preview_dpi(dpi))).await
}

#[tauri::command]
pub async fn mouse_preview_led(state: State<'_, AppState>, zone: u8, effect: LedEffect) -> CmdResult<()> {
    let manager = state.manager.clone();
    blocking(move || manager.mouse()?.call(move |m| m.preview_led(zone, &effect))).await
}

#[tauri::command]
pub async fn mouse_revert_preview(state: State<'_, AppState>) -> CmdResult<()> {
    let manager = state.manager.clone();
    blocking(move || ignore_disconnected(manager.mouse().and_then(|m| m.call(|m| m.revert_preview())))).await
}

// ---- App ----

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    start_hidden: bool,
    autostart: bool,
}

#[tauri::command]
pub fn app_get_settings(app: AppHandle, state: State<'_, AppState>) -> AppSettings {
    AppSettings {
        start_hidden: state.config.lock().unwrap().start_hidden,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

#[tauri::command]
pub fn app_set_settings(app: AppHandle, state: State<'_, AppState>, settings: AppSettings) -> CmdResult<AppSettings> {
    let launcher = app.autolaunch();
    let result = if settings.autostart { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| format!("autostart: {e}"))?;
    let mut config = state.config.lock().unwrap();
    config.start_hidden = settings.start_hidden;
    config.save().map_err(|e| e.to_string())?;
    drop(config);
    Ok(app_get_settings(app, state))
}

#[tauri::command]
pub fn app_quit(app: AppHandle) {
    app.exit(0);
}
