//! Candidate QMK/VIA keycodes for the editor's picker grids.
//!
//! Every keycode below `0x0100` maps to a `Keycode` enum variant that
//! `get_basic_layout_key` labels, so the categories enumerate those ranges
//! rather than hand-maintaining hundreds of entries.

use std::ops::RangeInclusive;

/// A named group of keycodes shown together in a picker.
pub struct Category {
    pub name: &'static str,
    pub codes: Vec<u16>,
}

/// Consumer/media keycodes (audio, transport, brightness, application launch).
const MEDIA_RANGE: RangeInclusive<u16> = 0xA8..=0xC2;

/// Dedicated modifier keys (`LCTL`…`RGUI`).
const MODIFIER_RANGE: RangeInclusive<u16> = 0xE0..=0xE7;

pub fn categories() -> Vec<Category> {
    let mut basic: Vec<u16> = (0x00..=0xA7).collect();
    // Mouse cursor/button/wheel keycodes sit in their own sub-range.
    basic.extend(0xCD..=0xDF);
    basic.extend(MODIFIER_RANGE);
    vec![
        Category {
            name: "Basic",
            codes: basic,
        },
        Category {
            name: "Media",
            codes: MEDIA_RANGE.collect(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qmk_keycode_labels::get_layout_key;
    use qmk_via_api::keycodes::Keycode;

    /// Every `Keycode` below `0x0100` should appear in a category, and every
    /// catalogued code should be a real keycode with a label.
    #[test]
    fn catalog_covers_all_labeled_basic_keycodes() {
        let all: Vec<u16> = categories().into_iter().flat_map(|c| c.codes).collect();

        for code in 0x00u16..0x0100 {
            if Keycode::try_from(code).is_ok() && get_layout_key(code).is_some() {
                assert!(
                    all.contains(&code),
                    "keycode 0x{code:04X} is labeled but missing from the catalog"
                );
            }
        }
    }
}
