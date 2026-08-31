//! Shared transport, feature probing, and key writing utilities for QMK/VIA/VIAL protocols.

use crate::key_action::{KeyAction, KeymapSnapshot};
use qmk_via_api::api::KeyboardApi;
use std::error::Error;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use qmk_via_api::keycodes::Keycode;

#[derive(Debug, Clone, Default)]
pub struct QmkFeatures {
    pub has_backlight: bool,
    pub has_rgblight: bool,
    pub has_rgb_matrix: bool,
    pub has_audio: bool,
}

impl QmkFeatures {
    pub fn probe(api: &KeyboardApi) -> Self {
        Self {
            has_backlight: api.get_backlight_brightness().is_ok(),
            has_rgblight: api.get_rgblight_brightness().is_ok(),
            has_rgb_matrix: api.get_rgb_matrix_brightness().is_ok(),
            has_audio: api.get_audio_enabled().is_ok(),
        }
    }

    pub fn is_keycode_supported(&self, code: u16) -> bool {
        // Backlight range
        if (Keycode::QK_BACKLIGHT_ON as u16..=Keycode::QK_BACKLIGHT_TOGGLE_BREATHING as u16)
            .contains(&code)
            && !self.has_backlight
        {
            return false;
        }
        // RGBLight range
        if (Keycode::QK_UNDERGLOW_TOGGLE as u16..=Keycode::RGB_MODE_TWINKLE as u16).contains(&code)
            && !self.has_rgblight
        {
            return false;
        }
        // RGB Matrix range
        if (Keycode::QK_RGB_MATRIX_ON as u16..=Keycode::QK_RGB_MATRIX_SPEED_DOWN as u16)
            .contains(&code)
            && !self.has_rgb_matrix
        {
            return false;
        }
        // Audio range
        if (Keycode::QK_AUDIO_ON as u16..=Keycode::QK_AUDIO_VOICE_PREVIOUS as u16).contains(&code)
            && !self.has_audio
        {
            return false;
        }
        true
    }

    pub fn action_filter(&self) -> Option<super::ActionFilter> {
        let features = self.clone();
        Some(Arc::new(move |action| match action {
            KeyAction::Qmk(code) => features.is_keycode_supported(*code),
            _ => true,
        }))
    }
}

/// Reads a complete keymap snapshot across all dynamic layers from a QMK/VIA/VIAL keyboard.
pub fn qmk_read_snapshot(
    api: &KeyboardApi,
    definition: &super::KeyboardDefinition,
) -> Result<KeymapSnapshot, Box<dyn Error>> {
    let layer_count = api
        .get_layer_count()
        .map_err(|e| format!("Failed to get layer count: {e}"))? as usize;
    let (rows, cols) = (definition.rows, definition.cols);
    let matrix_info = qmk_via_api::api::MatrixInfo {
        rows: rows as u8,
        cols: cols as u8,
    };

    let mut actions = vec![vec![vec![None; cols]; rows]; layer_count];
    for (layer, layer_actions) in actions.iter_mut().enumerate() {
        if let Ok(raw_matrix) = api.read_raw_matrix(matrix_info, layer as u8) {
            for (i, &keycode) in raw_matrix.iter().enumerate() {
                let row = i / cols;
                let col = i % cols;
                layer_actions[row][col] = Some(KeyAction::Qmk(keycode));
            }
        }
    }

    Ok(KeymapSnapshot {
        layers: crate::key_action::LayerInfo::indexed(layer_count),
        actions,
    })
}

/// Writes one key binding to a QMK/VIA/VIAL keyboard.
pub fn qmk_set_key(
    api: &KeyboardApi,
    layer_index: usize,
    row: usize,
    col: usize,
    action: &KeyAction,
) -> Result<(), Box<dyn Error>> {
    match action {
        KeyAction::Qmk(code) => qmk_set_key_with_retry(api, layer_index, row, col, *code),
        KeyAction::Zmk(_) => Err("Cannot apply a ZMK behavior to a QMK keyboard".into()),
    }
}

/// Writes one dynamic-keymap keycode through the VIA protocol.
///
/// A layer-state packet arriving between `set_key`'s send and its single
/// response read makes the crate report `BadCommandResponse` even though the
/// write usually applied; a matching `get_key` readback confirms success.
pub(crate) fn qmk_set_key_with_retry(
    api: &KeyboardApi,
    layer_index: usize,
    row: usize,
    col: usize,
    code: u16,
) -> Result<(), Box<dyn Error>> {
    match api.set_key(layer_index as u8, row as u8, col as u8, code) {
        Ok(_) => Ok(()),
        Err(qmk_via_api::Error::BadCommandResponse(_)) => {
            for _ in 0..3 {
                thread::sleep(Duration::from_millis(50));
                if let Ok(readback) = api.get_key(layer_index as u8, row as u8, col as u8) {
                    if readback == code {
                        return Ok(());
                    }
                }
            }
            Err("Failed to set key: the device did not confirm the write".into())
        }
        Err(e) => Err(format!("Failed to set key: {e}").into()),
    }
}
