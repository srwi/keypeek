use crate::layout_key::{Label, LayoutKey};
use crate::qmk_keycode_labels::advanced::get_advanced_layout_key;
use crate::qmk_keycode_labels::basic::get_basic_layout_key;
use crate::qmk_keycode_labels::layer::get_layer_layout_key;
use qmk_via_api::keycodes::Keycode;

/// The semantic classification and visual layout resolution of a QMK keycode.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyResolution {
    /// Transparent slot that falls through to lower layers.
    Transparent,
    /// A recognized QMK key with defined labels, symbols, and layout properties.
    Key(LayoutKey),
    /// Unrecognized/custom firmware keycode with no defined semantic mapping.
    Unknown,
}

/// Resolves a raw QMK keycode byte sequence into its semantic classification.
pub fn resolve_qmk_key(bytes: u16) -> KeyResolution {
    if bytes == Keycode::KC_TRANSPARENT as u16 {
        return KeyResolution::Transparent;
    }

    if let Some(key) = get_basic_layout_key(bytes)
        .or_else(|| get_layer_layout_key(bytes))
        .or_else(|| get_advanced_layout_key(bytes))
    {
        KeyResolution::Key(key)
    } else {
        KeyResolution::Unknown
    }
}

/// Convenience helper to obtain a `LayoutKey` if the keycode is a valid, recognized key.
pub fn get_layout_key(bytes: u16) -> Option<LayoutKey> {
    match resolve_qmk_key(bytes) {
        KeyResolution::Key(key) => Some(key),
        _ => None,
    }
}

pub fn get_hex_layout_key(keycode_bytes: u16) -> LayoutKey {
    LayoutKey {
        tap: Label::new(format!("0x{:04X}", keycode_bytes)),
        ..Default::default()
    }
}
