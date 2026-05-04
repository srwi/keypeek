use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use crate::qmk_keycode_labels::basic::get_basic_layout_key;
use crate::qmk_keycode_labels::constants::*;

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
            let mod_str = mod_mask_to_string(input_modifiers);

            if !mod_str.is_empty() {
                return Some(LayoutKey {
                    tap: Label::new(format!("{}({})", mod_str, keycode_str)),
                    kind: KeycodeKind::Modifier,
                    ..Default::default()
                });
            }

            None
        }
        input_bytes if QK_MOD_TAP.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_MOD_TAP.start);

            let mod_value = ((remainder >> 8) & 0xFF) << 8;
            let mod_str = mod_mask_to_string(mod_value);

            let keycode = (remainder & 0xFF) as u8;
            let keycode_str = get_basic_layout_key(keycode as u16)
                .map(|k| k.tap.short.unwrap_or(k.tap.full))
                .unwrap_or_else(|| format!("0x{:02X}", keycode));

            Some(LayoutKey {
                tap: Label::with_short(
                    format!("{}\n{}", mod_str, keycode_str),
                    format!("{}/{}", mod_str, keycode_str),
                ),
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        input_bytes if QK_LAYER_MOD.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_LAYER_MOD.start);
            let mask = 0x1f;
            let shift = 5;

            let layer = remainder >> shift;

            let mod_value = (remainder & mask) << 8;
            let mod_str = mod_mask_to_string(mod_value);

            Some(LayoutKey {
                tap: Label::with_short(
                    format!("LM{}\n{}", layer, mod_str),
                    format!("LM{}\n{}", layer, mod_str),
                ),
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
                ..Default::default()
            })
        }
        input_bytes if QK_ONE_SHOT_MOD.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_ONE_SHOT_MOD.start);

            let mod_str = mod_mask_to_string(remainder << 8);

            Some(LayoutKey {
                tap: Label::with_short(
                    format!("OSM\n{}", mod_str),
                    format!("OSM\n{}", mod_str),
                ),
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        input_bytes if QK_LAYER_TAP.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_LAYER_TAP.start);

            let layer = remainder >> 8;

            let keycode = (remainder & 0xFF) as u8;
            let keycode_str = get_basic_layout_key(keycode as u16)
                .map(|k| k.tap.short.unwrap_or(k.tap.full))
                .unwrap_or_else(|| format!("0x{:02X}", keycode));

            Some(LayoutKey {
                tap: Label::with_short(
                    format!("L{}\n{}", layer, keycode_str),
                    format!("L{}/{}", layer, keycode_str),
                ),
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
                ..Default::default()
            })
        }
        _ => None,
    }
}

fn mod_mask_to_string(input_modifiers: u16) -> String {
    if let Some((name, _)) = MODIFIER_KEY_TO_VALUE
        .iter()
        .find(|(_, v)| *v == input_modifiers)
    {
        return name.to_string();
    }

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
        .filter_map(|(name, modifiers)| {
            if *modifiers == (input_modifiers & *modifiers) && *modifiers != 0 
                && modifiers.count_ones() == 1 {
                Some(*name)
            } else {
                None
            }
        })
        .collect();
    enabled.join("+")
}