//! Log file, so that a bug report can include what happened. It keeps PeriHub's own messages
//! (devices connecting, errors, updates), never key presses or the apps in focus, and stays small:
//! past 1 MB it becomes `perihub.old.log` and a new file starts.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

const MAX_SIZE: u64 = 1024 * 1024;

/// Linux: ~/.local/state/perihub. Windows: %LOCALAPPDATA%\perihub.
pub fn dir() -> PathBuf {
    dirs::state_dir().or_else(dirs::data_local_dir).unwrap_or_else(std::env::temp_dir).join("perihub")
}

/// Writes every message to the terminal (if there is one) and to the log file.
pub struct LogWriter {
    file: Option<File>,
    size: u64,
}

impl LogWriter {
    pub fn open() -> Self {
        let mut writer = Self { file: None, size: 0 };
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
