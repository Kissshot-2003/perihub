//! Test sui settori letti da un G502 HERO reale (firmware U1 27.03).

use perihub_core::Rgb;
use perihub_core::hidpp::profile::{
    ButtonAction, LedEffect, ProfileSettings, SpecialFunction, build_directory, crc_ccitt, crc_is_valid, parse_directory,
    write_crc,
};

const BUTTONS: usize = 11;

fn sector(id: u16) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/g502/sector_{id:04x}.bin", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).unwrap()
}

#[test]
fn crc_matches_device() {
    assert_eq!(crc_ccitt(&sector(0)[..254]), 0x5d20);
    assert_eq!(crc_ccitt(&sector(1)[..254]), 0x329a);
    for id in 0..=5 {
        assert!(crc_is_valid(&sector(id)), "settore {id}");
    }
    // I settori di fabbrica non hanno CRC.
    assert!(!crc_is_valid(&sector(0x0101)));
}

#[test]
fn parse_then_apply_is_byte_identical() {
    for id in 1..=5 {
        let original = sector(id);
        let settings = ProfileSettings::parse(&original, BUTTONS).unwrap();
        let mut copy = original.clone();
        settings.apply(&mut copy).unwrap();
        assert_eq!(copy, original, "profilo {id}");
    }
}

#[test]
fn profile_one_contents() {
    let p = ProfileSettings::parse(&sector(1), BUTTONS).unwrap();
    assert_eq!(p.report_rate_ms, 1);
    assert_eq!(p.dpi_levels, vec![1200, 2400, 3200, 6400]);
    assert_eq!(p.default_dpi_index, 1);
    assert_eq!(p.shift_dpi_index, 0);
    assert_eq!(p.buttons[0], ButtonAction::Mouse { button: 1 });
    assert_eq!(p.buttons[4], ButtonAction::Mouse { button: 5 });
    assert_eq!(p.buttons[5], ButtonAction::Special { function: SpecialFunction::DpiShift });
    assert_eq!(p.buttons[6], ButtonAction::Special { function: SpecialFunction::DpiDown });
    assert_eq!(p.buttons[7], ButtonAction::Special { function: SpecialFunction::DpiUp });
    assert_eq!(p.buttons[8], ButtonAction::Special { function: SpecialFunction::ProfileCycle });
    assert_eq!(p.buttons[9], ButtonAction::Special { function: SpecialFunction::TiltRight });
    assert_eq!(p.buttons[10], ButtonAction::Special { function: SpecialFunction::TiltLeft });
    assert!(p.gshift_buttons.iter().all(|b| *b == ButtonAction::Disabled));
    assert_eq!(p.leds[0], LedEffect::Cycle { period_ms: 8000, brightness: 100 });
}

#[test]
fn profile_two_breathing_led() {
    let p = ProfileSettings::parse(&sector(2), BUTTONS).unwrap();
    assert_eq!(
        p.leds[1],
        LedEffect::Breathing { color: Rgb::new(0x00, 0xdc, 0xff), period_ms: 8000, brightness: 100 }
    );
}

#[test]
fn edits_touch_only_their_fields() {
    let original = sector(4);
    let mut settings = ProfileSettings::parse(&original, BUTTONS).unwrap();
    settings.dpi_levels = vec![800, 1600];
    settings.buttons[3] = ButtonAction::Key { modifiers: 0x01, key: 0x06 };
    settings.leds[0] = LedEffect::Fixed { color: Rgb::new(255, 0, 0) };
    let mut edited = original.clone();
    settings.apply(&mut edited).unwrap();
    assert!(crc_is_valid(&edited));
    let changed: Vec<usize> = (0..254).filter(|&i| edited[i] != original[i]).collect();
    let allowed = |i: usize| (3..13).contains(&i) || (44..48).contains(&i) || (208..219).contains(&i);
    assert!(changed.iter().all(|&i| allowed(i)), "byte modificati fuori dai campi: {changed:?}");
    assert_eq!(ProfileSettings::parse(&edited, BUTTONS).unwrap(), settings);
}

#[test]
fn directory_round_trip() {
    let dir = sector(0);
    let enabled = parse_directory(&dir, 5);
    assert_eq!(enabled, vec![true; 5]);
    assert_eq!(build_directory(&enabled, 256), dir);
}

#[test]
fn factory_sector_equals_user_sector_after_crc() {
    let mut rom = sector(0x0101);
    write_crc(&mut rom);
    assert_eq!(rom, sector(1));
}
