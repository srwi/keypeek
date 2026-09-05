use crate::layout_key::{Label, LayoutKey};
use zmk_studio_api::Keycode;

#[allow(dead_code)]
pub fn keycode_to_layout_key(keycode: &Keycode) -> LayoutKey {
    let mut key = super::hid_usage::hid_usage_to_layout_key(
        zmk_studio_api::HidUsage::from_encoded(*keycode as u32),
    );
    if key.tap.full.starts_with("0x") {
        key.tap = Label::new(keycode.to_name());
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zmk_and_qmk_resolve_identical_base_keys() {
        let zmk_a = keycode_to_layout_key(&Keycode::A);
        let qmk_a = crate::qmk_keycode_labels::get_basic_layout_key(
            qmk_via_api::keycodes::Keycode::KC_A as u16,
        )
        .unwrap();
        assert_eq!(zmk_a.tap.full, qmk_a.tap.full);

        let zmk_1 = keycode_to_layout_key(&Keycode::NUMBER_1);
        let qmk_1 = crate::qmk_keycode_labels::get_basic_layout_key(
            qmk_via_api::keycodes::Keycode::KC_1 as u16,
        )
        .unwrap();
        assert_eq!(zmk_1.tap.full, qmk_1.tap.full);
        assert_eq!(zmk_1.shifted, qmk_1.shifted);

        let zmk_mute = keycode_to_layout_key(&Keycode::C_MUTE);
        let qmk_mute = crate::qmk_keycode_labels::get_basic_layout_key(
            qmk_via_api::keycodes::Keycode::KC_AUDIO_MUTE as u16,
        )
        .unwrap();
        assert_eq!(zmk_mute.symbol, qmk_mute.symbol);
    }

    #[test]
    fn zmk_and_qmk_resolve_identical_modified_keys() {
        use qmk_via_api::{QmkKeycode, QmkModMask};
        use zmk_studio_api::{HidUsage, MOD_LCTL, MOD_LSFT, MOD_RALT};

        // Shift-wrapped: both resolve to flat shifted char without badge
        let zmk_shift_1 = super::super::hid_usage::hid_usage_to_layout_key(HidUsage::from_parts(
            0x07, 0x1E, MOD_LSFT,
        ));
        let qmk_shift_1 = crate::qmk_keycode_labels::get_advanced_layout_key(
            QmkKeycode::encode_mod_combo(QmkModMask::from_bits(QmkModMask::LSFT), 0x1E).unwrap(),
        )
        .unwrap();
        assert_eq!(zmk_shift_1.tap.full, qmk_shift_1.tap.full);
        assert_eq!(zmk_shift_1.tap.full, "!");
        assert!(zmk_shift_1.argument.is_none());
        assert!(qmk_shift_1.argument.is_none());

        // Non-text modifier: both show base key + modifier badge
        let zmk_ctrl_c = super::super::hid_usage::hid_usage_to_layout_key(HidUsage::from_parts(
            0x07, 0x06, MOD_LCTL,
        ));
        let qmk_ctrl_c = crate::qmk_keycode_labels::get_advanced_layout_key(
            QmkKeycode::encode_mod_combo(QmkModMask::from_bits(QmkModMask::LCTL), 0x06).unwrap(),
        )
        .unwrap();
        assert_eq!(zmk_ctrl_c.tap.full, qmk_ctrl_c.tap.full);
        assert_eq!(zmk_ctrl_c.argument, qmk_ctrl_c.argument);

        // AltGr-wrapped: both resolve identically
        let zmk_ralt_8 = super::super::hid_usage::hid_usage_to_layout_key(HidUsage::from_parts(
            0x07, 0x25, MOD_RALT,
        ));
        let qmk_ralt_8 = crate::qmk_keycode_labels::get_advanced_layout_key(
            QmkKeycode::encode_mod_combo(
                QmkModMask::from_bits(QmkModMask::LALT | QmkModMask::RIGHT_HAND),
                0x25,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(zmk_ralt_8.tap.full, qmk_ralt_8.tap.full);
        assert_eq!(zmk_ralt_8.argument, qmk_ralt_8.argument);

        // Modified media key (e.g. Ctrl + Audio Mute): both show mute symbol + Ctrl badge
        let zmk_ctrl_mute = super::super::hid_usage::hid_usage_to_layout_key(HidUsage::from_parts(
            0x0C, 0xE2, MOD_LCTL,
        ));
        let qmk_ctrl_mute = crate::qmk_keycode_labels::get_advanced_layout_key(
            QmkKeycode::encode_mod_combo(QmkModMask::from_bits(QmkModMask::LCTL), 0xA8).unwrap(),
        )
        .unwrap();
        assert_eq!(zmk_ctrl_mute.symbol, qmk_ctrl_mute.symbol);
        assert!(zmk_ctrl_mute.symbol.is_some());
        assert_eq!(zmk_ctrl_mute.argument, qmk_ctrl_mute.argument);
    }
}
