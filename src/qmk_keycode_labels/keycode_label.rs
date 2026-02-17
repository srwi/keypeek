use crate::layout_key::{Label, LayoutKey};
use crate::qmk_keycode_labels::advanced::get_advanced_layout_key;
use crate::qmk_keycode_labels::basic::get_basic_layout_key;
use crate::qmk_keycode_labels::layer::get_layer_layout_key;
use qmk_via_api::keycodes::Keycode;

pub fn get_layout_key(bytes: u16) -> Option<LayoutKey> {
    if bytes == Keycode::KC_TRANSPARENT as u16 {
        return None;
    }

    get_basic_layout_key(bytes)
        .or_else(|| get_layer_layout_key(bytes))
        .or_else(|| get_advanced_layout_key(bytes))
        .or_else(|| Some(get_hex_layout_key(bytes)))
}

fn get_hex_layout_key(keycode_bytes: u16) -> LayoutKey {
    LayoutKey {
        tap: Label::new(format!("0x{:04X}", keycode_bytes)),
        ..Default::default()
    }
}
