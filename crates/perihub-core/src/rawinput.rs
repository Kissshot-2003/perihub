//! Windows: key presses of chosen devices through Raw Input, shared by everything that needs them
//! (reactive lighting, software macro triggers).
//!
//! A process gets keyboard Raw Input in one window only: a second registration would replace the
//! first. So one thread listens, while at least one subscriber needs it, and hands each key to the
//! subscribers of the device it comes from. Keys simulated by programs (SendInput, including
//! PeriHub's own software macros) come with no device and are dropped. Nothing is stored.

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use windows_sys::Win32::Foundation::{HANDLE, HWND};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT,
    RIDEV_INPUTSINK, RIDEV_REMOVE, RIDI_DEVICENAME, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG, PostThreadMessageW, RI_KEY_BREAK,
    RI_KEY_E0, WM_INPUT, WM_QUIT,
};

use crate::apex::input::scan_code_usage;
use crate::error::{Error, Result};

const GENERIC_DESKTOP: u16 = 0x01;
const KEYBOARD: u16 = 0x06;

/// A key of a subscribed device, as a HID usage.
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub usage: u8,
    pub released: bool,
}

type Callback = Box<dyn Fn(KeyEvent) + Send>;

struct Subscriber {
    id: u64,
    /// Part of the Raw Input device name, e.g. `VID_1038&PID_1622`.
    device: String,
    on_key: Callback,
}

struct Listener {
    thread_id: u32,
    subscribers: Arc<Mutex<Vec<Subscriber>>>,
}

static LISTENER: Mutex<Option<Listener>> = Mutex::new(None);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Keys of the device whose Raw Input name contains `device` (case does not matter), until dropped.
pub struct Subscription(u64);

pub fn subscribe(device: &str, on_key: impl Fn(KeyEvent) + Send + 'static) -> Result<Subscription> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let subscriber = Subscriber { id, device: device.to_uppercase(), on_key: Box::new(on_key) };
    let mut listener = LISTENER.lock().unwrap();
    if listener.is_none() {
        let subscribers = Arc::new(Mutex::new(Vec::new()));
        let thread_id = spawn(subscribers.clone())?;
        *listener = Some(Listener { thread_id, subscribers });
    }
    listener.as_ref().expect("started above").subscribers.lock().unwrap().push(subscriber);
    Ok(Subscription(id))
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let mut listener = LISTENER.lock().unwrap();
        let Some(running) = listener.as_ref() else { return };
        let mut subscribers = running.subscribers.lock().unwrap();
        subscribers.retain(|s| s.id != self.0);
        // Nobody needs keys any more: stop receiving them at all.
        if subscribers.is_empty() {
            drop(subscribers);
            // SAFETY: posting a message to a thread id has no memory effects.
            unsafe { PostThreadMessageW(running.thread_id, WM_QUIT, 0, 0) };
            *listener = None;
        }
    }
}

fn spawn(subscribers: Arc<Mutex<Vec<Subscriber>>>) -> Result<u32> {
    let (started_tx, started_rx) = mpsc::channel();
    thread::Builder::new().name("perihub-rawinput".into()).spawn(move || run(&subscribers, &started_tx))?;
    started_rx.recv().map_err(|_| Error::Invalid("the key thread stopped".into()))?
}

fn register(window: HWND, flags: u32) -> bool {
    let device = RAWINPUTDEVICE { usUsagePage: GENERIC_DESKTOP, usUsage: KEYBOARD, dwFlags: flags, hwndTarget: window };
    // SAFETY: one valid RAWINPUTDEVICE with its size.
    unsafe { RegisterRawInputDevices(&device, 1, size_of::<RAWINPUTDEVICE>() as u32) != 0 }
}

fn run(subscribers: &Mutex<Vec<Subscriber>>, started: &mpsc::Sender<Result<u32>>) {
    let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    // SAFETY: a message-only window of a system class, owned and destroyed by this thread.
    let window = unsafe {
        CreateWindowExW(0, class.as_ptr(), null(), 0, 0, 0, 0, 0, HWND_MESSAGE, null_mut(), null_mut(), null())
    };
    if window.is_null() {
        let _ = started.send(Err(std::io::Error::last_os_error().into()));
        return;
    }
    // RIDEV_INPUTSINK: keys arrive even while another app is in the foreground. This replaces the
    // keyboard registration of the window library, which PeriHub does not use.
    if !register(window, RIDEV_INPUTSINK) {
        let _ = started.send(Err(std::io::Error::last_os_error().into()));
        // SAFETY: the window belongs to this thread.
        unsafe { DestroyWindow(window) };
        return;
    }
    // SAFETY: no arguments.
    let _ = started.send(Ok(unsafe { GetCurrentThreadId() }));

    let mut names: HashMap<usize, String> = HashMap::new();
    // SAFETY: MSG is plain data, filled in by GetMessageW.
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    // SAFETY: standard message loop on this thread; ends with WM_QUIT.
    while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {
        if msg.message == WM_INPUT
            && let Some((device, event)) = key(msg.lParam as HRAWINPUT)
        {
            let name = names.entry(device as usize).or_insert_with(|| device_name(device).unwrap_or_default());
            for subscriber in subscribers.lock().unwrap().iter().filter(|s| name.contains(&s.device)) {
                (subscriber.on_key)(event);
            }
        }
        // WM_INPUT must reach the default window procedure, which frees the input data.
        // SAFETY: the message came from GetMessageW.
        unsafe { DispatchMessageW(&msg) };
    }
    register(null_mut(), RIDEV_REMOVE);
    // SAFETY: the window belongs to this thread.
    unsafe { DestroyWindow(window) };
}

/// Device and key of a keyboard event; `None` for simulated keys (no device) and unknown keys.
fn key(handle: HRAWINPUT) -> Option<(HANDLE, KeyEvent)> {
    let mut input = RAWINPUT::default();
    let mut size = size_of::<RAWINPUT>() as u32;
    // SAFETY: the buffer is a RAWINPUT and `size` is its size.
    let read = unsafe {
        GetRawInputData(
            handle,
            RID_INPUT,
            (&raw mut input).cast::<c_void>(),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if read == u32::MAX || input.header.dwType != RIM_TYPEKEYBOARD || input.header.hDevice.is_null() {
        return None;
    }
    // SAFETY: dwType says the union holds keyboard data.
    let keyboard = unsafe { input.data.keyboard };
    let flags = u32::from(keyboard.Flags);
    let usage = scan_code_usage(keyboard.MakeCode, flags & RI_KEY_E0 != 0)?;
    Some((input.header.hDevice, KeyEvent { usage, released: flags & RI_KEY_BREAK != 0 }))
}

/// Raw Input device name in upper case; it contains the USB ids, e.g. `\\?\HID#VID_1038&PID_1622&MI_00#…`.
fn device_name(device: HANDLE) -> Option<String> {
    let mut len = 0u32;
    // SAFETY: first call only asks for the length; the second fills a buffer of that length.
    unsafe {
        GetRawInputDeviceInfoW(device, RIDI_DEVICENAME, null_mut(), &mut len);
        if len == 0 {
            return None;
        }
        let mut name = vec![0u16; len as usize];
        if GetRawInputDeviceInfoW(device, RIDI_DEVICENAME, name.as_mut_ptr().cast(), &mut len) == u32::MAX {
            return None;
        }
        Some(String::from_utf16_lossy(&name).to_uppercase())
    }
}
