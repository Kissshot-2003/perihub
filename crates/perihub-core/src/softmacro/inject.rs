//! Simulated keyboard and mouse for software macros, and opening websites, files and programs.
//!
//! Linux: two virtual devices made through /dev/uinput, "PeriHub virtual keyboard" and "PeriHub
//! virtual mouse". Windows: `SendInput`, with keys sent as scan codes so that they behave like the
//! physical keys, whatever the layout.

use std::path::Path;

use crate::error::{Error, Result};

pub trait Injector: Send {
    /// Presses or releases modifiers (bits as in a HID report) and a key (HID usage, 0 for none).
    /// Modifiers go down before the key and come up after it.
    fn keys(&mut self, modifiers: u8, key: u8, down: bool) -> Result<()>;
    /// Mouse buttons, bit 0 = left, 1 = right, 2 = middle, 3 = back, 4 = forward.
    fn buttons(&mut self, buttons: u16, down: bool) -> Result<()>;
    /// Media key (HID Consumer usage).
    fn consumer(&mut self, usage: u16, down: bool) -> Result<()>;
    /// Wheel notches: positive scrolls up.
    fn wheel(&mut self, amount: i8) -> Result<()>;
    fn text(&mut self, text: &str) -> Result<()>;
}

/// Whether macros can simulate input here, with the reason if not.
pub fn available() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::fs::OpenOptions::new().read(true).write(true).open("/dev/uinput").map(|_| ()).map_err(|_| {
            Error::Invalid("PeriHub cannot use /dev/uinput, which software macros need to press keys".into())
        })
    }
    #[cfg(not(target_os = "linux"))]
    Ok(())
}

pub fn open() -> Result<Box<dyn Injector>> {
    #[cfg(target_os = "linux")]
    return Ok(Box::new(linux::Uinput::open()?));
    #[cfg(windows)]
    return Ok(Box::new(windows::SendInput));
    #[cfg(not(any(target_os = "linux", windows)))]
    Err(Error::Invalid("software macros are not available on this system".into()))
}

fn is_web_address(target: &str) -> bool {
    (target.starts_with("https://") || target.starts_with("http://")) && !target.contains(char::is_whitespace)
}

/// What a macro may open: a website (http or https) or an existing file or program given by its
/// full path. Anything else, such as a command line, is refused.
pub fn check_target(target: &str) -> Result<()> {
    let target = target.trim();
    if is_web_address(target) || (Path::new(target).is_absolute() && Path::new(target).exists()) {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "\"{target}\" is neither a website (http or https) nor the full path of an existing file or program"
        )))
    }
}

/// Opens a website or file with its default app, or starts a program. No shell is involved: the
/// target is passed as a single argument.
pub fn open_target(target: &str) -> Result<()> {
    check_target(target)?;
    let target = target.trim();
    #[cfg(windows)]
    return windows::shell_open(target);
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = Path::new(target);
        let program = !is_web_address(target) && path.is_file() && path.metadata()?.permissions().mode() & 0o111 != 0;
        let mut child = if program {
            std::process::Command::new(path).spawn()?
        } else {
            std::process::Command::new("xdg-open").arg(target).spawn()?
        };
        // Waited for in the background, so that it does not stay behind as a zombie process.
        std::thread::spawn(move || child.wait());
        Ok(())
    }
}

