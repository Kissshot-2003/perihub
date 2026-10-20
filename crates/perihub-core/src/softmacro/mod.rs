//! Software macros: run by PeriHub, so they can do what a device's own macros cannot: start and
//! stop with two presses, repeat a number of times, type text, open an app or a website.
//!
//! A mouse button or a keyboard key that starts one sends an unused key from the device's own memory
//! (F19, F24, then F13–F18); PeriHub reads that key from the device itself and plays the macro
//! through a simulated keyboard and mouse. Keys are read from the devices, never from the system, so
//! other programs, and the keys PeriHub simulates, cannot start a macro. The keyboard is read only
//! while one of its keys sends the key of a macro, and every other key is dropped as it arrives:
//! nothing that is typed is kept or logged.

pub mod inject;
#[cfg(target_os = "linux")]
mod layout;
mod trigger;

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use inject::Injector;

/// Keys a button sends to start a software macro, in the order they are given out. F19 and F24
/// do nothing on any desktop; F13–F18 are launcher keys some desktops use. F20–F23 are never used:
/// on Linux they mute the microphone and switch the touchpad.
pub const TRIGGER_KEYS: [u8; 8] = [0x6E, 0x73, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D];

/// Shortest time between two rounds of a macro that repeats until stopped, so that one without
/// pauses cannot flood the system.
const MIN_ROUND: Duration = Duration::from_millis(20);
/// How often a pause checks whether the macro was stopped.
const STOP_CHECK: Duration = Duration::from_millis(20);

/// One action of a software macro. Keys, buttons, media keys, the wheel and pauses work like in
/// the device macros ([`crate::hidpp::macros::MacroStep`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SoftStep {
    KeyDown {
        modifiers: u8,
        key: u8,
    },
    KeyUp {
        modifiers: u8,
        key: u8,
    },
    /// Mouse buttons, bit 0 = left, 1 = right, 2 = middle, 3 = back, 4 = forward.
    MouseDown {
        buttons: u16,
    },
    MouseUp {
        buttons: u16,
    },
    ConsumerDown {
        usage: u16,
    },
    ConsumerUp {
        usage: u16,
    },
    /// Wheel notches: positive scrolls up.
    Wheel {
        amount: i8,
    },
    Delay {
        ms: u32,
    },
    /// Types the text as it is, whatever the keyboard layout.
    Text {
        text: String,
    },
    /// Opens a website (http or https address) or a file or program.
    Open {
        target: String,
    },
}

/// How a software macro runs when its button is pressed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SoftMode {
    Once,
    Repeat {
        times: u32,
    },
    /// Repeats until the button is pressed again.
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftwareMacro {
    /// Key from [`TRIGGER_KEYS`] that the buttons running this macro send.
    pub key: u8,
    pub name: String,
    pub mode: SoftMode,
    pub steps: Vec<SoftStep>,
}

impl SoftwareMacro {
    pub fn validate(&self) -> Result<()> {
        if !TRIGGER_KEYS.contains(&self.key) {
            return Err(Error::Invalid(format!("key {:#04x} cannot start a macro", self.key)));
        }
        if self.steps.is_empty() {
            return Err(Error::Invalid("the macro has no steps".into()));
        }
        if let SoftMode::Repeat { times } = self.mode
            && !(1..=10_000).contains(&times)
        {
            return Err(Error::Invalid("a macro repeats between 1 and 10000 times".into()));
        }
        for step in &self.steps {
            if let SoftStep::Open { target } = step {
                inject::check_target(target)?;
            }
        }
        Ok(())
    }
}

/// Keys, buttons and media keys a running macro holds down, released when it ends or is stopped.
#[derive(Default)]
struct Held {
    modifiers: u8,
    keys: HashSet<u8>,
    buttons: u16,
    consumer: HashSet<u16>,
}

