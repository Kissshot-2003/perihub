//! Detects supported devices when they are plugged in or unplugged.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use hidapi::{DeviceInfo, HidApi};
use serde::Serialize;

use crate::apex::Apex3Tkl;
use crate::config::KeyboardConfig;
use crate::conflicts::{self, Conflict};
use crate::error::{Error, Result};
use crate::hidpp::HidppDevice;
use crate::hidpp::models::{self, MouseModel};
use crate::keyboard::{KeyInterfaces, KeyboardInfo, KeyboardTask};
use crate::mouse::{MouseInfo, MouseTask};
use crate::worker::Worker;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Conflicting programs are looked for every this many detection passes (10 seconds).
const CONFLICT_CHECK_EVERY: u32 = 5;

const STEELSERIES: u16 = 0x1038;
const APEX_3_TKL: u16 = 0x1622;
const LOGITECH: u16 = 0x046D;

fn is_keyboard_config_interface(d: &DeviceInfo) -> bool {
    d.vendor_id() == STEELSERIES
        && d.product_id() == APEX_3_TKL
        && d.interface_number() == 1
        && ((d.usage_page() == 0xFFC0 && d.usage() == 1) || d.usage_page() == 0)
}

/// "Long" HID++ collection (report 0x11) of a known mouse. On Windows it has a separate path
/// from the short one.
fn mouse_hidpp_interface(d: &DeviceInfo) -> Option<&'static MouseModel> {
    let long_collection =
        (d.usage_page() == 0xFF00 && d.usage() == 2) || (d.usage_page() == 0 && d.interface_number() == 1);
    if d.vendor_id() == LOGITECH && long_collection { models::find(d.product_id()) } else { None }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DeviceStatus {
    pub keyboard: Option<KeyboardInfo>,
    pub mouse: Option<MouseInfo>,
    /// Last connection error, to show to the user (e.g. missing permissions).
    pub keyboard_error: Option<String>,
    pub mouse_error: Option<String>,
    /// Running programs that take control of the devices, such as Logitech G HUB.
    pub conflicts: Vec<String>,
}

#[derive(Default)]
struct Slots {
    keyboard: Option<(Worker<KeyboardTask>, KeyboardInfo)>,
    mouse: Option<(Worker<MouseTask>, MouseInfo)>,
    /// Set by the mouse's thread while another program controls the mouse.
    mouse_host_mode: Option<Arc<AtomicBool>>,
    reported_host_mode: bool,
    conflicts: Vec<Conflict>,
    keyboard_error: Option<String>,
    mouse_error: Option<String>,
    /// Path of the open mouse, to notice when it goes away while another one is connected.
    mouse_path: Option<CString>,
    /// Unverified mouse that failed to open: not retried until it is plugged in again.
    mouse_failed: Option<CString>,
    /// Interfaces of the connected keyboard that report key presses.
    key_interfaces: KeyInterfaces,
}

pub type Listener = Box<dyn Fn(&DeviceStatus) + Send + Sync>;

#[derive(Clone)]
pub struct Manager {
    slots: Arc<Mutex<Slots>>,
    keyboard_config: Arc<Mutex<KeyboardConfig>>,
}

impl Manager {
    /// Starts detection in the background. `listener` is called on every change.
    pub fn start(keyboard_config: KeyboardConfig, listener: Listener) -> Self {
        let manager = Self {
            slots: Arc::new(Mutex::new(Slots::default())),
            keyboard_config: Arc::new(Mutex::new(keyboard_config)),
        };
        let background = manager.clone();
        thread::Builder::new()
            .name("perihub-hotplug".into())
            .spawn(move || background.hotplug_loop(listener))
            .expect("failed to start device detection");
        manager
    }

    fn hotplug_loop(&self, listener: Listener) {
        let mut api = match HidApi::new() {
            Ok(api) => api,
            Err(error) => {
                let mut slots = self.slots.lock().unwrap();
                slots.keyboard_error = Some(format!("HID unavailable: {error}"));
                slots.mouse_error = slots.keyboard_error.clone();
                drop(slots);
                listener(&self.status());
                return;
            }
        };
        let mut pass = 0u32;
        loop {
            let mut changed = self.poll(&mut api);
            if pass.is_multiple_of(CONFLICT_CHECK_EVERY) {
                changed |= self.check_conflicts();
            }
            if changed {
                listener(&self.status());
            }
            pass = pass.wrapping_add(1);
            thread::sleep(POLL_INTERVAL);
        }
    }

