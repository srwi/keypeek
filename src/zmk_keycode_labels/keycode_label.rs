use crate::layout_key::{Label, LayoutKey};
use zmk_studio_api::Keycode;

pub fn keycode_to_layout_key(keycode: &Keycode) -> LayoutKey {
    let raw = *keycode as u32;
    let mods = ((raw >> 24) & 0xFF) as u8;
    let mut page = ((raw >> 16) & 0xFF) as u16;
    if page == 0 {
        page = 0x07;
    }
    let usage_id = (raw & 0xFFFF) as u16;

    if mods == 0 {
        if let Some(key) = crate::hid_labels::hid_usage_to_layout_key(page, usage_id) {
            return key;
        }
    } else if mods & !(zmk_studio_api::MOD_LSFT | zmk_studio_api::MOD_RSFT) == 0 {
        if let Some(base_key) = crate::hid_labels::hid_usage_to_layout_key(page, usage_id) {
            if let Some(shifted) = base_key.shifted {
                return LayoutKey {
                    tap: Label::new(shifted),
                    ..Default::default()
                };
            }
        }
    }

    LayoutKey {
        tap: Label::new(keycode.to_name()),
        ..Default::default()
    }
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
}