/// What the interface needs to know.
#[derive(Debug, Clone, Serialize)]
pub struct SoftMacroStatus {
    /// PeriHub can simulate a keyboard and a mouse (on Linux this needs access to /dev/uinput).
    pub available: bool,
    /// Why not, if it cannot.
    pub error: Option<String>,
    /// Keys of the macros running now.
    pub running: Vec<u8>,
    /// PeriHub reads the keyboard's keys, because one of them starts a macro.
    pub keyboard_read: bool,
    /// A keyboard key starts a macro, but PeriHub cannot read the keyboard's keys (on Linux this
    /// needs the optional rule for its keys).
    pub keyboard_error: Option<String>,
}

/// The keyboard as a source of trigger keys: the interfaces that report its keys and the trigger
/// keys its key map sends, on either layer.
#[derive(Debug, Clone, Default)]
pub struct KeyboardTriggers {
    pub boot: Option<CString>,
    pub bitmap: Option<CString>,
    pub keys: Vec<u8>,
}

pub struct SoftMacroEngine {
    macros: Mutex<Vec<SoftwareMacro>>,
    /// Stop flags of the macros running now, by key.
    running: Mutex<HashMap<u8, Arc<AtomicBool>>>,
    /// Created the first time a macro runs.
    injector: Mutex<Option<Box<dyn Injector>>>,
    mouse_reader: Mutex<Option<trigger::Trigger>>,
    keyboard_reader: Mutex<Option<trigger::Trigger>>,
    keyboard_error: Mutex<Option<String>>,
}

impl SoftMacroEngine {
    pub fn new(macros: Vec<SoftwareMacro>) -> Arc<Self> {
        Arc::new(Self {
            macros: Mutex::new(macros),
            running: Mutex::default(),
            injector: Mutex::default(),
            mouse_reader: Mutex::default(),
            keyboard_reader: Mutex::default(),
            keyboard_error: Mutex::default(),
        })
    }

    pub fn set_macros(&self, macros: Vec<SoftwareMacro>) {
        *self.macros.lock().unwrap() = macros;
    }

    /// Follows the mouse: reads its trigger keys while at least one macro exists. `mouse` is the
    /// path of its HID interface that also carries its keyboard reports and its USB product id,
    /// or `None` without a mouse.
    pub fn watch_mouse(self: &Arc<Self>, mouse: Option<(CString, u16)>) {
        let wanted = mouse
            .filter(|_| !self.macros.lock().unwrap().is_empty())
            .map(|(path, product_id)| trigger::Source::Mouse { path, product_id });
        if let Err(error) = self.watch(&self.mouse_reader, wanted) {
            log::warn!("software macros: cannot read the mouse's keys: {error}");
        }
    }

    /// Follows the keyboard: reads its keys only while one of them sends the key of a macro.
    /// `None` without a keyboard.
    pub fn watch_keyboard(self: &Arc<Self>, keyboard: Option<KeyboardTriggers>) {
        let used = |key: &u8| self.macros.lock().unwrap().iter().any(|m| m.key == *key);
        let wanted = keyboard
            .filter(|k| k.keys.iter().any(used))
            .map(|k| trigger::Source::Keyboard { boot: k.boot, bitmap: k.bitmap });
        let error = self.watch(&self.keyboard_reader, wanted).err().map(|e| e.to_string());
        let mut last = self.keyboard_error.lock().unwrap();
        // Called every few seconds while the macros are on screen: log a failure once.
        if let Some(error) = error.as_ref().filter(|_| *last != error) {
            log::warn!("software macros: cannot read the keyboard's keys: {error}");
        }
        *last = error;
    }

    /// Starts, keeps or stops one device's reader.
    fn watch(self: &Arc<Self>, slot: &Mutex<Option<trigger::Trigger>>, wanted: Option<trigger::Source>) -> Result<()> {
        let mut reader = slot.lock().unwrap();
        if reader.as_ref().filter(|r| r.is_alive()).map(|r| &r.source) == wanted.as_ref() {
            return Ok(());
        }
        *reader = None;
        let Some(source) = wanted else { return Ok(()) };
        let engine = Arc::downgrade(self);
        *reader = Some(trigger::Trigger::start(
            source,
            Arc::new(move |key| {
                if let Some(engine) = engine.upgrade() {
                    engine.pressed(key);
                }
            }),
        )?);
        Ok(())
    }

