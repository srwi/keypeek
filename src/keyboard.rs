use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::key_matrix::KeyMatrix;
use crate::layout_key::LayoutKey;
use crate::protocols::{KeyboardLayout, KeyboardProtocol};
use crate::ui_wake::UiWake;

/// A layer packet's size field is `sizeof(layer_state_t)` and at most 4 bytes.
const MAX_LAYER_STATE_BYTES: usize = 4;

pub struct Keyboard {
    pub layout: KeyboardLayout,
    pub time_to_hide_overlay: Arc<Mutex<Option<Instant>>>,
    matrix: Arc<Mutex<KeyMatrix>>,
    layer_state: Arc<Mutex<u32>>,
    default_layer_state: Arc<Mutex<u32>>,
    timeout_ms: Arc<Mutex<i64>>,
    alive: Arc<AtomicBool>,
    _keepalive: Option<mpsc::Sender<()>>,
}

impl Keyboard {
    pub fn new(
        protocol: Box<dyn KeyboardProtocol>,
        layout_name: String,
        timeout: i64,
        ui_wake: UiWake,
    ) -> Result<Self, String> {
        let definition = protocol.get_layout_definition();

        let layout = definition
            .get_layout(&layout_name)
            .map_err(|_| "Failed to get layout".to_string())?;

        let layers = protocol
            .get_layer_count()
            .map_err(|e| format!("Failed to get layer count: {e}"))?;

        let keys = protocol.read_all_keys(layers, definition.rows, definition.cols);
        let matrix = KeyMatrix::from_layout_keys(keys, definition.rows, definition.cols);

        let layer_state = Arc::new(Mutex::new(0));
        let default_layer_state = Arc::new(Mutex::new(0));
        let time_to_hide_overlay = Arc::new(Mutex::new(Some(Instant::now())));
        let timeout_ms = Arc::new(Mutex::new(timeout));
        let matrix = Arc::new(Mutex::new(matrix));
        let alive = Arc::new(AtomicBool::new(true));

        let keepalive = protocol.subscription_sender().map(|sender| {
            let (tx, rx) = mpsc::channel::<()>();
            thread::spawn(move || {
                loop {
                    let _ = sender.set_active(true);
                    match rx.recv_timeout(Duration::from_millis(1000)) {
                        Err(RecvTimeoutError::Timeout) => continue,
                        _ => break,
                    }
                }
                let _ = sender.set_active(false);
            });
            tx
        });

        let keyboard = Keyboard {
            layout,
            matrix: Arc::clone(&matrix),
            time_to_hide_overlay: Arc::clone(&time_to_hide_overlay),
            layer_state: Arc::clone(&layer_state),
            default_layer_state: Arc::clone(&default_layer_state),
            timeout_ms: Arc::clone(&timeout_ms),
            alive: Arc::clone(&alive),
            _keepalive: keepalive,
        };

        let layer_state_clone = Arc::clone(&keyboard.layer_state);
        let default_layer_state_clone = Arc::clone(&keyboard.default_layer_state);
        let time_to_hide_clone = Arc::clone(&keyboard.time_to_hide_overlay);
        let timeout_clone = Arc::clone(&keyboard.timeout_ms);
        let matrix_clone = Arc::clone(&matrix);
        let alive_clone = Arc::clone(&alive);

        thread::spawn(move || {
            // A dropped link (sleep, BLE/USB disconnect) makes `hid_read` error repeatedly.
            // Mark the connection dead after a few consecutive errors to trigger reconnect.
            const MAX_CONSECUTIVE_ERRORS: u32 = 5;
            let mut consecutive_errors: u32 = 0;

            loop {
                let response = match protocol.hid_read() {
                    Ok(response) => {
                        consecutive_errors = 0;
                        response
                    }
                    Err(_) => {
                        consecutive_errors += 1;
                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            alive_clone.store(false, Ordering::Relaxed);
                            ui_wake.request_repaint();
                            break;
                        }
                        thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                };

                let mut needs_repaint = false;
                if response[0] == 0xff {
                    let size = response[1] as usize;

                    // Not every 0xff packet is a layer packet: firmware without this module
                    // echoes our subscribe command back starting with 0xff. A real layer
                    // packet's length is sizeof(layer_state_t) (<=4), so skip anything else.
                    if size == 0 || size > MAX_LAYER_STATE_BYTES || 2 + 2 * size > response.len() {
                        continue;
                    }

                    let mut default_bytes = [0u8; 4];
                    default_bytes[..size].copy_from_slice(&response[2..2 + size]);
                    let default_layer_state = u32::from_le_bytes(default_bytes);

                    let mut layer_bytes = [0u8; 4];
                    layer_bytes[..size].copy_from_slice(&response[2 + size..2 + 2 * size]);
                    let layer_state = u32::from_le_bytes(layer_bytes);

                    if layer_state > 1 {
                        *time_to_hide_clone.lock().unwrap() = None;
                    } else {
                        let timeout = *timeout_clone.lock().unwrap();
                        if timeout < 0 {
                            *time_to_hide_clone.lock().unwrap() = None;
                        } else {
                            let time_to_hide =
                                Instant::now() + Duration::from_millis(timeout as u64);
                            *time_to_hide_clone.lock().unwrap() = Some(time_to_hide);
                        }
                    }

                    *layer_state_clone.lock().unwrap() = layer_state;
                    *default_layer_state_clone.lock().unwrap() = default_layer_state;
                    needs_repaint = true;
                } else if response[0] == 0xF1 {
                    let row = response[1] as usize;
                    let col = response[2] as usize;
                    let pressed = response[3];
                    if let Ok(mut mat) = matrix_clone.lock() {
                        mat.set_pressed(row, col, pressed != 0);
                    }
                    needs_repaint = time_to_hide_clone
                        .lock()
                        .unwrap()
                        .as_ref()
                        .is_none_or(|time_to_hide| Instant::now() < *time_to_hide);
                }

                if needs_repaint {
                    ui_wake.request_repaint();
                }
            }
        });

        Ok(keyboard)
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
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
                if !matrix.is_transparent(i, row, col) {
                    return (i as u8, is_active_default_layer && active_layer_above);
                }
            }
            active_layer_above |= is_active_momentary_layer;
        }

        (0, active_layer_above)
    }

    pub fn get_key(&self, layer: usize, row: usize, col: usize) -> Option<LayoutKey> {
        self.matrix
            .lock()
            .unwrap()
            .get_key(layer, row, col)
            .cloned()
    }

    pub fn is_key_pressed(&self, row: usize, col: usize) -> bool {
        self.matrix.lock().unwrap().is_pressed(row, col)
    }

    pub fn set_timeout(&self, timeout: i64) {
        *self.timeout_ms.lock().unwrap() = timeout;
    }

    pub fn set_layout(&mut self, layout: KeyboardLayout) {
        self.layout = layout;
    }
}
