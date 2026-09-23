use super::advanced::get_advanced_layout_key;
use super::basic::get_basic_layout_key;
use super::layer::get_layer_layout_key;
use keypeek_core::LayoutKey;

use qmk_via_api::keycodes::Keycode;

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
