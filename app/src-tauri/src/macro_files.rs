//! Macros saved to and loaded from files, to keep them or to share them. Only this module opens the
//! system's file dialogs and touches the chosen file: the interface gets no access to files.

use perihub_core::hidpp::macros::Macro;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, FilePath};

const FORMAT: &str = "perihub-macro";
const VERSION: u32 = 1;
/// A macro file is a few kilobytes at most: anything bigger is not one.
const MAX_FILE_SIZE: u64 = 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct MacroFile {
    format: String,
    version: u32,
    #[serde(flatten)]
    content: Macro,
}

async fn ask(app: AppHandle, save_as: Option<String>) -> Result<Option<std::path::PathBuf>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let dialog = app.dialog().file().add_filter("PeriHub macro", &["json"]);
        match save_as {
            Some(name) => dialog.set_title("Export macro").set_file_name(name).blocking_save_file(),
            None => dialog.set_title("Import macro").blocking_pick_file(),
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    picked.map(FilePath::into_path).transpose().map_err(|e| e.to_string())
}

/// Saves a macro to a file the user chooses. Returns `false` if the dialog was cancelled.
#[tauri::command]
pub async fn macro_export(app: AppHandle, content: Macro, name: String) -> Result<bool, String> {
    let name: String = name.chars().map(|c| if c.is_alphanumeric() || " -_".contains(c) { c } else { '_' }).collect();
    let Some(path) = ask(app, Some(format!("{}.json", name.trim()))).await? else { return Ok(false) };
    let file = MacroFile { format: FORMAT.into(), version: VERSION, content };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(true)
}

/// Reads a macro from a file the user chooses. `None` if the dialog was cancelled.
#[tauri::command]
pub async fn macro_import(app: AppHandle) -> Result<Option<Macro>, String> {
    let Some(path) = ask(app, None).await? else { return Ok(None) };
    let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if size > MAX_FILE_SIZE {
        return Err("this file is too big to be a PeriHub macro".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file: MacroFile = serde_json::from_str(&text).map_err(|e| format!("this is not a PeriHub macro: {e}"))?;
    if file.format != FORMAT || file.version != VERSION {
        return Err("this is not a PeriHub macro, or it comes from a newer version of PeriHub".into());
    }
    Ok(Some(file.content))
}

#[cfg(test)]
mod tests {
    use perihub_core::hidpp::macros::{MacroKind, MacroStep};

    use super::*;

    #[test]
    fn file_round_trip() {
        let content = Macro {
            kind: MacroKind::Sequence,
            press: vec![MacroStep::KeyDown { modifiers: 0x02, key: 0 }],
            release: vec![MacroStep::KeyUp { modifiers: 0x02, key: 0 }],
        };
        let file = MacroFile { format: FORMAT.into(), version: VERSION, content: content.clone() };
        let json = serde_json::to_value(&file).unwrap();
        assert_eq!(json["format"], "perihub-macro");
        assert_eq!(json["kind"], "sequence");
        assert_eq!(json["press"][0]["type"], "key_down");
        let back: MacroFile = serde_json::from_value(json).unwrap();
        assert_eq!(back.content, content);
    }
}
