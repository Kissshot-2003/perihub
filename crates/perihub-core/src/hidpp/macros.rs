//! Macros stored in the mouse's memory (HID++ 0x8100, macro format 1).
//!
//! A button bound to a macro points to a sector and an offset ([`ButtonAction::Macro`]). From
//! there the mouse runs commands until `0xFF`; `0x60` continues somewhere else, for example in
//! another sector. Opcodes come from libratbag (MIT, `src/hidpp20.h`).
//!
//! Key presses, key releases and delays were checked on a real G502 HERO: a macro made of them
//! typed the expected text. Other commands (mouse buttons, media keys, repeat…) are not read
//! yet: a macro that uses them is reported as unsupported instead of being guessed.
//!
//! [`ButtonAction::Macro`]: super::profile::ButtonAction::Macro

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const OP_NOOP: u8 = 0x00;
const OP_DELAY: u8 = 0x40;
const OP_KEY_DOWN: u8 = 0x43;
const OP_KEY_UP: u8 = 0x44;
const OP_JUMP: u8 = 0x60;
const OP_END: u8 = 0xFF;

/// Upper bound on the commands of one macro, against corrupt memory.
const MAX_STEPS: usize = 1024;

/// One command of a macro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MacroStep {
    /// Keyboard key (HID usage) pressed, with the standard HID modifiers as in a key binding.
    KeyDown {
        modifiers: u8,
        key: u8,
    },
    KeyUp {
        modifiers: u8,
        key: u8,
    },
    Delay {
        ms: u16,
    },
    /// Does nothing for the duration of one step.
    Noop,
}

/// Reads the macro that starts at `offset` in `sector`, following jumps.
/// `read_sector` returns a sector's bytes; the last two are its CRC and hold no commands.
pub fn parse(mut read_sector: impl FnMut(u16) -> Result<Vec<u8>>, sector: u8, offset: u8) -> Result<Vec<MacroStep>> {
    let mut steps = Vec::new();
    let mut jumps = HashSet::new();
    let (mut sector, mut at) = (sector, offset as usize);
    let mut data = read_sector(sector as u16)?;
    loop {
        if steps.len() >= MAX_STEPS {
            return Err(unsupported("too many commands"));
        }
        let end = data.len().saturating_sub(2);
        let op = if at < end { data[at] } else { return Err(unsupported("no end command in its sector")) };
        let len = match op {
            OP_NOOP | OP_END => 1,
            OP_DELAY | OP_KEY_DOWN | OP_KEY_UP | OP_JUMP => 3,
            _ => return Err(unsupported(&format!("command {op:#04x}"))),
        };
        if at + len > end {
            return Err(unsupported("a command crosses the end of its sector"));
        }
        let args = &data[at + 1..at + len];
        match op {
            OP_END => return Ok(steps),
            OP_NOOP => steps.push(MacroStep::Noop),
            OP_DELAY => steps.push(MacroStep::Delay { ms: u16::from_be_bytes([args[0], args[1]]) }),
            OP_KEY_DOWN => steps.push(MacroStep::KeyDown { modifiers: args[0], key: args[1] }),
            OP_KEY_UP => steps.push(MacroStep::KeyUp { modifiers: args[0], key: args[1] }),
            _ => {
                // [0x60, offset, sector]: the reverse of a button binding.
                let (to_sector, to_offset) = (args[1], args[0]);
                if !jumps.insert((to_sector, to_offset)) {
                    return Err(unsupported("it jumps in a loop"));
                }
                if to_sector != sector {
                    data = read_sector(to_sector as u16)?;
                }
                (sector, at) = (to_sector, to_offset as usize);
                continue;
            }
        }
        at += len;
    }
}

/// Bytes of a macro, ending with the end command. It holds no jumps: it must fit where it is placed.
pub fn encode(steps: &[MacroStep]) -> Vec<u8> {
    let mut out = Vec::with_capacity(steps.len() * 3 + 1);
    for step in steps {
        match *step {
            MacroStep::KeyDown { modifiers, key } => out.extend([OP_KEY_DOWN, modifiers, key]),
            MacroStep::KeyUp { modifiers, key } => out.extend([OP_KEY_UP, modifiers, key]),
            MacroStep::Delay { ms } => {
                let [hi, lo] = ms.to_be_bytes();
                out.extend([OP_DELAY, hi, lo]);
            }
            MacroStep::Noop => out.push(OP_NOOP),
        }
    }
    out.push(OP_END);
    out
}

fn unsupported(why: &str) -> Error {
    Error::Protocol(format!("unsupported onboard macro: {why}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: usize = 256;

    /// A sector with `bytes` at `offset`, the rest erased.
    fn sector_with(offset: usize, bytes: &[u8]) -> Vec<u8> {
        let mut data = vec![0xFF; SIZE];
        data[offset..offset + bytes.len()].copy_from_slice(bytes);
        data
    }

    fn reader(sectors: Vec<(u16, Vec<u8>)>) -> impl FnMut(u16) -> Result<Vec<u8>> {
        move |id| {
            sectors
                .iter()
                .find(|(s, _)| *s == id)
                .map(|(_, d)| d.clone())
                .ok_or_else(|| Error::Invalid(format!("sector {id}")))
        }
    }

    #[test]
    fn encode_then_parse() {
        let steps = vec![
            MacroStep::KeyDown { modifiers: 0x01, key: 0x06 },
            MacroStep::Delay { ms: 300 },
            MacroStep::KeyUp { modifiers: 0x01, key: 0x06 },
            MacroStep::Noop,
        ];
        let bytes = encode(&steps);
        assert_eq!(bytes, [0x43, 0x01, 0x06, 0x40, 0x01, 0x2C, 0x44, 0x01, 0x06, 0x00, 0xFF]);
        assert_eq!(parse(reader(vec![(7, sector_with(0x20, &bytes))]), 7, 0x20).unwrap(), steps);
    }

    #[test]
    fn follows_jumps_to_other_sectors() {
        let first = sector_with(0, &[0x43, 0x00, 0x04, 0x60, 0x10, 0x09]);
        let second = sector_with(0x10, &[0x44, 0x00, 0x04, 0xFF]);
        let steps = parse(reader(vec![(8, first), (9, second)]), 8, 0).unwrap();
        assert_eq!(steps, [MacroStep::KeyDown { modifiers: 0, key: 4 }, MacroStep::KeyUp { modifiers: 0, key: 4 }]);
    }

    #[test]
    fn rejects_what_it_cannot_read() {
        // Unknown command (mouse button down).
        assert!(parse(reader(vec![(6, sector_with(0, &[0x41, 0x00, 0x01, 0xFF]))]), 6, 0).is_err());
        // Jump back to itself.
        assert!(parse(reader(vec![(6, sector_with(0, &[0x60, 0x00, 0x06]))]), 6, 0).is_err());
        // Runs into the CRC without an end command.
        let mut no_end = vec![0x00; SIZE];
        no_end[SIZE - 2..].copy_from_slice(&[0xFF, 0xFF]);
        assert!(parse(reader(vec![(6, no_end)]), 6, 0).is_err());
        // A command cut by the CRC.
        assert!(parse(reader(vec![(6, sector_with(SIZE - 4, &[0x40, 0x00]))]), 6, (SIZE - 4) as u8).is_err());
    }
}
