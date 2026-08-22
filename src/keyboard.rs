use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering};
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
/// Leading byte of a layer-state packet, followed by a size and two bitmasks.
const LAYER_STATE_PACKET: u8 = 0xff;
/// Leading byte of a key event packet, followed by `row`, `col`, `pressed`.
const KEY_EVENT_PACKET: u8 = 0xF1;

/// The active layers as seen through the visible-layer bitmask (bit `i` selects layer
/// `i`; see `Settings::visible_layers`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ActiveLayers {
    /// A selected layer above the base layer is held.
    Selected,
    /// An active layer is masked out.
    Excluded,
    /// Nothing is masked out and no selected layer is held, so the timeout decides how
    /// long the overlay lingers.
    Base,
}

impl ActiveLayers {
    fn classify(layer_state: u32, default_layer_state: u32, visible_layers: u32) -> Self {
        // The base layer is always active underneath the momentary and default layers.
        let active = layer_state | default_layer_state | 1;

        // Holding the base layer is not a reason to keep the overlay up; the timeout
        // governs that instead, so it never counts as a selected layer.
        let held_visible = layer_state & visible_layers & !1 != 0;
        let any_hidden = active & !visible_layers != 0;
        match (held_visible, any_hidden) {
            (true, _) => Self::Selected,
            (_, true) => Self::Excluded,
            _ => Self::Base,
        }
    }
}

/// How long the overlay stays visible; `None` keeps it visible until the layer state
/// changes, `Duration::ZERO` hides it right away.
fn overlay_visible_duration(
    active: ActiveLayers,
    previous: ActiveLayers,
    timeout_ms: i64,
) -> Option<Duration> {
    match active {
        ActiveLayers::Selected => None,
        ActiveLayers::Excluded => Some(Duration::ZERO),
        ActiveLayers::Base => {
            if timeout_ms < 0 {
                None
            } else if previous == ActiveLayers::Excluded {
                // Leaving an excluded layer must not surface the base layer.
                Some(Duration::ZERO)
            } else {
                Some(Duration::from_millis(timeout_ms as u64))
            }
        }
    }
}

pub struct Keyboard {
    pub layout: KeyboardLayout,
    pub time_to_hide_overlay: Arc<Mutex<Option<Instant>>>,
    matrix: Arc<Mutex<KeyMatrix>>,
    layer_state: Arc<Mutex<u32>>,
    default_layer_state: Arc<Mutex<u32>>,
    timeout_ms: Arc<AtomicI64>,
    visible_layers: Arc<AtomicU32>,
    alive: Arc<AtomicBool>,
    _keepalive: Option<mpsc::Sender<()>>,
}

