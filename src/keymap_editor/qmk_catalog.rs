//! Candidate QMK keycodes and categories for picker grids.

use super::picker::{Candidate, CandidateGroup};
use super::qmk_editor::{encode_layer_mod, encode_layer_tap};
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

/// Returns the static list of QMK candidate key categories.
pub fn categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(|| {
        let mut basic: Vec<u16> =
            keycodes_between(Keycode::KC_NO, Keycode::KC_SYSTEM_WAKE).collect();
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

/// QMK layer keycode types.
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

/// Encodes a layer keycode for a given layer kind and layer index.
pub(super) fn encode_layer(kind: LayerKind, layer: usize) -> Option<u16> {
    let range = kind.range();
    let layer = layer as u16;
    range
        .contains(&(range.start + layer))
        .then_some(range.start + layer)
}

/// Returns candidate groups for all supported layer keycode types.
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

/// Returns a candidate group for selecting a Layer-Tap layer.
pub fn layer_tap_group(layer_count: usize, tap_code: u16) -> CandidateGroup {
    CandidateGroup {
        name: "Layer",
        candidates: (0..layer_count.min(16))
            .filter_map(|layer| {
                let visual = qmk_candidate(encode_layer(LayerKind::Mo, layer)?).key;
                let lt_code = encode_layer_tap(layer, tap_code)?;
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
                let visual = qmk_candidate(encode_layer(LayerKind::Mo, layer)?).key;
                let lm_code = encode_layer_mod(layer, mods)?;
                Some(Candidate::new(KeyAction::Qmk(lm_code), visual))
            })
            .collect(),
    }
}

/// Creates a candidate definition for a QMK keycode.
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
