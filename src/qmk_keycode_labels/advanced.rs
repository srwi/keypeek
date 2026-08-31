use crate::layout_key::modifier_symbols;
use crate::layout_key::{behavior_names, BorderStyle, KeycodeKind, Label, LayoutKey};
use crate::qmk_keycode_labels::basic::get_basic_layout_key;
use crate::qmk_keycode_labels::constants::*;

use qmk_via_api::{QmkKeycode, QmkModMask};

pub fn get_advanced_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    match QmkKeycode::from_u16(keycode_bytes) {
        QmkKeycode::ModCombo { mods, keycode } => {
            // Shift and RAlt are the only mods that change the output
            // character (RAlt as the layout's Level-3 shift, where defined).
            // Resolve that directly and show it flat, with no badge.
            // Everything else (Ctrl/Gui) produces no text; fall through to
            // base key + mod badge. On macOS plain Alt is Option, a Level-3
            // shift in its own right, so it counts as one too.
            let alt_is_level3 = cfg!(target_os = "macos") || mods.is_right();
            let text_modifier = if !mods.has_ctrl() && !mods.has_gui() {
                if mods.has_shift() && !mods.has_alt() {
                    Some(crate::os_layout::Modifier::Shift)
                } else if mods.has_alt() && !mods.has_shift() && alt_is_level3 {
                    Some(crate::os_layout::Modifier::RAlt)
                } else if mods.has_shift() && mods.has_alt() && alt_is_level3 {
                    Some(crate::os_layout::Modifier::ShiftRAlt)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(m) = text_modifier {
                if let Some(text) = crate::os_layout::resolve(keycode as u16, m) {
                    return Some(LayoutKey {
                        tap: Label::new(text),
                        ..Default::default()
                    });
                }
            }

            let (tap, symbol) = match get_basic_layout_key(keycode as u16) {
                Some(k) => (k.tap, k.symbol),
                None => (Label::new(format!("0x{:02X}", keycode)), None),
            };
            Some(LayoutKey {
                tap,
                argument: Some(mod_mask_to_label(mods)),
                symbol,
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        QmkKeycode::ModTap { mods, keycode } => {
            let mod_label = mod_mask_to_label(mods);
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();

            Some(LayoutKey {
                tap: tap_key.tap,
                behavior: Some(behavior_names::MOD_TAP.label()),
                argument: Some(mod_label),
                shifted: tap_key.shifted,
                ralt: tap_key.ralt,
                ralt_shifted: tap_key.ralt_shifted,
                mod_mask: Some(to_held_mod_mask(mods)),
                symbol: tap_key.symbol,
                kind: KeycodeKind::Basic,
                ..Default::default()
            })
        }
        QmkKeycode::LayerMod { layer, mods } => Some(LayoutKey {
            tap: Label::new(format!("L{}", layer)),
            argument: (!mods.is_empty()).then(|| mod_mask_to_label(mods)),
            mod_mask: Some(to_held_mod_mask(mods)),
            kind: KeycodeKind::Modifier,
            layer_ref: Some(layer),
            border: BorderStyle::None,
            ..Default::default()
        }),
        QmkKeycode::OneShotMod(mods) => {
            let mod_label = mod_mask_to_label(mods);

            Some(LayoutKey {
                tap: mod_label,
                behavior: Some(behavior_names::ONE_SHOT_MOD.label()),
                mod_mask: Some(to_held_mod_mask(mods)),
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        QmkKeycode::LayerTap { layer, keycode } => {
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();

            Some(LayoutKey {
                tap: tap_key.tap,
                shifted: tap_key.shifted,
                ralt: tap_key.ralt,
                ralt_shifted: tap_key.ralt_shifted,
                symbol: tap_key.symbol,
                kind: KeycodeKind::Modifier,
                layer_ref: Some(layer),
                border: BorderStyle::None,
                ..Default::default()
            })
        }
        _ => None,
    }
}

fn mod_mask_to_label(mods: QmkModMask) -> Label {
    modifier_symbols::glyphs(
        mods.has_ctrl(),
        mods.has_shift(),
        mods.has_alt(),
        mods.has_gui(),
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
