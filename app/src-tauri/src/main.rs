// Su Windows niente finestra del terminale nelle build di rilascio.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Mutex;

use perihub_core::Manager;
use perihub_core::config::AppConfig;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager as _, WindowEvent};

pub struct AppState {
    pub manager: Manager,
    pub config: Mutex<AppConfig>,
}

/// Argomento passato dall'avvio automatico: parte nascosto nell'area di notifica.
const HIDDEN_ARG: &str = "--hidden";

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let hidden_requested = std::env::args().any(|a| a == HIDDEN_ARG);

    tauri::Builder::default()
        // Un secondo avvio riporta in primo piano la finestra già aperta.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .setup(move |app| {
            let config = AppConfig::load();
            let handle = app.handle().clone();
            let manager = Manager::start(
                config.keyboard.clone(),
                Box::new(move |status| {
                    let _ = handle.emit("devices-changed", status);
                }),
            );
            let start_hidden = hidden_requested || config.start_hidden;
            app.manage(AppState { manager, config: Mutex::new(config) });

            let open = MenuItem::with_id(app, "open", "Open PeriHub", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("PeriHub")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_main(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // Solo in sviluppo: PERIHUB_PAGE=mouse:buttons apre direttamente una pagina.
            #[cfg(debug_assertions)]
            if let (Ok(page), Some(window)) = (std::env::var("PERIHUB_PAGE"), app.get_webview_window("main")) {
                let mut url = window.url()?;
                url.set_fragment(Some(&page));
                window.navigate(url)?;
            }

            if !start_hidden {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Chiudere la finestra la nasconde: il programma resta attivo per mantenere luci ed effetti.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::kb_get_config,
            commands::kb_set_lighting,
            commands::kb_set_brightness,
            commands::kb_read_brightness,
            commands::kb_get_keymap,
            commands::kb_apply_keymap,
            commands::kb_reset_keymap,
            commands::kb_key_catalog,
            commands::mouse_get_state,
            commands::mouse_save_profile,
            commands::mouse_restore_factory,
            commands::mouse_set_profile_enabled,
            commands::mouse_set_active_profile,
            commands::mouse_preview_dpi,
            commands::mouse_preview_led,
            commands::mouse_revert_preview,
            commands::app_get_settings,
            commands::app_set_settings,
            commands::app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start PeriHub");
}
