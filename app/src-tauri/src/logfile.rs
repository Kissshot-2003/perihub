//! Log file, so that a bug report can include what happened. It keeps PeriHub's own messages
//! (devices connecting, errors, updates), never key presses or the apps in focus, and stays small:
//! past 1 MB it becomes `perihub.old.log` and a new file starts.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

const MAX_SIZE: u64 = 1024 * 1024;

/// Linux: ~/.local/state/perihub.
#[cfg(not(windows))]
pub fn dir() -> PathBuf {
    dirs::state_dir().or_else(dirs::data_local_dir).unwrap_or_else(std::env::temp_dir).join("perihub")
}

/// Windows: %LOCALAPPDATA%\io.github.hexdump.perihub\logs, next to the web view's data (the folder
/// is named after the identifier in tauri.conf.json), which the uninstaller removes if asked to.
/// 0.4.1 and earlier wrote to %LOCALAPPDATA%\perihub, which is the install folder.
#[cfg(windows)]
pub fn dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("io.github.hexdump.perihub").join("logs")
}

/// Moves the log of 0.4.1 and earlier out of the install folder, which it kept from being removed.
#[cfg(windows)]
fn move_old_log(dir: &std::path::Path) {
    let Some(old) = dirs::data_local_dir().map(|d| d.join("perihub")) else { return };
    for name in ["perihub.log", "perihub.old.log"] {
        if old.join(name).exists() && !dir.join(name).exists() {
            let _ = fs::rename(old.join(name), dir.join(name));
        }
    }
}

/// Writes every message to the terminal (if there is one) and to the log file.
pub struct LogWriter {
    file: Option<File>,
    size: u64,
}

impl LogWriter {
    pub fn open() -> Self {
        let mut writer = Self { file: None, size: 0 };
        #[cfg(windows)]
        {
            let dir = dir();
            if fs::create_dir_all(&dir).is_ok() {
                move_old_log(&dir);
            }
        }
        writer.reopen();
        writer
    }

    fn reopen(&mut self) {
        let dir = dir();
        let path = dir.join("perihub.log");
        let file = fs::create_dir_all(&dir).and_then(|_| {
            if fs::metadata(&path).is_ok_and(|m| m.len() > MAX_SIZE) {
                fs::rename(&path, dir.join("perihub.old.log"))?;
            }
            OpenOptions::new().create(true).append(true).open(&path)
        });
        self.size = file.as_ref().ok().and_then(|f| f.metadata().ok()).map_or(0, |m| m.len());
        self.file = file.ok();
    }
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Without a console (Windows release builds) stderr fails: the file still gets the message.
        let _ = io::stderr().write_all(buf);
        if let Some(file) = &mut self.file
            && file.write_all(buf).is_ok()
        {
            self.size += buf.len() as u64;
        }
        if self.size > MAX_SIZE {
            self.reopen();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.as_mut().map_or(Ok(()), File::flush)
    }
}