    pub fn status(&self) -> SoftMacroStatus {
        let (available, error) = match inject::available() {
            Ok(()) => (true, None),
            Err(error) => (false, Some(error.to_string())),
        };
        SoftMacroStatus {
            available,
            error,
            running: self.running.lock().unwrap().keys().copied().collect(),
            keyboard_read: self.keyboard_reader.lock().unwrap().as_ref().is_some_and(|r| r.is_alive()),
            keyboard_error: self.keyboard_error.lock().unwrap().clone(),
        }
    }

    /// A trigger key was pressed on a device.
    fn pressed(self: &Arc<Self>, key: u8) {
        let Some(m) = self.macros.lock().unwrap().iter().find(|m| m.key == key).cloned() else { return };
        let mut running = self.running.lock().unwrap();
        if let Some(stop) = running.get(&key) {
            // Pressing the button of a running macro stops it if it repeats until stopped; a macro
            // that runs once or a set number of times just finishes.
            if m.mode == SoftMode::Toggle {
                stop.store(true, Ordering::SeqCst);
            }
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        running.insert(key, stop.clone());
        drop(running);
        let engine = self.clone();
        let spawned = thread::Builder::new().name("perihub-macro".into()).spawn(move || {
            engine.play(&m, &stop);
            let mut running = engine.running.lock().unwrap();
            if running.get(&m.key).is_some_and(|s| Arc::ptr_eq(s, &stop)) {
                running.remove(&m.key);
            }
        });
        if let Err(error) = spawned {
            log::warn!("software macro not started: {error}");
            self.running.lock().unwrap().remove(&key);
        }
    }

    /// Stops every running macro; their keys are released by their threads.
    pub fn stop_all(&self) {
        for stop in self.running.lock().unwrap().values() {
            stop.store(true, Ordering::SeqCst);
        }
    }

    /// Stops every running macro and waits a little for their keys to be released, before quitting:
    /// on Windows a key PeriHub pressed would otherwise stay down.
    pub fn stop_all_and_wait(&self) {
        self.stop_all();
        let deadline = Instant::now() + Duration::from_millis(500);
        while !self.running.lock().unwrap().is_empty() && Instant::now() < deadline {
            thread::sleep(STOP_CHECK);
        }
    }

    /// The macros, as saved.
    pub fn macros(&self) -> Vec<SoftwareMacro> {
        self.macros.lock().unwrap().clone()
    }

    fn play(&self, m: &SoftwareMacro, stop: &AtomicBool) {
        let rounds = match m.mode {
            SoftMode::Once => 1,
            SoftMode::Repeat { times } => times,
            SoftMode::Toggle => u32::MAX,
        };
        let mut held = Held::default();
        'rounds: for _ in 0..rounds {
            let round = Instant::now();
            for step in &m.steps {
                if stop.load(Ordering::SeqCst) {
                    break 'rounds;
                }
                if let Err(error) = self.step(step, &mut held, stop) {
                    log::warn!("software macro stopped: {error}");
                    break 'rounds;
                }
            }
            if m.mode == SoftMode::Toggle {
                sleep_unless_stopped(MIN_ROUND.saturating_sub(round.elapsed()), stop);
            }
        }
        if let Err(error) = self.release(&mut held) {
            log::warn!("software macro: keys not released: {error}");
        }
    }

    fn inject(&self, action: impl FnOnce(&mut dyn Injector) -> Result<()>) -> Result<()> {
        let mut injector = self.injector.lock().unwrap();
        if injector.is_none() {
            *injector = Some(inject::open()?);
        }
        action(injector.as_deref_mut().expect("just created"))
    }

