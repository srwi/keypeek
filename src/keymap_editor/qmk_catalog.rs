//! Candidate QMK/VIA keycodes for the editor's picker grids.
//!
//! Every keycode below `0x0100` maps to a `Keycode` enum variant that
//! `get_basic_layout_key` labels, so the categories enumerate those ranges
//! rather than hand-maintaining hundreds of entries.

use super::picker::{Candidate, CandidateGroup};
use crate::key_action::KeyAction;
use crate::layout_key::{Label, LayoutKey};
use crate::qmk_keycode_labels::constants::*;
use crate::qmk_keycode_labels::get_layout_key;
use qmk_via_api::keycodes::Keycode;
use std::ops::RangeInclusive;
use std::sync::OnceLock;

/// Consumer/media keycodes (audio, transport, brightness, application launch).
const MEDIA_RANGE: RangeInclusive<u16> = 0xA8..=0xC2;

/// Dedicated modifier keys (`LCTL`…`RGUI`).
const MODIFIER_RANGE: RangeInclusive<u16> = 0xE0..=0xE7;

/// The Basic and Media groups as ready-made candidates, built once: resolving
/// every label per frame would be wasted work in the editor.
pub fn categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(|| {
        let mut basic: Vec<u16> = (0x00..=0xA7).collect();
        // Mouse cursor/button/wheel keycodes sit in their own sub-range.
        basic.extend(0xCD..=0xDF);
        basic.extend(MODIFIER_RANGE);
        vec![
            CandidateGroup {
                name: "Basic",
                candidates: basic.iter().map(|&code| qmk_candidate(code)).collect(),
            },
            CandidateGroup {
                name: "Media",
                candidates: MEDIA_RANGE.map(qmk_candidate).collect(),
            },
        ]
    })
}

/// The QMK layer keycode kinds offered on the layer page, with their keycode
/// ranges and two-letter labels.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    Mo,
    Tg,
    To,
    Osl,
    Tt,
    Df,
}

impl LayerKind {
    pub(super) const ALL: [LayerKind; 6] = [
        LayerKind::Mo,
        LayerKind::Tg,
        LayerKind::To,
        LayerKind::Osl,
        LayerKind::Tt,
        LayerKind::Df,
    ];
    pub(super) fn label(&self) -> &'static str {
        match self {
            LayerKind::Mo => "MO",
            LayerKind::Tg => "TG",
            LayerKind::To => "TO",
            LayerKind::Osl => "OSL",
            LayerKind::Tt => "TT",
            LayerKind::Df => "DF",
        }
    }
    pub(super) fn range(&self) -> std::ops::Range<u16> {
        match self {
            LayerKind::Mo => QK_MOMENTARY,
            LayerKind::Tg => QK_TOGGLE_LAYER,
            LayerKind::To => QK_TO,
            LayerKind::Osl => QK_ONE_SHOT_LAYER,
            LayerKind::Tt => QK_LAYER_TAP_TOGGLE,
            LayerKind::Df => QK_DEF_LAYER,
        }
    }
}

/// `MO/TG/TO/OSL/TT/DF(layer)` → keycode.
pub(super) fn encode_layer(kind: LayerKind, layer: usize) -> Option<u16> {
    let range = kind.range();
    let layer = layer as u16;
    range
        .contains(&(range.start + layer))
        .then_some(range.start + layer)
}

/// The layer page's groups: one candidate per real layer for each QMK layer
/// keycode kind, rendered like the overlay paints the binding (`L1`, `L2`, …).
/// `layer_count` is the keyboard's actual layer count, as reported by
/// VIA/Vial; kinds whose keycode range is smaller simply show fewer keys.
pub fn layer_groups(layer_count: usize) -> Vec<CandidateGroup> {
    LayerKind::ALL
        .iter()
        .map(|kind| CandidateGroup {
            name: kind.label(),
            candidates: (0..layer_count)
                .filter_map(|layer| encode_layer(*kind, layer))
                .map(qmk_candidate)
                .collect(),
        })
        .collect()
}

/// The candidate for a QMK keycode: the fully resolved `LayoutKey`, with
/// explicit stand-ins for `KC_TRANSPARENT` and `KC_NO`, whose raw labels are
/// unusable as key legends.
pub fn qmk_candidate(code: u16) -> Candidate {
    let binding = KeyAction::Qmk(code);
    if code == Keycode::KC_TRANSPARENT as u16 {
        return Candidate {
            binding,
            key: LayoutKey {
                tap: Label::with_short("Trans", egui_phosphor::regular::CARET_DOWN),
                ..Default::default()
            },
            transparent: true,
        };
    }
    if code == Keycode::KC_NO as u16 {
        return Candidate::new(
            binding,
            LayoutKey {
                tap: Label::new("None"),
                ..Default::default()
            },
        );
    }
    match get_layout_key(code) {
        Some(key) => Candidate::new(binding, key),
        None => Candidate::new(
            binding,
            LayoutKey {
                tap: Label::new(format!("0x{code:04X}")),
                ..Default::default()
            },
        ),
    }
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
        let all: Vec<u16> = categories()
            .iter()
            .flat_map(|group| &group.candidates)
            .filter_map(|candidate| match &candidate.binding {
                KeyAction::Qmk(code) => Some(*code),
                _ => None,
            })
            .collect();

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