impl Keyboard {
    pub fn new(
        protocol: Box<dyn KeyboardProtocol>,
        layout_name: String,
        timeout: i64,
        visible_layers: u32,
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
        let timeout_ms = Arc::new(AtomicI64::new(timeout));
        let visible_layers = Arc::new(AtomicU32::new(visible_layers));
        let matrix = Arc::new(Mutex::new(matrix));
        let alive = Arc::new(AtomicBool::new(true));

        let keepalive = protocol
            .subscription_sender()
            .map_err(|e| e.to_string())?
            .map(|sender| {
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
            visible_layers: Arc::clone(&visible_layers),
            alive: Arc::clone(&alive),
            _keepalive: keepalive,
        };

        let layer_state_clone = Arc::clone(&keyboard.layer_state);
        let default_layer_state_clone = Arc::clone(&keyboard.default_layer_state);
        let time_to_hide_clone = Arc::clone(&keyboard.time_to_hide_overlay);
        let timeout_clone = Arc::clone(&keyboard.timeout_ms);
        let visible_layers_clone = Arc::clone(&keyboard.visible_layers);
        let matrix_clone = Arc::clone(&matrix);
        let alive_clone = Arc::clone(&alive);

        thread::spawn(move || {
            // A dropped link (sleep, BLE/USB disconnect) makes `hid_read` error repeatedly.
            // Mark the connection dead after a few consecutive errors to trigger reconnect.
            const MAX_CONSECUTIVE_ERRORS: u32 = 5;
            let mut consecutive_errors: u32 = 0;
            let mut previous_layers = ActiveLayers::Base;

            loop {
                let response = match protocol.hid_read() {
                    Ok(response) if response.is_empty() => continue,
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
                match response.first().copied() {
                    Some(LAYER_STATE_PACKET) if response.len() >= 2 => {
                        let size = response[1] as usize;

                        // Not every 0xff packet is a layer packet: firmware without this module
                        // echoes our subscribe command back starting with 0xff. A real layer
                        // packet's length is sizeof(layer_state_t) (<=4), so skip anything else.
                        if size == 0
                            || size > MAX_LAYER_STATE_BYTES
                            || 2 + 2 * size > response.len()
                        {
                            continue;
                        }

                        let mut default_bytes = [0u8; 4];
                        default_bytes[..size].copy_from_slice(&response[2..2 + size]);
                        let default_layer_state = u32::from_le_bytes(default_bytes);

                        let mut layer_bytes = [0u8; 4];
                        layer_bytes[..size].copy_from_slice(&response[2 + size..2 + 2 * size]);
                        let layer_state = u32::from_le_bytes(layer_bytes);

                        let active_layers = ActiveLayers::classify(
                            layer_state,
                            default_layer_state,
                            visible_layers_clone.load(Ordering::Relaxed),
                        );
                        let visible_for = overlay_visible_duration(
                            active_layers,
                            previous_layers,
                            timeout_clone.load(Ordering::Relaxed),
                        );
                        previous_layers = active_layers;
                        *time_to_hide_clone.lock().unwrap() =
                            visible_for.map(|duration| Instant::now() + duration);

                        *layer_state_clone.lock().unwrap() = layer_state;
                        *default_layer_state_clone.lock().unwrap() = default_layer_state;
                        needs_repaint = true;
                    }
                    Some(KEY_EVENT_PACKET) if response.len() >= 4 => {
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
                    _ => {}
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
            if (is_active_momentary_layer || is_active_default_layer)
                && !matrix.is_transparent(i, row, col)
            {
                return (i as u8, is_active_default_layer && active_layer_above);
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

    /// `HELD_MOD_SHIFT`/`HELD_MOD_RALT` bits OR'd over every currently-pressed
    /// key's `mod_mask` — used by the Single-legend live preview to detect
    /// "Shift/RAlt is held right now", regardless of which specific key
    /// (dedicated Shift key, home-row mod, One-Shot-Mod, ...) is holding it.
    fn held_mod_mask(&self) -> u16 {
        self.layout.keys.iter().fold(0u16, |acc, key| {
            if !self.is_key_pressed(key.row, key.col) {
                return acc;
            }
            let (effective_layer, _) = self.get_effective_key_layer(key.row, key.col);
            let mask = self
                .get_key(effective_layer as usize, key.row, key.col)
                .and_then(|k| k.mod_mask)
                .unwrap_or(0);
            acc | mask
        })
    }

    pub fn is_shift_held(&self) -> bool {
        self.held_mod_mask() & crate::layout_key::HELD_MOD_SHIFT != 0
    }

    pub fn is_ralt_held(&self) -> bool {
        self.held_mod_mask() & crate::layout_key::HELD_MOD_RALT != 0
    }

    pub fn set_timeout(&self, timeout: i64) {
        self.timeout_ms.store(timeout, Ordering::Relaxed);
    }

    pub fn set_visible_layers(&self, visible_layers: u32) {
        self.visible_layers.store(visible_layers, Ordering::Relaxed);
    }

    pub fn set_layout(&mut self, layout: KeyboardLayout) {
        self.layout = layout;
    }
}
