use qmk_via_api::keycodes::Keycode;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::key_matrix::KeyMatrix;
use crate::protocols::{KeyboardLayout, KeyboardProtocol};

pub struct Keyboard {
    pub layout: KeyboardLayout,
    pub time_to_hide_overlay: Arc<Mutex<Option<Instant>>>,
    matrix: Arc<Mutex<KeyMatrix>>,
    layer_state: Arc<Mutex<u32>>,
    default_layer_state: Arc<Mutex<u32>>,
}

impl Keyboard {
    pub fn new(
        protocol: Box<dyn KeyboardProtocol>,
        layout_name: String,
        timeout: u64,
    ) -> Result<Self, String> {
        let definition = protocol.get_layout_definition();

        let layout = definition
            .get_layout(&layout_name)
            .map_err(|_| "Failed to get layout".to_string())?;

        let layers = protocol
            .get_layer_count()
            .map_err(|e| format!("Failed to get layer count: {e}"))?;

        let keycodes = protocol.read_all_keycodes(layers, definition.rows, definition.cols);

        let layer_state = Arc::new(Mutex::new(0));
        let default_layer_state = Arc::new(Mutex::new(0));
        let time_to_hide_overlay = Arc::new(Mutex::new(Some(Instant::now())));
        let matrix = Arc::new(Mutex::new(KeyMatrix::new(
            keycodes,
            definition.rows,
            definition.cols,
        )));

        let keyboard = Keyboard {
            layout,
            matrix: Arc::clone(&matrix),
            time_to_hide_overlay: Arc::clone(&time_to_hide_overlay),
            layer_state: Arc::clone(&layer_state),
            default_layer_state: Arc::clone(&default_layer_state),
        };

        let layer_state_clone = Arc::clone(&keyboard.layer_state);
        let default_layer_state_clone = Arc::clone(&keyboard.default_layer_state);
        let time_to_hide_clone = Arc::clone(&keyboard.time_to_hide_overlay);
        let matrix_clone = Arc::clone(&matrix);

        thread::spawn(move || loop {
            if let Ok(response) = protocol.hid_read() {
                if response[0] == 0xff {
                    let size = response[1] as usize;

                    let mut default_bytes = [0u8; 4];
                    default_bytes[..size].copy_from_slice(&response[2..2 + size]);
                    let default_layer_state = u32::from_le_bytes(default_bytes);

                    let mut layer_bytes = [0u8; 4];
                    layer_bytes[..size].copy_from_slice(&response[2 + size..2 + 2 * size]);
                    let layer_state = u32::from_le_bytes(layer_bytes);

                    if layer_state > 1 {
                        *time_to_hide_clone.lock().unwrap() = None;
                    } else {
                        let time_to_hide = Instant::now() + Duration::from_millis(timeout);
                        *time_to_hide_clone.lock().unwrap() = Some(time_to_hide);
                    }

                    *layer_state_clone.lock().unwrap() = layer_state;
                    *default_layer_state_clone.lock().unwrap() = default_layer_state;
                } else if response[0] == 0xF1 {
                    let row = response[1] as usize;
                    let col = response[2] as usize;
                    let pressed = response[3];
                    if let Ok(mut mat) = matrix_clone.lock() {
                        mat.set_pressed(row, col, pressed != 0);
                    }
                }
            }
        });

        Ok(keyboard)
    }

    pub fn get_effective_key_layer(&self, row: usize, col: usize) -> (u8, bool) {
        let layer_state = *self.layer_state.lock().unwrap();
        let default_layer_state = *self.default_layer_state.lock().unwrap();
        let matrix = self.matrix.lock().unwrap();
        let num_layers = matrix.get_num_layers().min(32);

        // Track if there is any active momentary layer above the effective layer
        // (i.e, key should be shown as background key)
        let mut active_layer_above = false;

        for i in (1..num_layers).rev() {
            let layer_mask = 1u32 << (i as u32);
            let is_active_default_layer = (default_layer_state & layer_mask) != 0;
            let is_active_momentary_layer = (layer_state & layer_mask) != 0;
            if is_active_momentary_layer || is_active_default_layer {
                if matrix.get_keycode(i, row, col) != Keycode::KC_TRANSPARENT as u16 {
                    return (i as u8, is_active_default_layer && active_layer_above);
                }
            }
            active_layer_above |= is_active_momentary_layer;
        }

        (0, active_layer_above)
    }

    pub fn get_keycode(&self, layer: usize, row: usize, col: usize) -> u16 {
        self.matrix.lock().unwrap().get_keycode(layer, row, col)
    }

    pub fn is_key_pressed(&self, row: usize, col: usize) -> bool {
        self.matrix.lock().unwrap().is_pressed(row, col)
    }
}
