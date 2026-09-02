//! Candidate QMK keycodes and categories for picker grids.

use super::picker::{Candidate, CandidateGroup};
use crate::key_action::KeyAction;
use crate::qmk_keycode_labels::{resolve_qmk_key, KeyResolution};
use qmk_via_api::keycodes::{Keycode, KeycodeCategory};
use qmk_via_api::ranges::{QK_KB, QK_MACRO, QK_TAP_DANCE, QK_USER};
use qmk_via_api::QmkLayerOp;
use std::sync::OnceLock;

fn candidate_group(name: &'static str, codes: impl IntoIterator<Item = u16>) -> CandidateGroup {
    CandidateGroup {
        name,
        candidates: codes
            .into_iter()
            .filter(|&c| matches!(resolve_qmk_key(c), KeyResolution::Key(_)))
            .map(qmk_candidate)
            .collect(),
    }
}

/// Returns the static list of QMK candidate key categories.
pub fn categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(|| {
        KeycodeCategory::ALL
            .iter()
            .map(|&cat| {
                let codes: Vec<u16> = match cat {
                    KeycodeCategory::Custom => Keycode::all_in_category(cat)
                        .iter()
                        .map(|&k| k as u16)
                        .chain(QK_TAP_DANCE.start..QK_TAP_DANCE.start + 32)
                        .collect(),
                    _ => Keycode::all_in_category(cat).iter().map(|&k| k as u16).collect(),
                };
                candidate_group(cat.label(), codes)
            })
            .collect()
    })
}

/// Returns the candidate group for a specific QMK keycode category.
pub fn category(cat: KeycodeCategory) -> &'static CandidateGroup {
    let index = KeycodeCategory::ALL
        .iter()
        .position(|&c| c == cat)
        .expect("all categories are indexed");
    &categories()[index]
}

/// Returns candidate groups for the Custom section (Macro, Tap Dance, User, Keyboard).
pub fn custom_groups() -> &'static [CandidateGroup] {
    static CUSTOM_GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CUSTOM_GROUPS.get_or_init(|| {
        vec![
            candidate_group("Macro", (0..32).map(|i| QK_MACRO.start + i)),
            candidate_group("Tap Dance", (0..32).map(|i| QK_TAP_DANCE.start + i)),
            candidate_group("User", (0..32).map(|i| QK_USER.start + i)),
            candidate_group("Keyboard", (0..32).map(|i| QK_KB.start + i)),
        ]
    })
}

/// Returns candidate groups for all supported layer keycode types.
pub fn layer_groups(layer_count: usize) -> Vec<CandidateGroup> {
    QmkLayerOp::ALL
        .iter()
        .map(|op| CandidateGroup {
            name: op.label(),
            candidates: (0..layer_count.min(32))
                .filter_map(|layer| op.encode(layer as u8))
                .map(qmk_candidate)
                .collect(),
        })
        .collect()
}

/// Returns a candidate group for selecting a layer (L0..L15).
pub fn layer_picker_group(layer_count: usize) -> CandidateGroup {
    CandidateGroup {
        name: "Layer",
        candidates: (0..layer_count.min(16))
            .filter_map(|layer| {
                let mo_code = QmkLayerOp::Momentary.encode(layer as u8)?;
                Some(qmk_candidate(mo_code))
            })
            .collect(),
    }
}

/// Creates a candidate definition for a QMK keycode.
pub fn qmk_candidate(code: u16) -> Candidate {
    Candidate::from_action(KeyAction::Qmk(code), &[])
}