/// Linux input event code of a key, from its HID usage (the kernel's own table, hid-input.c).
#[cfg(any(target_os = "linux", test))]
#[rustfmt::skip]
fn evdev_key(usage: u8) -> Option<u16> {
    const LETTERS: [u16; 26] = [30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44];
    Some(match usage {
        0x04..=0x1D => LETTERS[usize::from(usage - 0x04)],
        0x1E..=0x26 => u16::from(usage - 0x1E) + 2, // 1–9
        0x27 => 11,
        0x28 => 28, 0x29 => 1, 0x2A => 14, 0x2B => 15, 0x2C => 57, 0x2D => 12, 0x2E => 13, 0x2F => 26,
        0x30 => 27, 0x31 => 43, 0x32 => 43, 0x33 => 39, 0x34 => 40, 0x35 => 41, 0x36 => 51, 0x37 => 52,
        0x38 => 53, 0x39 => 58,
        0x3A..=0x43 => u16::from(usage - 0x3A) + 59, // F1–F10
        0x44 => 87, 0x45 => 88,
        0x46 => 99, 0x47 => 70, 0x48 => 119, 0x49 => 110, 0x4A => 102, 0x4B => 104, 0x4C => 111, 0x4D => 107,
        0x4E => 109, 0x4F => 106, 0x50 => 105, 0x51 => 108, 0x52 => 103, 0x53 => 69, 0x54 => 98, 0x55 => 55,
        0x56 => 74, 0x57 => 78, 0x58 => 96, 0x59 => 79, 0x5A => 80, 0x5B => 81, 0x5C => 75, 0x5D => 76,
        0x5E => 77, 0x5F => 71, 0x60 => 72, 0x61 => 73, 0x62 => 82, 0x63 => 83, 0x64 => 86, 0x65 => 127,
        0x67 => 117,
        0x68..=0x73 => u16::from(usage - 0x68) + 183, // F13–F24
        0x85 => 121, 0x87 => 89, 0x88 => 93, 0x89 => 124, 0x8A => 92, 0x8B => 94, 0x90 => 122, 0x91 => 123,
        0xE0 => 29, 0xE1 => 42, 0xE2 => 56, 0xE3 => 125, 0xE4 => 97, 0xE5 => 54, 0xE6 => 100, 0xE7 => 126,
        _ => return None,
    })
}

/// Linux input event code of a media key, for the keys PeriHub offers.
#[cfg(any(target_os = "linux", test))]
fn evdev_consumer(usage: u16) -> Option<u16> {
    Some(match usage {
        0xCD => 164,  // play/pause
        0xB5 => 163,  // next track
        0xB6 => 165,  // previous track
        0xB7 => 166,  // stop
        0xE2 => 113,  // mute
        0xE9 => 115,  // volume up
        0xEA => 114,  // volume down
        0x192 => 140, // calculator
        0x194 => 144, // file manager
        0x18A => 155, // mail
        0x223 => 172, // browser home
        0x224 => 158, // browser back
        0x225 => 159, // browser forward
        0x227 => 173, // browser refresh
        _ => return None,
    })
}

#[cfg(target_os = "linux")]
mod linux {
    use std::time::Duration;

    use evdev::uinput::VirtualDevice;
    use evdev::{AttributeSet, EventType, InputEvent, KeyCode, RelativeAxisCode};

    use super::{Injector, evdev_consumer, evdev_key};
    use crate::error::{Error, Result};

    /// Codes of the modifier keys, by their bit in a HID report.
    const MODIFIERS: [u16; 8] = [29, 42, 56, 125, 97, 54, 100, 126];
    /// BTN_LEFT, BTN_RIGHT, BTN_MIDDLE, BTN_SIDE (back), BTN_EXTRA (forward).
    const BUTTONS: [u16; 5] = [0x110, 0x111, 0x112, 0x113, 0x114];
    const MEDIA: [u16; 14] =
        [0xCD, 0xB5, 0xB6, 0xB7, 0xE2, 0xE9, 0xEA, 0x192, 0x194, 0x18A, 0x223, 0x224, 0x225, 0x227];

    pub struct Uinput {
        keyboard: VirtualDevice,
        mouse: VirtualDevice,
    }

    impl Uinput {
        pub fn open() -> Result<Self> {
            let keys: AttributeSet<KeyCode> = (0..=255u8)
                .filter_map(evdev_key)
                .chain(MEDIA.iter().filter_map(|&u| evdev_consumer(u)))
                .map(KeyCode)
                .collect();
            let keyboard = VirtualDevice::builder()?.name("PeriHub virtual keyboard").with_keys(&keys)?.build()?;
            let buttons: AttributeSet<KeyCode> = BUTTONS.iter().map(|&c| KeyCode(c)).collect();
            // Pointer axes make desktops treat it as a mouse, so that its clicks count.
            let axes: AttributeSet<RelativeAxisCode> = [
                RelativeAxisCode::REL_X,
                RelativeAxisCode::REL_Y,
                RelativeAxisCode::REL_WHEEL,
                RelativeAxisCode::REL_HWHEEL,
                RelativeAxisCode::REL_WHEEL_HI_RES,
            ]
            .into_iter()
            .collect();
            let mouse = VirtualDevice::builder()?
                .name("PeriHub virtual mouse")
                .with_keys(&buttons)?
                .with_relative_axes(&axes)?
                .build()?;
            // The desktop needs a moment to pick up new devices: earlier events would be lost.
            std::thread::sleep(Duration::from_millis(300));
            Ok(Self { keyboard, mouse })
        }
    }

