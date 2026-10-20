//! The main window and the tray icon. The window exists only while it is open: closing it
//! destroys the web view, which is most of PeriHub's memory and CPU (measured on Windows with
//! 0.3.0: perihub.exe alone 4 MB and 0.03% of a core, its WebView2 processes about 45 MB and 2%).
//! PeriHub keeps running in the tray to animate the lighting and follow the devices.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder, Wry};

const MAIN: &str = "main";
const TRAY: &str = "main";

/// Shows the window, creating it if needed, optionally on a page ("settings", "mouse:buttons"…).
pub fn show(app: &AppHandle, page: Option<&str>) {
    let window = match app.get_webview_window(MAIN) {
        Some(window) => window,
        None => match create(app) {
            Ok(window) => window,
            Err(error) => {
                log::warn!("cannot open the window: {error}");
                return;
            }
        },
    };
    if let Some(page) = page {
        open_page(&window, page);
    }
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn create(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let config = app.config().app.windows.iter().find(|w| w.label == MAIN).expect("main window in tauri.conf.json");
    let window = WebviewWindowBuilder::from_config(app, config)?.build()?;
    // Development only: PERIHUB_PAGE=mouse:buttons opens a page directly.
    #[cfg(debug_assertions)]
    if let Ok(page) = std::env::var("PERIHUB_PAGE") {
        open_page(&window, &page);
    }
    Ok(window)
}

fn open_page(window: &WebviewWindow, page: &str) {
    if let Ok(mut url) = window.url() {
        url.set_fragment(Some(page));
        let _ = window.navigate(url);
    }
}

fn menu(app: &AppHandle, update: Option<&str>) -> tauri::Result<Menu<Wry>> {
    let open = MenuItem::with_id(app, "open", "Open PeriHub", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    match update {
        Some(version) => {
            let update = MenuItem::with_id(app, "update", format!("Update to {version}…"), true, None::<&str>)?;
            Menu::with_items(app, &[&open, &update, &quit])
        }
        None => Menu::with_items(app, &[&open, &quit]),
    }
}

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id(TRAY)
        .tooltip("PeriHub")
        .menu(&menu(app, None)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app, None),
            "update" => show(app, Some("settings")),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show(tray.app_handle(), None);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Tells about a new version from the tray, where PeriHub spends most of its time: in the
/// tooltip and with a menu entry that opens the settings.
pub fn show_update(app: &AppHandle, version: &str) {
    if let Some(tray) = app.tray_by_id(TRAY) {
        let _ = tray.set_tooltip(Some(format!("PeriHub: version {version} is available")));
        if let Ok(menu) = menu(app, Some(version)) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}