    /// One detection pass. Returns `true` if anything changed.
    fn poll(&self, api: &mut HidApi) -> bool {
        if let Err(error) = api.refresh_devices() {
            log::warn!("HID enumeration failed: {error}");
            return false;
        }
        let keyboard = api.device_list().find(|d| is_keyboard_config_interface(d)).cloned();
        let key_interface = |number| {
            api.device_list()
                .find(|d| {
                    d.vendor_id() == STEELSERIES && d.product_id() == APEX_3_TKL && d.interface_number() == number
                })
                .map(|d| d.path().to_owned())
        };
        let key_interfaces = KeyInterfaces { boot: key_interface(0), bitmap: key_interface(2) };
        let mice: Vec<(DeviceInfo, &'static MouseModel)> =
            api.device_list().filter_map(|d| mouse_hidpp_interface(d).map(|m| (d.clone(), m))).collect();
        let mut changed = false;

        let keyboard_present = {
            let mut slots = self.slots.lock().unwrap();
            if slots.keyboard.as_ref().is_some_and(|(w, _)| !w.is_alive() || keyboard.is_none()) {
                slots.keyboard = None;
                slots.key_interfaces = KeyInterfaces::default();
                changed = true;
            }
            slots.keyboard.is_some()
        };
        if let (false, Some(info)) = (keyboard_present, &keyboard) {
            let config = self.keyboard_config.lock().unwrap().clone();
            let result = info
                .open_device(api)
                .map_err(Error::from)
                .and_then(|dev| KeyboardTask::open(Apex3Tkl::new(dev), &config, key_interfaces.clone()));
            let mut slots = self.slots.lock().unwrap();
            let error = match result {
                Ok(task) => {
                    let info = task.info.clone();
                    slots.keyboard = Some((Worker::spawn("perihub-keyboard", task), info));
                    slots.key_interfaces = key_interfaces;
                    None
                }
                Err(error) => Some(open_error_message(error)),
            };
            changed |= slots.keyboard.is_some() || slots.keyboard_error != error;
            slots.keyboard_error = error;
        }

        let connected = |path: &Option<CString>| mice.iter().any(|(d, _)| Some(d.path()) == path.as_deref());
        let candidate = {
            let mut slots = self.slots.lock().unwrap();
            if slots.mouse.as_ref().is_some_and(|(w, _)| !w.is_alive()) || !connected(&slots.mouse_path) {
                changed |= slots.mouse.take().is_some();
                slots.mouse_path = None;
            }
            if !connected(&slots.mouse_failed) {
                slots.mouse_failed = None;
            }
            if slots.mouse.is_some() {
                None
            } else {
                // A tested model wins if more than one supported mouse is connected.
                let failed = slots.mouse_failed.clone();
                mice.iter().filter(|(d, _)| Some(d.path()) != failed.as_deref()).max_by_key(|(_, m)| m.verified)
            }
        };
        if let Some((info, model)) = candidate {
            let result = info
                .open_device(api)
                .map_err(Error::from)
                .and_then(|dev| HidppDevice::new(dev, model.device_index, !model.verified))
                .and_then(|dev| MouseTask::open(dev, model));
            let mut slots = self.slots.lock().unwrap();
            let error = match result {
                Ok(task) => {
                    let mouse = task.info.clone();
                    slots.mouse_host_mode = Some(task.host_mode_flag());
                    slots.reported_host_mode = mouse.host_mode;
                    slots.mouse = Some((Worker::spawn("perihub-mouse", task), mouse));
                    slots.mouse_path = Some(info.path().to_owned());
                    None
                }
                Err(error) => {
                    if !model.verified && !is_permission_error(&error) {
                        slots.mouse_failed = Some(info.path().to_owned());
                    }
                    Some(format!("{}: {}", model.name, open_error_message(error)))
                }
            };
            changed |= slots.mouse.is_some() || slots.mouse_error != error;
            slots.mouse_error = error;
        }
        // The mouse's thread notices when another program takes the mouse or gives it back.
        let mut slots = self.slots.lock().unwrap();
        let host_mode =
            slots.mouse.is_some() && slots.mouse_host_mode.as_ref().is_some_and(|f| f.load(Ordering::SeqCst));
        if host_mode != slots.reported_host_mode {
            slots.reported_host_mode = host_mode;
            changed = true;
        }
        changed
    }

    /// Looks for running programs that take control of the devices. Returns `true` if the list changed.
    fn check_conflicts(&self) -> bool {
        let running = conflicts::running();
        let mut slots = self.slots.lock().unwrap();
        if slots.conflicts == running {
            return false;
        }
        log::info!("programs that conflict with PeriHub: {:?}", running.iter().map(|c| c.name()).collect::<Vec<_>>());
        slots.conflicts = running;
        true
    }

    pub fn status(&self) -> DeviceStatus {
        let slots = self.slots.lock().unwrap();
        DeviceStatus {
            keyboard: slots.keyboard.as_ref().map(|(_, info)| info.clone()),
            mouse: slots
                .mouse
                .as_ref()
                .map(|(_, info)| MouseInfo { host_mode: slots.reported_host_mode, ..info.clone() }),
            keyboard_error: slots.keyboard_error.clone(),
            mouse_error: slots.mouse_error.clone(),
            conflicts: slots.conflicts.iter().map(|c| c.name().to_string()).collect(),
        }
    }

    pub fn keyboard(&self) -> Result<Worker<KeyboardTask>> {
        self.slots.lock().unwrap().keyboard.as_ref().map(|(w, _)| w.clone()).ok_or(Error::NotConnected)
    }

    pub fn mouse(&self) -> Result<Worker<MouseTask>> {
        self.slots.lock().unwrap().mouse.as_ref().map(|(w, _)| w.clone()).ok_or(Error::NotConnected)
    }

    /// Whether PeriHub may read the keyboard's key presses (for reactive lighting), or `None` if
    /// the keyboard is not connected. Only opens the interface for a moment; nothing is read.
    pub fn key_presses_readable(&self) -> Option<bool> {
        let path = self.slots.lock().unwrap().key_interfaces.boot.clone()?;
        Some(HidApi::new().and_then(|api| api.open_path(&path)).is_ok())
    }

    /// Settings applied whenever the keyboard is connected.
    pub fn keyboard_config(&self) -> KeyboardConfig {
        self.keyboard_config.lock().unwrap().clone()
    }

    /// Updates the settings to reapply whenever the keyboard is reconnected.
    pub fn set_keyboard_config(&self, config: KeyboardConfig) {
        *self.keyboard_config.lock().unwrap() = config;
    }
}

fn is_permission_error(error: &Error) -> bool {
    let text = error.to_string();
    text.contains("Permission denied") || text.contains("ermesso") || text.contains("ccess is denied")
}

fn open_error_message(error: Error) -> String {
    if is_permission_error(&error) {
        "permission denied: install the PeriHub udev rule (see the README)".into()
    } else {
        error.to_string()
    }
}