    fn key(code: u16, down: bool) -> InputEvent {
        InputEvent::new(EventType::KEY.0, code, i32::from(down))
    }

    impl Injector for Uinput {
        fn keys(&mut self, modifiers: u8, usage: u8, down: bool) -> Result<()> {
            let modifiers = (0..8).filter(|bit| modifiers >> bit & 1 != 0).map(|bit| key(MODIFIERS[bit], down));
            let main = (usage != 0)
                .then(|| evdev_key(usage).ok_or_else(|| Error::Invalid(format!("key {usage:#04x}"))))
                .transpose()?
                .map(|code| key(code, down));
            // One event per report, as a keyboard sends them.
            let events: Vec<InputEvent> =
                if down { modifiers.chain(main).collect() } else { main.into_iter().chain(modifiers).collect() };
            for event in events {
                self.keyboard.emit(&[event])?;
            }
            Ok(())
        }

        fn buttons(&mut self, buttons: u16, down: bool) -> Result<()> {
            let events: Vec<InputEvent> =
                (0..5).filter(|bit| buttons >> bit & 1 != 0).map(|bit| key(BUTTONS[bit], down)).collect();
            Ok(self.mouse.emit(&events)?)
        }

        fn consumer(&mut self, usage: u16, down: bool) -> Result<()> {
            let code = evdev_consumer(usage).ok_or_else(|| Error::Invalid(format!("media key {usage:#06x}")))?;
            Ok(self.keyboard.emit(&[key(code, down)])?)
        }

