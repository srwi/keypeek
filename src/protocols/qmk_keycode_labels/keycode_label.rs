use super::advanced::get_advanced_layout_key;
use super::basic::get_basic_layout_key;
use super::layer::get_layer_layout_key;
use crate::layout_key::{Label, LayoutKey};

use qmk_via_api::keycodes::Keycode;

/// Derives the display `LayoutKey` for a QMK keycode.
/// Returns `None` for `KC_TRANSPARENT` (falls through to lower layers).
/// Unknown keycodes return a hex fallback (`0x%04X`) so they are never silently dropped.
#[allow(dead_code)]
pub fn qmk_to_layout_key(bytes: u16) -> Option<LayoutKey> {
    if bytes == Keycode::KC_TRANSPARENT as u16 {
        return None;
    }

    try_resolve_qmk_key(bytes).or_else(|| Some(get_hex_layout_key(bytes)))
}

/// Attempts to resolve a QMK keycode to a known key layout, returning `None` if unknown
/// or transparent.
pub fn try_resolve_qmk_key(bytes: u16) -> Option<LayoutKey> {
    if bytes == Keycode::KC_TRANSPARENT as u16 {
        return None;
    }

    get_basic_layout_key(bytes)
        .or_else(|| get_layer_layout_key(bytes))
        .or_else(|| get_advanced_layout_key(bytes))
}

#[allow(dead_code)]
pub fn get_hex_layout_key(bytes: u16) -> LayoutKey {
    LayoutKey {
        tap: Label::new(format!("0x{:04X}", bytes)),
        ..Default::default()
    }
}
