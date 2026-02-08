use crate::keycode_labels::basic::get_basic_layout_key;
use crate::keycode_labels::constants::*;
use crate::layout_key::{KeycodeKind, Label, LayoutKey};

pub fn get_advanced_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    match keycode_bytes {
        input_bytes if QK_MODS.contains(&input_bytes) => {
            let keycode = input_bytes & 0xff;
            let keycode_str = get_basic_layout_key(keycode)
                .map(|k| k.tap.full)
                .unwrap_or_else(|| format!("0x{:02X}", keycode));

            let input_modifiers = input_bytes & 0x1f00;

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
            let keycode_str = get_basic_layout_key(keycode as u16)
                .map(|k| k.tap.full)
                .unwrap_or_else(|| format!("0x{:02X}", keycode));

            Some(LayoutKey {
                tap: Label::new(format!("MT({},{})", mod_str, keycode_str)),
                kind: KeycodeKind::Modifier,
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
                tap: Label::new(format!("LM({},{})", layer, mod_str)),
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
                ..Default::default()
            })
        }
        input_bytes if QK_ONE_SHOT_MOD.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_ONE_SHOT_MOD.start);

            let mod_str = mod_value_to_string(remainder);

            Some(LayoutKey {
                tap: Label::new(format!("OSM({})", mod_str)),
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        input_bytes if QK_LAYER_TAP.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_LAYER_TAP.start);

            let layer = remainder >> 8;

            let keycode = (remainder & 0xFF) as u8;
            let keycode_str = get_basic_layout_key(keycode as u16)
                .map(|k| k.tap.full)
                .unwrap_or_else(|| format!("0x{:02X}", keycode));

            Some(LayoutKey {
                tap: Label::new(format!("LT({},{})", layer, keycode_str)),
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
                ..Default::default()
            })
        }
        _ => None,
    }
}

fn mod_value_to_string(mod_mask: u16) -> String {
    let mut mods = Vec::new();
    if mod_mask & MOD_LCTL != 0 {
        mods.push("MOD_LCTL");
    }
    if mod_mask & MOD_LSFT != 0 {
        mods.push("MOD_LSFT");
    }
    if mod_mask & MOD_LALT != 0 {
        mods.push("MOD_LALT");
    }
    if mod_mask & MOD_LGUI != 0 {
        mods.push("MOD_LGUI");
    }
    if mod_mask & MOD_RCTL != 0 {
        mods.push("MOD_RCTL");
    }
    if mod_mask & MOD_RSFT != 0 {
        mods.push("MOD_RSFT");
    }
    if mod_mask & MOD_RALT != 0 {
        mods.push("MOD_RALT");
    }
    if mod_mask & MOD_RGUI != 0 {
        mods.push("MOD_RGUI");
    }

    if mods.is_empty() {
        "None".to_string()
    } else {
        mods.join(" | ")
    }
}
