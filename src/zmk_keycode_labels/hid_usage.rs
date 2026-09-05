use crate::hid_labels::{mod_combo_key, Modifiers};
use crate::layout_key::{Label, LayoutKey};
use zmk_studio_api::HidUsage;

pub fn hid_usage_to_layout_key(usage: HidUsage) -> LayoutKey {
    let mods = Modifiers::from_zmk_mask(usage.modifiers());
    if mods.is_empty() {
        if let Some(key) = crate::hid_labels::hid_usage_to_layout_key(usage.page(), usage.id()) {
            return key;
        }

        if let Some(keycode) = usage.known_keycode() {
            return LayoutKey {
                tap: Label::new(keycode.to_name()),
                ..Default::default()
            };
        }

        return LayoutKey {
            tap: Label::new(format!("0x{:08X}", usage.to_hid_usage())),
            ..Default::default()
        };
    }

    let base = crate::hid_labels::hid_usage_to_layout_key(usage.page(), usage.id()).or_else(|| {
        usage.base().known_keycode().map(|k| LayoutKey {
            tap: Label::new(k.to_name()),
            ..Default::default()
        })
    });
    mod_combo_key(usage.page(), usage.id(), mods, base)
}
