//! Updates on Windows. PeriHub asks GitHub for the latest release and, when the user agrees,
//! downloads its installer, checks its signature and runs it: the installer closes PeriHub and
//! starts the new version. On Linux updates come from the packages, so nothing is checked.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// This build can update itself.
pub const SUPPORTED: bool = cfg!(windows);

#[derive(Serialize)]
pub struct UpdateInfo {
    version: String,
}

async fn find(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    if !SUPPORTED {
        return Err("on Linux, updates come from your packages".into());
    }
    let updater = app.updater().map_err(|e| e.to_string())?;
    updater.check().await.map_err(|e| format!("cannot check for updates: {e}"))
}

/// The newer version on GitHub, if there is one.
#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    Ok(find(&app).await?.map(|update| UpdateInfo { version: update.version }))
}

#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    let update = find(&app).await?.ok_or("PeriHub is already up to date")?;
    log::info!("installing PeriHub {}", update.version);
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| format!("update failed: {e}"))?;
    // On Windows the installer has already closed PeriHub; elsewhere start the new version.
    app.restart()
}
