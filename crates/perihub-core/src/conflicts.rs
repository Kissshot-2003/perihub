//! Other programs that talk to the same devices. Logitech G HUB, for example, puts the mouse
//! under software control when it starts and after every wake-up, so the mouse stops using its
//! onboard profiles. PeriHub does not fight such a program: it tells the user instead.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Conflict {
    LogitechGHub,
}

impl Conflict {
    pub fn name(self) -> &'static str {
        match self {
            Self::LogitechGHub => "Logitech G HUB",
        }
    }

    /// Executable names, lowercase. Only names seen on a real PC: lghub_agent and
    /// lghub_system_tray in the Windows test of 0.3.0, lghub is G HUB's window.
    #[cfg_attr(not(windows), allow(dead_code))]
    fn processes(self) -> &'static [&'static str] {
        match self {
            Self::LogitechGHub => &["lghub.exe", "lghub_agent.exe", "lghub_system_tray.exe"],
        }
    }
}

const ALL: &[Conflict] = &[Conflict::LogitechGHub];

/// Conflicting programs that are running now. Always empty on Linux, where they do not exist.
pub fn running() -> Vec<Conflict> {
    let names = process_names();
    ALL.iter().copied().filter(|c| c.processes().iter().any(|p| names.iter().any(|n| n == p))).collect()
}

/// Lowercase executable names of the running processes.
#[cfg(windows)]
fn process_names() -> Vec<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };

    let mut names = Vec::new();
    // SAFETY: plain Win32 calls; the entry carries its size and the snapshot is closed.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return names;
        }
        let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut ok = Process32FirstW(snapshot, &mut entry);
        while ok != 0 {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            names.push(String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase());
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    names
}

#[cfg(not(windows))]
fn process_names() -> Vec<String> {
    Vec::new()
}
