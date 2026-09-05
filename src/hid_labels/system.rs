use crate::layout_key::{Label, LayoutKey};

/// Resolves a USB HID Generic Desktop Page (0x01) system usage into a `LayoutKey`.
pub fn hid_system_key(usage_id: u16) -> Option<LayoutKey> {
    match usage_id {
        0x81 => Some(LayoutKey {
            tap: Label::new("Power"),
            ..Default::default()
        }),
        0x82 => Some(LayoutKey {
            tap: Label::new("Sleep"),
            ..Default::default()
        }),
        0x83 => Some(LayoutKey {
            tap: Label::new("Wake"),
            ..Default::default()
        }),
        _ => None,
    }
}
