//! Tests on a macro read from a real G502 HERO (the same mouse as `fixtures/g502`): profile 5 with
//! button 7 ("DPI –") bound to a macro in sector 6. The macro was written with a test tool, then
//! checked by pressing the button: it types "heyclaude".

use perihub_core::hidpp::macros::{self, Macro, MacroKind, MacroStep, Packer};
use perihub_core::hidpp::profile::{ButtonAction, ProfileSettings, SpecialFunction, crc_is_valid};

const BUTTONS: usize = 11;

fn sector(id: u16) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/g502-macro/sector_{id:04x}.bin", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).unwrap()
}

/// Each letter: key down, `hold_ms`, key up, then `gap_ms` before the next one.
fn typed(text: &str, hold_ms: u16, gap_ms: u16) -> Macro {
    let mut steps = Vec::new();
    for (i, c) in text.bytes().enumerate() {
        let key = c - b'a' + 0x04;
        steps.push(MacroStep::KeyDown { modifiers: 0, key });
        steps.push(MacroStep::Delay { ms: hold_ms });
        steps.push(MacroStep::KeyUp { modifiers: 0, key });
        if i + 1 < text.len() {
            steps.push(MacroStep::Delay { ms: gap_ms });
        }
    }
    Macro { kind: MacroKind::Once, press: steps, release: Vec::new() }
}

#[test]
fn button_points_to_the_macro() {
    let p = ProfileSettings::parse(&sector(5), BUTTONS).unwrap();
    assert_eq!(p.buttons[6], ButtonAction::Macro { sector: 6, offset: 0, content: None });
    assert_eq!(p.buttons[7], ButtonAction::Special { function: SpecialFunction::DpiUp });
    assert_eq!(p.macro_addresses(), vec![(6u8, 0u16)]);
}

#[test]
fn profile_with_macro_round_trips() {
    let original = sector(5);
    let settings = ProfileSettings::parse(&original, BUTTONS).unwrap();
    let mut copy = original.clone();
    settings.apply(&mut copy).unwrap();
    assert_eq!(copy, original);
}

#[test]
fn macro_types_heyclaude() {
    let content = macros::parse(|id| Ok(sector(id)), 6, 0).unwrap();
    assert_eq!(content, typed("heyclaude", 10, 10));
    assert_eq!(macros::sectors_used(|id| Ok(sector(id)), 6, 0).unwrap(), [6]);
}

#[test]
fn encoding_matches_the_mouse() {
    let data = sector(6);
    let mut packer = Packer::new([6], 256);
    assert_eq!(packer.place(&typed("heyclaude", 10, 10)).unwrap(), (6, 0));
    let mut written = packer.finish().remove(0).1;
    perihub_core::hidpp::profile::write_crc(&mut written);
    assert_eq!(written, data);
    assert!(crc_is_valid(&data));
}

#[test]
fn content_survives_serialization() {
    let action = ButtonAction::Macro { sector: 6, offset: 0, content: Some(typed("hi", 10, 10)) };
    let json = serde_json::to_value(&action).unwrap();
    assert_eq!(json["type"], "macro");
    assert_eq!(json["content"]["kind"], "once");
    assert_eq!(json["content"]["press"][0], serde_json::json!({ "type": "key_down", "modifiers": 0, "key": 0x0b }));
    assert_eq!(serde_json::from_value::<ButtonAction>(json).unwrap(), action);
    // The interface may send the binding back without its content.
    let bare: ButtonAction = serde_json::from_str(r#"{"type":"macro","sector":6,"offset":0}"#).unwrap();
    assert_eq!(bare.encode().unwrap(), [0x00, 0x06, 0x00, 0x00]);
}
