//! Shared transport, feature probing, and key writing utilities for QMK/VIA/VIAL protocols.

use crate::key_action::{KeyAction, KeymapSnapshot};
use qmk_via_api::api::KeyboardApi;
pub use qmk_via_api::QmkFeatures;
use std::error::Error;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Returns an action filter that disables keycodes not supported by the keyboard's features.
pub fn qmk_action_filter(features: QmkFeatures) -> Option<super::ActionFilter> {
    Some(Arc::new(move |action| match action {
        KeyAction::Qmk(code) => features.is_keycode_supported(*code),
        _ => true,
    }))
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
        let raw_matrix = api
            .read_raw_matrix(matrix_info, layer as u8)
            .map_err(|e| format!("Failed to read layer {layer} keymap: {e}"))?;
        for (i, &keycode) in raw_matrix.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            if row < rows && col < cols {
                layer_actions[row][col] = Some(KeyAction::Qmk(keycode));
            }
        }
    }

    Ok(KeymapSnapshot {
        layers: crate::key_action::LayerInfo::indexed(layer_count),
        actions,
    })
}

/// Writes a key binding to a QMK keyboard.
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

/// Writes a keycode via the VIA protocol with readback verification on error.
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

#[cfg(test)]
mod tests {
    use super::*;
    use qmk_via_api::keycodes::Keycode;

    #[test]
    fn test_qmk_action_filter() {
        let features = QmkFeatures {
            has_backlight: false,
            has_rgblight: true,
            has_rgb_matrix: false,
            has_audio: false,
        };

        let filter = qmk_action_filter(features).expect("filter should be Some");
        assert!(filter(&KeyAction::Qmk(Keycode::KC_A as u16)));
        assert!(filter(&KeyAction::Qmk(Keycode::QK_UNDERGLOW_TOGGLE as u16)));
        assert!(!filter(&KeyAction::Qmk(
            Keycode::QK_BACKLIGHT_TOGGLE as u16
        )));
        assert!(!filter(&KeyAction::Qmk(
            Keycode::QK_RGB_MATRIX_TOGGLE as u16
        )));
        assert!(!filter(&KeyAction::Qmk(Keycode::QK_AUDIO_TOGGLE as u16)));
    }
}
