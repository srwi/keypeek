use crate::layout_key::modifier_symbols;
use crate::layout_key::{behavior_names, BorderStyle, KeycodeKind, Label, LayoutKey};
use crate::qmk_keycode_labels::basic::get_basic_layout_key;
use crate::qmk_keycode_labels::constants::*;

pub fn get_advanced_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    match keycode_bytes {
        input_bytes if QK_MODS.contains(&input_bytes) => {
            let keycode = input_bytes & 0xff;
            let mods = (input_bytes & 0x1f00) >> 8;

            // Shift and RAlt are the only mods that change the output
            // character (RAlt as the layout's Level-3 shift, where defined).
            // Resolve that directly and show it flat, with no badge.
            // Everything else (Ctrl/Gui) produces no text; fall through to
            // base key + mod badge. On macOS plain Alt is Option, a Level-3
            // shift in its own right, so it counts as one too.
            let plain = mods & !MOD_RIGHT_FLAG;
            let alt_is_level3 = cfg!(target_os = "macos") || mods & MOD_RIGHT_FLAG != 0;
            let text_modifier = if plain == MOD_LSFT {
                Some(crate::os_layout::Modifier::Shift)
            } else if plain == MOD_LALT && alt_is_level3 {
                Some(crate::os_layout::Modifier::RAlt)
            } else if plain == (MOD_LSFT | MOD_LALT) && alt_is_level3 {
                Some(crate::os_layout::Modifier::ShiftRAlt)
            } else {
                None
            };
            if let Some(m) = text_modifier {
                if let Some(text) = crate::os_layout::resolve(keycode, m) {
                    return Some(LayoutKey {
                        tap: Label::new(text),
                        ..Default::default()
                    });
                }
            }

            let (tap, symbol) = match get_basic_layout_key(keycode) {
                Some(k) => (k.tap, k.symbol),
                None => (Label::new(format!("0x{:02X}", keycode)), None),
            };
            Some(LayoutKey {
                tap,
                argument: Some(mod_value_to_label(mods)),
                symbol,
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        input_bytes if QK_MOD_TAP.contains(&input_bytes) => {
            let remainder = input_bytes - QK_MOD_TAP.start;

            let mod_value = (remainder >> 8) & 0x1F;
            let mod_label = mod_value_to_label(mod_value);

            let keycode = (remainder & 0xFF) as u8;
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();

            Some(LayoutKey {
                tap: tap_key.tap,
                behavior: Some(behavior_names::MOD_TAP.label()),
                argument: Some(mod_label),
                shifted: tap_key.shifted,
                ralt: tap_key.ralt,
                ralt_shifted: tap_key.ralt_shifted,
                mod_mask: Some(to_held_mod_mask(mod_value)),
                symbol: tap_key.symbol,
                kind: KeycodeKind::Basic,
                ..Default::default()
            })
        }
        input_bytes if QK_LAYER_MOD.contains(&input_bytes) => {
            let remainder = input_bytes - QK_LAYER_MOD.start;
            let layer = remainder >> 5;
            let mod_mask = remainder & 0x1F;

            Some(LayoutKey {
                tap: Label::new(format!("L{}", layer)),
                argument: (mod_mask != 0).then(|| mod_value_to_label(mod_mask)),
                mod_mask: Some(to_held_mod_mask(mod_mask)),
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
                border: BorderStyle::None,
                ..Default::default()
            })
        }
        input_bytes if QK_ONE_SHOT_MOD.contains(&input_bytes) => {
            let remainder = input_bytes - QK_ONE_SHOT_MOD.start;

            let mod_label = mod_value_to_label(remainder);

            Some(LayoutKey {
                tap: mod_label,
                behavior: Some(behavior_names::ONE_SHOT_MOD.label()),
                mod_mask: Some(to_held_mod_mask(remainder)),
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        input_bytes if QK_LAYER_TAP.contains(&input_bytes) => {
            let remainder = input_bytes - QK_LAYER_TAP.start;

            let layer = remainder >> 8;

            let keycode = (remainder & 0xFF) as u8;
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();

            Some(LayoutKey {
                tap: tap_key.tap,
                shifted: tap_key.shifted,
                ralt: tap_key.ralt,
                ralt_shifted: tap_key.ralt_shifted,
                symbol: tap_key.symbol,
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer as u8),
                border: BorderStyle::None,
                ..Default::default()
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

#[cfg(test)]
mod tests {
    use super::get_advanced_layout_key;

    // A Shift-wrapped key (LSFT(KC_0)) shows the flat result character, not
    // a Base+Shifted stack and no badge: the modifier's effect IS the output,
    // so there is nothing left to badge. The expected char is German-specific
    // (AZERTY shifts its digit row differently), so it needs a live German
    // session.
    #[test]
    #[ignore]
    fn shift_wrapped_key_shows_flat_result_no_badge() {
        let key = get_advanced_layout_key(0x0227).unwrap();
        assert_eq!(key.tap.full, "=");
        assert!(key.shifted.is_none());
        assert!(key.argument.is_none());
    }
}
