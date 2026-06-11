use crate::layout_key::modifier_symbols::*;
use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use crate::qmk_keycode_labels::basic::get_basic_layout_key;
use crate::qmk_keycode_labels::constants::*;

pub fn get_advanced_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    match keycode_bytes {
        input_bytes if QK_MODS.contains(&input_bytes) => {
            let keycode = input_bytes & 0xff;
            let inner_key = get_basic_layout_key(keycode);
            let keycode_str = inner_key
                .as_ref()
                .map(|k| k.tap.full.clone())
                .unwrap_or_else(|| format!("0x{:02X}", keycode));

            let input_modifiers = input_bytes & 0x1f00;

            // A lone shift over a key that has a shifted legend just produces that
            // shifted character (e.g. S(KC_1) == "!"), so render it as a plain key.
            if input_modifiers == QK_LSFT || input_modifiers == QK_RSFT {
                if let Some(shifted) = inner_key.as_ref().and_then(|k| k.shifted.clone()) {
                    return Some(LayoutKey {
                        tap: Label::new(shifted),
                        ..Default::default()
                    });
                }
            }

            // Try to find exact matches first
            if let Some((name, _)) = MODIFIER_KEY_TO_VALUE
                .iter()
                .find(|(_, v)| *v == input_modifiers)
            {
                return Some(LayoutKey {
                    tap: Label::new(format!("{}({})", name, keycode_str)),
                    kind: KeycodeKind::Modifier,
                    ..Default::default()
                });
            }

            // Left and right side modifiers are mutually exclusive. Therefore a single boolean
            // is used to indicate which side to use.
            let is_right_side_mods = (input_modifiers & QK_RMODS_MIN) != 0;
            let enabled: Vec<&str> = MODIFIER_KEY_TO_VALUE
                .iter()
                .filter(|(_, modifiers)| {
                    if is_right_side_mods {
                        *modifiers >= QK_RMODS_MIN
                    } else {
                        *modifiers < QK_RMODS_MIN
                    }
                })
                .filter_map(|(modifiers_name, modifiers)| {
                    if (input_modifiers & *modifiers) == *modifiers {
                        Some(*modifiers_name)
                    } else {
                        None
                    }
                })
                .collect();

            if !enabled.is_empty() {
                // Build nested parentheses style, e.g. LCTL(LALT(A))
                let mut nested_mods = String::new();
                for (i, part) in enabled.iter().enumerate() {
                    if i > 0 {
                        nested_mods.push('(');
                    }
                    nested_mods.push_str(part);
                }
                if !nested_mods.is_empty() {
                    nested_mods.push('(');
                }
                nested_mods.push_str(&keycode_str);
                for _ in 0..enabled.len() {
                    nested_mods.push(')');
                }

                return Some(LayoutKey {
                    tap: Label::new(nested_mods),
                    kind: KeycodeKind::Modifier,
                    ..Default::default()
                });
            }

            None
        }
        input_bytes if QK_MOD_TAP.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_MOD_TAP.start);

            let mod_value = (remainder >> 8) & 0x1F;
            let mod_str = mod_value_to_string(mod_value);

            let keycode = (remainder & 0xFF) as u8;
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();

            Some(LayoutKey {
                tap: tap_key.tap,
                function: Some(Label::new(mod_str)),
                shifted: tap_key.shifted,
                symbol: tap_key.symbol,
                kind: KeycodeKind::Basic,
                ..Default::default()
            })
        }
        input_bytes if QK_LAYER_MOD.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_LAYER_MOD.start);
            let mask = 0x1f;
            let shift = 5;

            let layer = remainder >> shift;

            let mod_value = remainder & mask;
            let mod_str = mod_value_to_string(mod_value);

            Some(LayoutKey {
                tap: Label::new(mod_str),
                function: Some(Label::new(format!("L{}", layer))),
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
                ..Default::default()
            })
        }
        input_bytes if QK_ONE_SHOT_MOD.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_ONE_SHOT_MOD.start);

            let mod_str = mod_value_to_string(remainder);

            Some(LayoutKey {
                tap: Label::new(mod_str),
                function: Some(Label::new("OSM")),
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        input_bytes if QK_LAYER_TAP.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_LAYER_TAP.start);

            let layer = remainder >> 8;

            let keycode = (remainder & 0xFF) as u8;
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();

            Some(LayoutKey {
                tap: tap_key.tap,
                function: Some(Label::new(format!("L{}", layer))),
                shifted: tap_key.shifted,
                symbol: tap_key.symbol,
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
            })
        }
        _ => None,
    }
}

fn mod_value_to_string(mod_mask: u16) -> String {
    // Left and right share the same low-nibble encoding and render to the same
    // symbol, so only bits 0-3 need to be checked.
    let mut mods = Vec::new();
    if mod_mask & MOD_LCTL != 0 {
        mods.push(MOD_SYMBOL_CTRL);
    }
    if mod_mask & MOD_LSFT != 0 {
        mods.push(MOD_SYMBOL_SHIFT);
    }
    if mod_mask & MOD_LALT != 0 {
        mods.push(MOD_SYMBOL_ALT);
    }
    if mod_mask & MOD_LGUI != 0 {
        mods.push(MOD_SYMBOL_GUI);
    }

    if mods.is_empty() {
        "None".to_string()
    } else {
        mods.join("")
    }
}
