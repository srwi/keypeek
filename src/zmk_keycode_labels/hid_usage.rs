use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use zmk_studio_api::{HidUsage, MOD_LSFT, MOD_RSFT};

use super::keycode_label::keycode_to_layout_key;

pub fn hid_usage_to_layout_key(usage: HidUsage) -> LayoutKey {
    if usage.modifiers() == 0 {
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

    // A lone shift over a key with a shifted legend just yields that character
    // (LS(N1) == "!"), so render it as a plain key.
    let mods = usage.modifiers();
    if mods & !(MOD_LSFT | MOD_RSFT) == 0 {
        if let Some(base_keycode) = base.known_keycode() {
            if let Some(shifted) = keycode_to_layout_key(&base_keycode).shifted {
                return LayoutKey {
                    tap: Label::new(shifted),
                    ..Default::default()
                };
            }
        }
    }

    let base_label = if let Some(base_keycode) = base.known_keycode() {
        let base_key = keycode_to_layout_key(&base_keycode);
        if let Some(symbol) = base_key.symbol {
            symbol
        } else {
            base_key.tap.full
        }
    } else {
        format!("0x{:08X}", base.to_hid_usage())
    };

    let mut rendered = base_label;
    for modifier in usage.modifier_labels().iter().rev() {
        rendered = format!("{modifier}({rendered})");
    }

    LayoutKey {
        tap: Label::new(rendered),
        kind: KeycodeKind::Modifier,
        ..Default::default()
    }
}