        fn wheel(&mut self, amount: i8) -> Result<()> {
            let amount = i32::from(amount);
            Ok(self.mouse.emit(&[
                InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_WHEEL.0, amount),
                InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_WHEEL_HI_RES.0, amount * 120),
            ])?)
        }

        fn text(&mut self, _text: &str) -> Result<()> {
            Err(Error::Invalid("typing text is not available on Linux yet".into()))
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::mem::size_of;

    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
        MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
        MOUSEEVENTF_XUP, MOUSEINPUT, SendInput as send_input, VIRTUAL_KEY,
    };

    use super::Injector;
    use crate::apex::input::usage_scan_code;
    use crate::error::{Error, Result};

    const XBUTTON1: u32 = 1;
    const XBUTTON2: u32 = 2;
    const WHEEL_DELTA: i32 = 120;
    /// Modifier usages by their bit in a HID report.
    const MODIFIERS: [u8; 8] = [0xE0, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7];

    pub struct SendInput;

    fn keyboard(vk: VIRTUAL_KEY, scan: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
        }
    }

    fn mouse(flags: u32, data: i32) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 },
            },
        }
    }

    fn send(inputs: &[INPUT]) -> Result<()> {
        if inputs.is_empty() {
            return Ok(());
        }
        // SAFETY: a valid slice of INPUT with the size of one element.
        let sent = unsafe { send_input(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32) };
        if sent as usize == inputs.len() { Ok(()) } else { Err(std::io::Error::last_os_error().into()) }
    }

    fn scan_input(usage: u8, down: bool) -> Result<INPUT> {
        let (scan, e0) = usage_scan_code(usage).ok_or_else(|| Error::Invalid(format!("key {usage:#04x}")))?;
        let mut flags = KEYEVENTF_SCANCODE;
        if e0 {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if !down {
            flags |= KEYEVENTF_KEYUP;
        }
        Ok(keyboard(0, scan, flags))
    }

    /// Virtual key of a media key, for the keys PeriHub offers.
    fn media_key(usage: u16) -> Option<VIRTUAL_KEY> {
        Some(match usage {
            0xCD => 0xB3,  // VK_MEDIA_PLAY_PAUSE
            0xB5 => 0xB0,  // VK_MEDIA_NEXT_TRACK
            0xB6 => 0xB1,  // VK_MEDIA_PREV_TRACK
            0xB7 => 0xB2,  // VK_MEDIA_STOP
            0xE2 => 0xAD,  // VK_VOLUME_MUTE
            0xE9 => 0xAF,  // VK_VOLUME_UP
            0xEA => 0xAE,  // VK_VOLUME_DOWN
            0x192 => 0xB7, // VK_LAUNCH_APP2 (calculator)
            0x194 => 0xB6, // VK_LAUNCH_APP1 (file explorer)
            0x18A => 0xB4, // VK_LAUNCH_MAIL
            0x223 => 0xAC, // VK_BROWSER_HOME
            0x224 => 0xA6, // VK_BROWSER_BACK
            0x225 => 0xA7, // VK_BROWSER_FORWARD
            0x227 => 0xA8, // VK_BROWSER_REFRESH
            _ => return None,
        })
    }

    impl Injector for SendInput {
        fn keys(&mut self, modifiers: u8, usage: u8, down: bool) -> Result<()> {
            let mut inputs = Vec::new();
            let mods: Vec<u8> = (0..8).filter(|bit| modifiers >> bit & 1 != 0).map(|bit| MODIFIERS[bit]).collect();
            if down {
                for m in &mods {
                    inputs.push(scan_input(*m, true)?);
                }
            }
            if usage != 0 {
                inputs.push(scan_input(usage, down)?);
            }
            if !down {
                for m in &mods {
                    inputs.push(scan_input(*m, false)?);
                }
            }
            send(&inputs)
        }

        fn buttons(&mut self, buttons: u16, down: bool) -> Result<()> {
            let flags = |d, u| if down { d } else { u };
            let inputs: Vec<INPUT> = (0..5)
                .filter(|bit| buttons >> bit & 1 != 0)
                .map(|bit| match bit {
                    0 => mouse(flags(MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP), 0),
                    1 => mouse(flags(MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP), 0),
                    2 => mouse(flags(MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP), 0),
                    3 => mouse(flags(MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP), XBUTTON1 as i32),
                    _ => mouse(flags(MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP), XBUTTON2 as i32),
                })
                .collect();
            send(&inputs)
        }

        fn consumer(&mut self, usage: u16, down: bool) -> Result<()> {
            let vk = media_key(usage).ok_or_else(|| Error::Invalid(format!("media key {usage:#06x}")))?;
            send(&[keyboard(vk, 0, if down { 0 } else { KEYEVENTF_KEYUP })])
        }

        fn wheel(&mut self, amount: i8) -> Result<()> {
            send(&[mouse(MOUSEEVENTF_WHEEL, i32::from(amount) * WHEEL_DELTA)])
        }

        /// Each character is sent as itself (`KEYEVENTF_UNICODE`), so accents work with any layout.
        /// Many games read physical keys only and ignore such characters.
        fn text(&mut self, text: &str) -> Result<()> {
            let inputs: Vec<INPUT> = text
                .encode_utf16()
                .flat_map(|unit| {
                    [keyboard(0, unit, KEYEVENTF_UNICODE), keyboard(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)]
                })
                .collect();
            send(&inputs)
        }
    }

    /// Opens a website or file with its default app, or starts a program, as Explorer would.
    pub fn shell_open(target: &str) -> Result<()> {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let (verb, file) = (wide("open"), wide(target));
        // SAFETY: both strings are NUL-terminated and live until the call returns.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // Values above 32 mean success.
        if result as usize > 32 { Ok(()) } else { Err(Error::Invalid(format!("cannot open {target}"))) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_key_codes() {
        assert_eq!(evdev_key(0x04), Some(30)); // A
        assert_eq!(evdev_key(0x1D), Some(44)); // Z
        assert_eq!(evdev_key(0x27), Some(11)); // 0
        assert_eq!(evdev_key(0x45), Some(88)); // F12
        assert_eq!(evdev_key(0x6E), Some(189)); // F19
        assert_eq!(evdev_key(0x73), Some(194)); // F24
        assert_eq!(evdev_key(0xE6), Some(100)); // Right Alt
        assert_eq!(evdev_consumer(0xE9), Some(115)); // volume up
        assert_eq!(evdev_key(0x00), None);
    }

    #[test]
    fn targets() {
        assert!(check_target("https://github.com/hex-dump/perihub").is_ok());
        assert!(check_target("http://example.org/a b").is_err());
        assert!(check_target("relative/path").is_err());
        assert!(check_target("file:///etc/passwd").is_err());
        assert!(check_target(&std::env::temp_dir().display().to_string()).is_ok());
    }
}
