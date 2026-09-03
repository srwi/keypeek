pub mod builders;
pub mod consumer;
pub mod keyboard;
pub mod os_layout;
pub mod system;

pub use builders::{layer_switch_key, layer_tap_key, mod_tap_key, one_shot_mod_key};

pub use consumer::hid_consumer_key;
pub use keyboard::hid_keyboard_key;
pub use system::hid_system_key;

use crate::layout_key::LayoutKey;

/// Resolves a key from its HID usage page and usage ID.
pub fn hid_usage_to_layout_key(page: u16, usage_id: u16) -> Option<LayoutKey> {
    match page {
        0x01 => hid_system_key(usage_id),
        0x07 => hid_keyboard_key(usage_id),
        0x0C => hid_consumer_key(usage_id),
        _ => None,
    }
}
