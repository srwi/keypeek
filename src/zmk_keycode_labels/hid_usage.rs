use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use zmk_studio_api::HidUsage;

use super::keycode_label::keycode_to_layout_key;

pub fn hid_usage_to_layout_key(usage: HidUsage) -> LayoutKey {
    let mods = usage.modifiers();
    if mods == 0 {
        if let Some(keycode) = usage.known_keycode() {
            return keycode_to_layout_key(&keycode);
        }

        return LayoutKey {
            tap: Label::new(format!("0x{:08X}", usage.to_hid_usage())),
            ..Default::default()
        };
    }

    if let Some(named_key) = usage.known_keycode() {
        return keycode_to_layout_key(&named_key);
    }

    let base = usage.base();
    let mut base_key = if let Some(base_keycode) = base.known_keycode() {
        keycode_to_layout_key(&base_keycode)
    } else {
        LayoutKey {
            tap: Label::new(format!("0x{:08X}", base.to_hid_usage())),
            ..Default::default()
        }
    };

    // For pure shift modifiers (Left Shift: 0x02, Right Shift: 0x20)
    if mods == 0x02 || mods == 0x20 {
        if let Some(pos) = base_key.tap.full.find('\n') {
            base_key.tap = Label::new(base_key.tap.full[..pos].to_string());
            return base_key;
        }
    }

    let mut rendered = base_key.tap.full;
    for modifier in usage.modifier_labels().iter().rev() {
        let sym = match modifier.as_ref() {
            "LC" | "RC" => "\u{2388}",
            "LS" | "RS" => egui_phosphor::regular::ARROW_FAT_UP,
            "LA" | "RA" => egui_phosphor::regular::OPTION,
            "LG" | "RG" => egui_phosphor::fill::DIAMOND,
            _ => modifier,
        };
        rendered = format!("{}{}", sym, rendered);
    }

    LayoutKey {
        tap: Label::new(rendered),
        kind: KeycodeKind::Modifier,
        ..Default::default()
    }
}