    fn step(&self, step: &SoftStep, held: &mut Held, stop: &AtomicBool) -> Result<()> {
        match step {
            SoftStep::KeyDown { modifiers, key } => {
                self.inject(|i| i.keys(*modifiers & !held.modifiers, *key, true))?;
                held.modifiers |= modifiers;
                if *key != 0 {
                    held.keys.insert(*key);
                }
            }
            SoftStep::KeyUp { modifiers, key } => {
                self.inject(|i| i.keys(*modifiers & held.modifiers, *key, false))?;
                held.modifiers &= !modifiers;
                held.keys.remove(key);
            }
            SoftStep::MouseDown { buttons } => {
                self.inject(|i| i.buttons(*buttons, true))?;
                held.buttons |= buttons;
            }
            SoftStep::MouseUp { buttons } => {
                self.inject(|i| i.buttons(*buttons, false))?;
                held.buttons &= !buttons;
            }
            SoftStep::ConsumerDown { usage } => {
                self.inject(|i| i.consumer(*usage, true))?;
                held.consumer.insert(*usage);
            }
            SoftStep::ConsumerUp { usage } => {
                self.inject(|i| i.consumer(*usage, false))?;
                held.consumer.remove(usage);
            }
            SoftStep::Wheel { amount } => self.inject(|i| i.wheel(*amount))?,
            SoftStep::Delay { ms } => sleep_unless_stopped(Duration::from_millis(u64::from(*ms)), stop),
            SoftStep::Text { text } => self.inject(|i| i.text(text))?,
            SoftStep::Open { target } => inject::open_target(target)?,
        }
        Ok(())
    }

    fn release(&self, held: &mut Held) -> Result<()> {
        let held = std::mem::take(held);
        if held.modifiers == 0 && held.keys.is_empty() && held.buttons == 0 && held.consumer.is_empty() {
            return Ok(());
        }
        self.inject(|i| {
            for key in &held.keys {
                i.keys(0, *key, false)?;
            }
            if held.modifiers != 0 {
                i.keys(held.modifiers, 0, false)?;
            }
            if held.buttons != 0 {
                i.buttons(held.buttons, false)?;
            }
            for usage in &held.consumer {
                i.consumer(*usage, false)?;
            }
            Ok(())
        })
    }
}

impl Drop for SoftMacroEngine {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn sleep_unless_stopped(duration: Duration, stop: &AtomicBool) {
    let end = Instant::now() + duration;
    while !stop.load(Ordering::SeqCst) {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        thread::sleep(left.min(STOP_CHECK));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tap() -> Vec<SoftStep> {
        vec![SoftStep::KeyDown { modifiers: 0, key: 4 }, SoftStep::KeyUp { modifiers: 0, key: 4 }]
    }

    #[test]
    fn validation() {
        let ok = SoftwareMacro { key: TRIGGER_KEYS[0], name: "a".into(), mode: SoftMode::Once, steps: tap() };
        assert!(ok.validate().is_ok());
        // F20 mutes the microphone on Linux: never a trigger.
        assert!(SoftwareMacro { key: 0x6F, ..ok.clone() }.validate().is_err());
        assert!(SoftwareMacro { steps: vec![], ..ok.clone() }.validate().is_err());
        assert!(SoftwareMacro { mode: SoftMode::Repeat { times: 0 }, ..ok.clone() }.validate().is_err());
        let open = |target: &str| SoftwareMacro { steps: vec![SoftStep::Open { target: target.into() }], ..ok.clone() };
        assert!(open("https://example.org").validate().is_ok());
        assert!(open("rm -rf /").validate().is_err());
        assert!(open("javascript:alert(1)").validate().is_err());
    }

    #[test]
    fn json_shape_matches_the_device_macros() {
        let step = serde_json::to_value(SoftStep::KeyDown { modifiers: 2, key: 4 }).unwrap();
        assert_eq!(step, serde_json::json!({ "type": "key_down", "modifiers": 2, "key": 4 }));
        let mode = serde_json::to_value(SoftMode::Repeat { times: 3 }).unwrap();
        assert_eq!(mode, serde_json::json!({ "type": "repeat", "times": 3 }));
    }

    #[test]
    fn a_pause_ends_early_when_stopped() {
        let stop = AtomicBool::new(true);
        let start = Instant::now();
        sleep_unless_stopped(Duration::from_secs(5), &stop);
        assert!(start.elapsed() < Duration::from_millis(100));
    }
}
