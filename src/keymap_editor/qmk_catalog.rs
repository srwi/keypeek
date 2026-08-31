//! Candidate QMK keycodes and categories for picker grids.

use super::picker::{Candidate, CandidateGroup};
use super::qmk_editor::{encode_layer_mod, encode_layer_tap};
use crate::key_action::KeyAction;
use crate::qmk_keycode_labels::constants::*;
use crate::qmk_keycode_labels::{resolve_qmk_key, KeyResolution};
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
    Candidate::from_action(KeyAction::Qmk(code), &[])
}
