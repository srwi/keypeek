use crate::layout_key::modifier_symbols;
use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use crate::qmk_keycode_labels::basic::get_basic_layout_key;
use crate::qmk_keycode_labels::constants::*;

pub fn get_advanced_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    match keycode_bytes {
        input_bytes if QK_MODS.contains(&input_bytes) => {
            let keycode = input_bytes & 0xff;
            let inner_key = get_basic_layout_key(keycode);

            let input_modifiers = input_bytes & 0x1f00;

            // A lone shift over a key with a shifted legend just yields that character
            // (S(KC_1) == "!"), so render it as a plain key. Compare only the low nibble.
            if (input_modifiers >> 8) & 0x0f == MOD_LSFT {
                if let Some(shifted) = inner_key.as_ref().and_then(|k| k.shifted.clone()) {
                    return Some(LayoutKey {
                        tap: Label::new(shifted),
                        ..Default::default()
                    });
                }
            }

            // Otherwise show the modified key in `tap` and the applied modifiers as
            // glyphs in the function strip (e.g. "C" + "⎈" for LCTL(KC_C)).
            let (tap, symbol) = match inner_key {
                Some(k) => (k.tap, k.symbol),
                None => (Label::new(format!("0x{:02X}", keycode)), None),
            };
            Some(LayoutKey {
                tap,
                function: Some(mod_value_to_label(input_modifiers >> 8)),
                symbol,
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        input_bytes if QK_MOD_TAP.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_MOD_TAP.start);

            let mod_value = (remainder >> 8) & 0x1F;
            let mod_label = mod_value_to_label(mod_value);

            let keycode = (remainder & 0xFF) as u8;
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();

            Some(LayoutKey {
                tap: tap_key.tap,
                function: Some(mod_label.prefixed("MT: ")),
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
            let mod_label = mod_value_to_label(mod_value);

            Some(LayoutKey {
                tap: Label::new(format!("L{}", layer)),
                function: Some(mod_label),
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
                ..Default::default()
            })
        }
        input_bytes if QK_ONE_SHOT_MOD.contains(&input_bytes) => {
            let remainder = input_bytes & !(QK_ONE_SHOT_MOD.start);

            let mod_label = mod_value_to_label(remainder);

            Some(LayoutKey {
                tap: mod_label,
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

fn mod_value_to_label(mod_mask: u16) -> Label {
    // Left/right share the low-nibble encoding, so only bits 0-3 matter.
    modifier_symbols::glyphs(
        mod_mask & MOD_LCTL != 0,
        mod_mask & MOD_LSFT != 0,
        mod_mask & MOD_LALT != 0,
        mod_mask & MOD_LGUI != 0,
    )
}
