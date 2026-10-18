//! Detects supported devices when they are plugged in or unplugged.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use hidapi::{DeviceInfo, HidApi};
use serde::Serialize;

use crate::apex::Apex3Tkl;
use crate::config::KeyboardConfig;
use crate::error::{Error, Result};
use crate::hidpp::HidppDevice;
use crate::keyboard::{KeyboardInfo, KeyboardTask};
use crate::mouse::{MouseInfo, MouseTask};
use crate::worker::Worker;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

const STEELSERIES: u16 = 0x1038;
const APEX_3_TKL: u16 = 0x1622;
const LOGITECH: u16 = 0x046D;
const G502_HERO: u16 = 0xC08B;

fn is_keyboard_config_interface(d: &DeviceInfo) -> bool {
    d.vendor_id() == STEELSERIES
        && d.product_id() == APEX_3_TKL
        && d.interface_number() == 1
        && ((d.usage_page() == 0xFFC0 && d.usage() == 1) || d.usage_page() == 0)
}

/// "Long" HID++ collection (report 0x11). On Windows it has a separate path from the short one.
fn is_mouse_hidpp_interface(d: &DeviceInfo) -> bool {
    d.vendor_id() == LOGITECH
        && d.product_id() == G502_HERO
        && d.interface_number() == 1
        && ((d.usage_page() == 0xFF00 && d.usage() == 2) || d.usage_page() == 0)
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DeviceStatus {
    pub keyboard: Option<KeyboardInfo>,
    pub mouse: Option<MouseInfo>,
    /// Last connection error, to show to the user (e.g. missing permissions).
    pub keyboard_error: Option<String>,
    pub mouse_error: Option<String>,
}

#[derive(Default)]
struct Slots {
    keyboard: Option<(Worker<KeyboardTask>, KeyboardInfo)>,
    mouse: Option<(Worker<MouseTask>, MouseInfo)>,
    keyboard_error: Option<String>,
    mouse_error: Option<String>,
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
        loop {
            if self.poll(&mut api) {
                listener(&self.status());
            }
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
        let mouse = api.device_list().find(|d| is_mouse_hidpp_interface(d)).cloned();
        let mut changed = false;

        let keyboard_present = {
            let mut slots = self.slots.lock().unwrap();
            if slots.keyboard.as_ref().is_some_and(|(w, _)| !w.is_alive() || keyboard.is_none()) {
                slots.keyboard = None;
                changed = true;
            }
            slots.keyboard.is_some()
        };
        if let (false, Some(info)) = (keyboard_present, &keyboard) {
            let config = self.keyboard_config.lock().unwrap().clone();
            let result = info
                .open_device(api)
                .map_err(Error::from)
                .and_then(|dev| KeyboardTask::open(Apex3Tkl::new(dev), &config));
            let mut slots = self.slots.lock().unwrap();
            let error = match result {
                Ok(task) => {
                    let info = task.info.clone();
                    slots.keyboard = Some((Worker::spawn("perihub-keyboard", task), info));
                    None
                }
                Err(error) => Some(open_error_message(error)),
            };
            changed |= slots.keyboard.is_some() || slots.keyboard_error != error;
            slots.keyboard_error = error;
        }

        let mouse_present = {
            let mut slots = self.slots.lock().unwrap();
            if slots.mouse.as_ref().is_some_and(|(w, _)| !w.is_alive() || mouse.is_none()) {
                slots.mouse = None;
                changed = true;
            }
            slots.mouse.is_some()
        };
        if let (false, Some(info)) = (mouse_present, &mouse) {
            let result =
                info.open_device(api).map_err(Error::from).and_then(HidppDevice::new).and_then(MouseTask::open);
            let mut slots = self.slots.lock().unwrap();
            let error = match result {
                Ok(task) => {
                    let info = task.info.clone();
                    slots.mouse = Some((Worker::spawn("perihub-mouse", task), info));
                    None
                }
                Err(error) => Some(open_error_message(error)),
            };
            changed |= slots.mouse.is_some() || slots.mouse_error != error;
            slots.mouse_error = error;
        }
        changed
    }

    pub fn status(&self) -> DeviceStatus {
        let slots = self.slots.lock().unwrap();
        DeviceStatus {
            keyboard: slots.keyboard.as_ref().map(|(_, info)| info.clone()),
            mouse: slots.mouse.as_ref().map(|(_, info)| info.clone()),
            keyboard_error: slots.keyboard_error.clone(),
            mouse_error: slots.mouse_error.clone(),
        }
    }

    pub fn keyboard(&self) -> Result<Worker<KeyboardTask>> {
        self.slots.lock().unwrap().keyboard.as_ref().map(|(w, _)| w.clone()).ok_or(Error::NotConnected)
    }

    pub fn mouse(&self) -> Result<Worker<MouseTask>> {
        self.slots.lock().unwrap().mouse.as_ref().map(|(w, _)| w.clone()).ok_or(Error::NotConnected)
    }

    /// Updates the settings to reapply whenever the keyboard is reconnected.
    pub fn set_keyboard_config(&self, config: KeyboardConfig) {
        *self.keyboard_config.lock().unwrap() = config;
    }
}

fn open_error_message(error: Error) -> String {
    let text = error.to_string();
    if text.contains("Permission denied") || text.contains("ermesso") || text.contains("ccess is denied") {
        "permission denied: install the PeriHub udev rule (see the README)".into()
    } else {
        text
    }
}
