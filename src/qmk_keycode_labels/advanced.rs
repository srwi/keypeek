use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use crate::qmk_keycode_labels::basic::get_basic_layout_key;
use crate::qmk_keycode_labels::constants::*;

pub fn get_advanced_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    match keycode_bytes {
        input_bytes if QK_MODS.contains(&input_bytes) => {
            let keycode = input_bytes & 0xff;
            let tap_key = get_basic_layout_key(keycode).unwrap_or_default();
            let keycode_str = tap_key.tap.full.clone();

            let input_modifiers = input_bytes & 0x1f00;

            // For pure shift modifiers, try to show the shifted character directly
            if input_modifiers == QK_LSFT || input_modifiers == QK_RSFT {
                if let Some(pos) = keycode_str.find('\n') {
                    return Some(LayoutKey {
                        tap: Label::new(keycode_str[..pos].to_string()),
                        ..tap_key
                    });
                }
            }

            // Try to find exact matches first
            if let Some((name, _)) = MODIFIER_KEY_TO_VALUE
                .iter()
                .find(|(_, v)| *v == input_modifiers)
            {
                let mut display_name = name.to_string();
                // Use our new symbols if it's a simple modifier
                if input_modifiers == QK_LCTL || input_modifiers == QK_RCTL {
                    display_name = "\u{2388}".to_string();
                } else if input_modifiers == QK_LALT || input_modifiers == QK_RALT {
                    display_name = egui_phosphor::regular::OPTION.to_string();
                } else if input_modifiers == QK_LGUI || input_modifiers == QK_RGUI {
                    display_name = egui_phosphor::fill::DIAMOND.to_string();
                } else if input_modifiers == QK_LSFT || input_modifiers == QK_RSFT {
                    display_name = egui_phosphor::regular::ARROW_FAT_UP.to_string();
                }

                return Some(LayoutKey {
                    tap: Label::new(format!("{}{}", display_name, keycode_str)),
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
                // Build compact symbol style, e.g. ⎈⌥A
                let mut compact_label = String::new();
                for part in enabled.iter() {
                    let sym = match *part {
                        "LCTL" | "RCTL" | "C" => "\u{2388}",
                        "LSFT" | "RSFT" | "S" => egui_phosphor::regular::ARROW_FAT_UP,
                        "LALT" | "RALT" | "A" | "ALGR" => egui_phosphor::regular::OPTION,
                        "LGUI" | "RGUI" | "G" | "LCMD" | "LWIN" | "RCMD" | "RWIN" => {
                            egui_phosphor::fill::DIAMOND
                        }
                        _ => *part,
                    };
                    compact_label.push_str(sym);
                }
                compact_label.push_str(&keycode_str);

                return Some(LayoutKey {
                    tap: Label::new(compact_label),
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
                hold: Some(Label::new(mod_str)),
                symbol: tap_key.symbol,
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
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();

            Some(LayoutKey {
                tap: tap_key.tap,
                hold: Some(Label::new(format!("L{}", layer))),
                symbol: tap_key.symbol,
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
        mods.push("\u{2388}");
    }
    if mod_mask & MOD_LSFT != 0 {
        mods.push(egui_phosphor::regular::ARROW_FAT_UP);
    }
    if mod_mask & MOD_LALT != 0 {
        mods.push(egui_phosphor::regular::OPTION);
    }
    if mod_mask & MOD_LGUI != 0 {
        mods.push(egui_phosphor::fill::DIAMOND);
    }
    if mod_mask & MOD_RCTL != 0 {
        mods.push("\u{2388}");
    }
    if mod_mask & MOD_RSFT != 0 {
        mods.push(egui_phosphor::regular::ARROW_FAT_UP);
    }
    if mod_mask & MOD_RALT != 0 {
        mods.push(egui_phosphor::regular::OPTION);
    }
    if mod_mask & MOD_RGUI != 0 {
        mods.push(egui_phosphor::fill::DIAMOND);
    }

    if mods.is_empty() {
        "None".to_string()
    } else {
        mods.into_iter().map(|s| s.to_string()).collect::<Vec<_>>().join("")
    }
}
