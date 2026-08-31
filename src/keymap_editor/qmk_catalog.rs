//! Candidate QMK keycodes and categories for picker grids.

use super::picker::{Candidate, CandidateGroup};
use crate::key_action::KeyAction;
use crate::qmk_keycode_labels::{resolve_qmk_key, KeyResolution};
use qmk_via_api::keycodes::{Keycode, KeycodeCategory};
use qmk_via_api::{QmkKeycode, QmkLayerOp, QmkModMask};
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
                candidate_group(
                    cat.label(),
                    Keycode::all_in_category(cat).iter().map(|&k| k as u16),
                )
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

/// Returns a candidate group for selecting a Layer-Tap layer.
pub fn layer_tap_group(layer_count: usize, tap_code: u16) -> CandidateGroup {
    CandidateGroup {
        name: "Layer",
        candidates: (0..layer_count.min(16))
            .filter_map(|layer| {
                let visual = qmk_candidate(QmkLayerOp::Momentary.encode(layer as u8)?).key;
                let lt_code = QmkKeycode::encode_layer_tap(layer as u8, tap_code as u8)?;
                Some(Candidate::new(KeyAction::Qmk(lt_code), visual))
            })
            .collect(),
    }
}

/// Returns a candidate group for selecting a Layer-Mod layer.
pub fn layer_mod_group(layer_count: usize, mods: u16) -> CandidateGroup {
    CandidateGroup {
        name: "Layer",
        candidates: (0..layer_count.min(16))
            .filter_map(|layer| {
                let visual = qmk_candidate(QmkLayerOp::Momentary.encode(layer as u8)?).key;
                let lm_code =
                    QmkKeycode::encode_layer_mod(layer as u8, QmkModMask::from_bits(mods as u8))?;
                Some(Candidate::new(KeyAction::Qmk(lm_code), visual))
            })
            .collect(),
    }
}

/// Creates a candidate definition for a QMK keycode.
pub fn qmk_candidate(code: u16) -> Candidate {
    Candidate::from_action(KeyAction::Qmk(code), &[])
}
