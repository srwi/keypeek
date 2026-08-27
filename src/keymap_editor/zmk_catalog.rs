//! Candidate ZMK keycodes for the editor's picker grids, split into the
//! keyboard (usage page 0x07) and consumer (page 0x0C) pages. The candidate
//! lists and their button text are computed once and cached, since enumerating
//! every encoded usage is expensive to do per frame.

use crate::key_action::KeyAction;
use std::sync::OnceLock;
use zmk_studio_api::{Behavior, HidUsage, Keycode};
pub const HID_USAGE_KEYBOARD: u16 = 0x07;
pub const HID_USAGE_CONSUMER: u16 = 0x0C;

pub struct Category {
    pub name: &'static str,
    /// Encoded HID usages (page | id | modifiers), usable directly as
    /// `HidUsage::from_encoded`, with their resolved button text.
    pub candidates: Vec<super::picker::Candidate>,
}

pub fn categories() -> &'static [Category] {
    static CATEGORIES: OnceLock<Vec<Category>> = OnceLock::new();
    CATEGORIES.get_or_init(build_categories)
}

fn build_categories() -> Vec<Category> {
    let mut keyboard = Vec::new();
    let mut consumer = Vec::new();
    // Scan every encoded usage below the consumer range; keep only known
    // keyboard-page (0x07) and consumer-page (0x0C) keycodes.
    for encoded in 0..=((HID_USAGE_CONSUMER as u32) << 16 | 0x3FF) {
        let Some(_keycode) = Keycode::from_hid_usage(encoded) else {
            continue;
        };
        let page = HidUsage::from_encoded(encoded).page();
        let candidate = keycode_candidate(encoded);
        match page {
            HID_USAGE_KEYBOARD => keyboard.push(candidate),
            HID_USAGE_CONSUMER => consumer.push(candidate),
            _ => {}
        }
    }
    vec![
        Category {
            name: "Keyboard",
            candidates: keyboard,
        },
        Category {
            name: "Consumer",
            candidates: consumer,
        },
    ]
}

/// The candidate for a ZMK keycode: the fully resolved `LayoutKey` from the
/// key-press behavior, falling back to a hex label for unknown usages.
pub fn keycode_candidate(encoded: u32) -> super::picker::Candidate {
    let usage = HidUsage::from_encoded(encoded);
    let action = KeyAction::Zmk(Behavior::KeyPress(usage));
    let key = match action.resolve_label(&[]) {
        Some(key) if !key.tap.full.is_empty() => key,
        Some(mut key) => {
            if key.symbol.is_none() && key.tap.is_empty() {
                key.symbol = Some(format!("0x{:02X}", usage.id()));
            }
            key
        }
        None => crate::layout_key::LayoutKey {
            tap: crate::layout_key::Label::new(format!("0x{:02X}", usage.id())),
            ..Default::default()
        },
    };
    super::picker::Candidate::new(encoded, key)
}
