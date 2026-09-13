use super::basic::get_basic_layout_key;
use super::constants::*;
use crate::layout_key::{behavior_names, BorderStyle, KeycodeKind, Label, LayoutKey};
use qmk_via_api::{QmkKeycode, QmkModMask};

pub fn get_advanced_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    match QmkKeycode::from_u16(keycode_bytes) {
        QmkKeycode::ModCombo { mods, keycode } => {
            let base = get_basic_layout_key(keycode as u16);
            Some(crate::hid_labels::mod_combo_key(
                0x07,
                keycode as u16,
                crate::firmware::qmk::codec::from_qmk_mask(mods),
                base,
            ))
        }
        QmkKeycode::ModTap { mods, keycode } => {
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();
            Some(crate::hid_labels::mod_tap_key(
                tap_key,
                mod_mask_to_label(mods),
                held_mod_mask(mods),
                Some(behavior_names::MOD_TAP.label()),
            ))
        }
        QmkKeycode::LayerMod { layer, mods } => Some(LayoutKey {
            tap: Label::new(format!("L{}", layer)),
            argument: (!mods.is_empty()).then(|| mod_mask_to_label(mods)),
            mod_mask: held_mod_mask(mods),
            kind: KeycodeKind::Modifier,
            layer_ref: Some(layer),
            border: BorderStyle::None,
            ..Default::default()
        }),
        QmkKeycode::OneShotMod(mods) => Some(crate::hid_labels::one_shot_mod_key(
            mod_mask_to_label(mods),
            held_mod_mask(mods),
            Some(behavior_names::ONE_SHOT_MOD.label()),
        )),
        QmkKeycode::LayerTap { layer, keycode } => {
            let tap_key = get_basic_layout_key(keycode as u16).unwrap_or_default();
            Some(crate::hid_labels::layer_tap_key(layer, tap_key, None))
        }
        _ => None,
    }
}

fn held_mod_mask(mods: QmkModMask) -> Option<u16> {
    let mask = to_held_mod_mask(mods);
    (mask != 0).then_some(mask)
}

fn mod_mask_to_label(mods: QmkModMask) -> Label {
    crate::layout_key::modifier_symbols::glyphs(
        mods.has_ctrl(),
        mods.has_shift(),
        mods.has_alt(),
        mods.has_gui(),
    )
}
