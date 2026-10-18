//! The Linux udev rule must grant access to exactly the mice PeriHub recognizes.

use perihub_core::hidpp::models::MODELS;

const RULE: &str = include_str!("../../../packaging/linux/70-perihub.rules");

#[test]
fn udev_rule_matches_mouse_list() {
    let mut in_rule: Vec<String> = RULE
        .lines()
        .filter(|line| line.contains(r#"ATTRS{idVendor}=="046d""#))
        .map(|line| line.split(r#"ATTRS{idProduct}==""#).nth(1).unwrap()[..4].to_string())
        .collect();
    let mut in_list: Vec<String> = MODELS.iter().map(|m| format!("{:04x}", m.product_id)).collect();
    in_rule.sort();
    in_list.sort();
    assert_eq!(in_rule, in_list);
}
