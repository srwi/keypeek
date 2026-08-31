//! Candidate QMK/VIA keycodes for the editor's picker grids.
//!
//! Every keycode below `0x0100` maps to a `Keycode` enum variant that
//! `get_basic_layout_key` labels, so the categories enumerate those ranges
//! rather than hand-maintaining hundreds of entries.

use super::picker::{Candidate, CandidateGroup};
use crate::key_action::KeyAction;
use crate::layout_key::{Label, LayoutKey};
use crate::qmk_keycode_labels::constants::*;
use crate::qmk_keycode_labels::{get_hex_layout_key, resolve_qmk_key, KeyResolution};
use qmk_via_api::keycodes::Keycode;
use std::sync::OnceLock;

fn keycodes_between(from: Keycode, to: Keycode) -> impl Iterator<Item = u16> {
    (from as u16)..=(to as u16)
}

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

/// The candidate groups built once: resolving every label per frame would be
/// wasted work in the editor.
pub fn categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(|| {
        let mut basic: Vec<u16> =
            keycodes_between(Keycode::KC_NO, Keycode::KC_SYSTEM_WAKE).collect();
        // Mouse cursor/button/wheel keycodes sit in their own sub-range.
        basic.extend(keycodes_between(
            Keycode::QK_MOUSE_CURSOR_UP,
            Keycode::QK_MOUSE_ACCELERATION_2,
        ));
        basic.extend(keycodes_between(
            Keycode::KC_LEFT_CTRL,
            Keycode::KC_RIGHT_GUI,
        ));

        let special = [
            Keycode::QK_BOOTLOADER,
            Keycode::QK_REBOOT,
            Keycode::QK_GRAVE_ESCAPE,
            Keycode::QK_CAPS_WORD_TOGGLE,
            Keycode::QK_REPEAT_KEY,
            Keycode::QK_ALT_REPEAT_KEY,
            Keycode::QK_CLEAR_EEPROM,
            Keycode::QK_DEBUG_TOGGLE,
        ]
        .map(|k| k as u16);

        let audio = [
            Keycode::QK_AUDIO_ON,
            Keycode::QK_AUDIO_OFF,
            Keycode::QK_AUDIO_TOGGLE,
            Keycode::QK_AUDIO_CLICKY_TOGGLE,
            Keycode::QK_AUDIO_CLICKY_UP,
            Keycode::QK_AUDIO_CLICKY_DOWN,
            Keycode::QK_AUDIO_CLICKY_RESET,
        ]
        .map(|k| k as u16);

        let custom: Vec<u16> = keycodes_between(Keycode::QK_MACRO_0, Keycode::QK_MACRO_15)
            .chain(keycodes_between(Keycode::QK_KB_0, Keycode::QK_KB_31))
            .chain(keycodes_between(Keycode::QK_USER_0, Keycode::QK_USER_31))
            .collect();

        vec![
            candidate_group("Basic", basic),
            candidate_group(
                "Media",
                keycodes_between(Keycode::KC_AUDIO_MUTE, Keycode::KC_LAUNCHPAD),
            ),
            candidate_group("Special", special),
            candidate_group(
                "Backlight",
                keycodes_between(
                    Keycode::QK_BACKLIGHT_ON,
                    Keycode::QK_BACKLIGHT_TOGGLE_BREATHING,
                ),
            ),
            candidate_group(
                "RGB Underglow",
                keycodes_between(Keycode::QK_UNDERGLOW_TOGGLE, Keycode::RGB_MODE_TWINKLE),
            ),
            candidate_group(
                "RGB Matrix",
                keycodes_between(
                    Keycode::QK_RGB_MATRIX_ON,
                    Keycode::QK_RGB_MATRIX_SPEED_DOWN,
                ),
            ),
            candidate_group("Audio", audio),
            candidate_group("Custom", custom),
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

/// The Layer-Tap page's layer radio: one plain layer key per real layer,
/// rendered like the layer page's keys. Each candidate's binding is the full
/// `LT(layer, tap)` keycode for the draft's current tap key, so the editor can
/// highlight the staged layer and stage a new one from the same grid.
pub fn layer_tap_group(layer_count: usize, tap_code: u16) -> CandidateGroup {
    CandidateGroup {
        name: "Layer",
        candidates: (0..layer_count.min(16))
            .filter(|_| tap_code <= 0xFF)
            .filter_map(|layer| {
                let visual = qmk_candidate(encode_layer(LayerKind::Mo, layer)?).key;
                let lt_code = QK_LAYER_TAP.start + ((layer as u16) << 8) + tap_code;
                Some(Candidate::new(KeyAction::Qmk(lt_code), visual))
            })
            .collect(),
    }
}

/// The Layer-Mod page's layer radio: one plain layer key per real layer,
/// rendered like the layer page's keys.
pub fn layer_mod_group(layer_count: usize, mods: u16) -> CandidateGroup {
    CandidateGroup {
        name: "Layer",
        candidates: (0..layer_count.min(16))
            .filter_map(|layer| {
                let visual = qmk_candidate(encode_layer(LayerKind::Mo, layer)?).key;
                let lm_code = QK_LAYER_MOD.start + ((layer as u16) << 5) + (mods & 0x1F);
                Some(Candidate::new(KeyAction::Qmk(lm_code), visual))
            })
            .collect(),
    }
}

/// The candidate for a QMK keycode: the fully resolved `LayoutKey`, with
/// explicit stand-ins for `KC_TRANSPARENT` and `KC_NO`, whose raw labels are
/// unusable as key legends.
pub fn qmk_candidate(code: u16) -> Candidate {
    let binding = KeyAction::Qmk(code);
    if code == Keycode::KC_NO as u16 {
        return Candidate::new(
            binding,
            LayoutKey {
                tap: Label::new("None"),
                ..Default::default()
            },
        );
    }
    match resolve_qmk_key(code) {
        KeyResolution::Transparent => Candidate {
            binding,
            key: LayoutKey {
                tap: Label::with_short("Transparent", egui_phosphor::regular::CARET_DOWN),
                ..Default::default()
            },
            transparent: true,
        },
        KeyResolution::Key(key) => Candidate::new(binding, *key),
        KeyResolution::Unknown => Candidate::new(binding, get_hex_layout_key(code)),
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

    #[test]
    fn all_catalog_candidates_have_labels() {
        for group in categories() {
            for candidate in &group.candidates {
                assert!(
                    !candidate.key.tap.full.is_empty()
                        || candidate.key.symbol.is_some()
                        || candidate.transparent,
                    "Group {:?} candidate {:?} has no label or symbol",
                    group.name,
                    candidate.binding
                );
            }
        }
    }

    #[test]
    fn invalid_keycodes_excluded_from_catalog() {
        let all: Vec<u16> = categories()
            .iter()
            .flat_map(|group| &group.candidates)
            .filter_map(|candidate| match &candidate.binding {
                KeyAction::Qmk(code) => Some(*code),
                _ => None,
            })
            .collect();

        assert!(!all.contains(&0x0002), "0x0002 should not be in catalog");
        assert!(!all.contains(&0x0003), "0x0003 should not be in catalog");
        assert_eq!(get_layout_key(0x0002), None);
        assert_eq!(get_layout_key(0x0003), None);
    }
}
