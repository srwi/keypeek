use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::key_action::{KeyAction, KeymapSnapshot};
use crate::key_matrix::{BoundKey, KeyMatrix};
use crate::layout_key::LayoutKey;
use crate::protocols::{DeviceLocked, KeyboardLayout, KeyboardProtocol, WriteSupport};
use crate::ui_wake::UiWake;
use std::error::Error;

/// A layer packet's size field is `sizeof(layer_state_t)` and at most 4 bytes.
const MAX_LAYER_STATE_BYTES: usize = 4;
/// Leading byte of a layer-state packet, followed by a size and two bitmasks.
const LAYER_STATE_PACKET: u8 = 0xff;
/// Leading byte of a key event packet, followed by `row`, `col`, `pressed`.
const KEY_EVENT_PACKET: u8 = 0xF1;

/// A `0xff`-led packet is only a real layer-state packet when its size field is
/// `sizeof(layer_state_t)` and both bitmasks fit; firmware without this module
/// echoes other packets with the same leading byte.
fn is_layer_state_packet(response: &[u8]) -> bool {
    let size = response[1] as usize;
    size != 0 && size <= MAX_LAYER_STATE_BYTES && 2 + 2 * size <= response.len()
}

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

/// The overlay's tuning knobs, all changeable while connected.
#[derive(Clone, Copy)]
pub struct OverlayConfig {
    /// How long the overlay lingers once no selected layer is held; negative never hides.
    pub timeout_ms: i64,
    /// How long a layer has to be held before the overlay appears.
    pub activation_delay_ms: u32,
    /// Bit `i` keeps the overlay up while layer `i` is active; see `ActiveLayers`.
    pub visible_layers: u32,
}

/// The stretch of time the overlay is shown for: from `from` until `until`, where `None`
/// keeps it up until the layer state changes again.
#[derive(Clone, Copy)]
struct VisibilityWindow {
    from: Instant,
    until: Option<Instant>,
}

impl VisibilityWindow {
    /// An empty window, keeping the overlay hidden until the next layer state arrives.
    fn hidden(now: Instant) -> Self {
        Self {
            from: now,
            until: Some(now),
        }
    }

    fn is_visible(&self, now: Instant) -> bool {
        now >= self.from && self.until.is_none_or(|until| now < until)
    }

    /// How long until the overlay appears or disappears on its own.
    fn changes_in(&self, now: Instant) -> Option<Duration> {
        let next = if now < self.from {
            Some(self.from)
        } else {
            self.until
        };
        next.filter(|at| now < *at).map(|at| at - now)
    }
}

/// The window a freshly arrived layer state puts the overlay in.
fn next_visibility_window(
    active: ActiveLayers,
    previous: ActiveLayers,
    current: VisibilityWindow,
    now: Instant,
    config: OverlayConfig,
) -> VisibilityWindow {
    // A held layer whose activation delay has not elapsed yet.
    let pending = now < current.from;

    match active {
        ActiveLayers::Selected => VisibilityWindow {
            // A window still arming or already up keeps its start: layers added mid-hold
            // must not restart the countdown, nor blink a visible overlay away.
            from: if pending || current.is_visible(now) {
                current.from
            } else {
                now + Duration::from_millis(config.activation_delay_ms as u64)
            },
            until: None,
        },
        ActiveLayers::Excluded => VisibilityWindow::hidden(now),
        // Neither leaving an excluded layer nor releasing a layer before its activation
        // delay elapsed may surface the base layer.
        ActiveLayers::Base if previous == ActiveLayers::Excluded || pending => {
            VisibilityWindow::hidden(now)
        }
        ActiveLayers::Base => VisibilityWindow {
            from: now,
            until: (config.timeout_ms >= 0)
                .then(|| now + Duration::from_millis(config.timeout_ms as u64)),
        },
    }
}

pub struct Keyboard {
    pub layout: KeyboardLayout,
    overlay_visibility: Arc<Mutex<VisibilityWindow>>,
    matrix: Arc<Mutex<KeyMatrix>>,
    layer_state: Arc<Mutex<u32>>,
    default_layer_state: Arc<Mutex<u32>>,
    config: Arc<Mutex<OverlayConfig>>,
    alive: Arc<AtomicBool>,
    command_tx: mpsc::Sender<KeymapCommand>,
    write_support: WriteSupport,
    _keepalive: Option<mpsc::Sender<()>>,
}

/// A keymap write request for the protocol, executed on the reader thread so
/// writes and reads never race the same HID handle.
pub enum KeymapCommand {
    SetKey {
        layer_index: usize,
        row: usize,
        col: usize,
        action: KeyAction,
        respond: mpsc::Sender<Result<(), String>>,
    },
    Save {
        respond: mpsc::Sender<Result<(), String>>,
    },
    Discard {
        respond: mpsc::Sender<Result<(), String>>,
    },
    /// Fire-and-forget; closes any transient write connection.
    EndEditSession,
}

/// The error text shown for a failed write. Locked ZMK devices get a
/// retryable message instead of the raw RPC error.
fn write_error_text(error: Box<dyn Error>) -> String {
    if error.is::<DeviceLocked>() {
        "Device is locked. Press the ZMK Studio unlock key combination on your keyboard, \
         then try again."
            .to_string()
    } else {
        error.to_string()
    }
}

/// Replaces the matrix content from a fresh snapshot while keeping per-key
/// pressed state (a discard must not clear keys the user is holding).
fn replace_matrix_content(matrix: &Arc<Mutex<KeyMatrix>>, snapshot: KeymapSnapshot) {
    let mut guard = matrix.lock().unwrap();
    let pressed = guard.pressed.clone();
    let rows = pressed.len();
    let cols = pressed.first().map_or(0, Vec::len);
    let mut replacement = KeyMatrix::from_snapshot(snapshot, rows, cols);
    replacement.pressed = pressed;
    *guard = replacement;
}

/// Executes one command on the protocol. Runs on the reader thread.
fn run_keymap_command(
    protocol: &mut dyn KeyboardProtocol,
    command: KeymapCommand,
    layer_names: &[String],
    matrix: &Arc<Mutex<KeyMatrix>>,
    ui_wake: &UiWake,
) {
    match command {
        KeymapCommand::SetKey {
            layer_index,
            row,
            col,
            action,
            respond,
        } => {
            let layer_info = matrix
                .lock()
                .unwrap()
                .layer_infos()
                .get(layer_index)
                .cloned();

            let result = layer_info
                .ok_or_else(|| format!("Unknown layer index {layer_index}"))
                .and_then(|layer| {
                    protocol
                        .set_key(&layer, layer_index, row, col, &action)
                        .map_err(write_error_text)
                });

            if result.is_ok() {
                let label = action.resolve_label(layer_names);
                let mut guard = matrix.lock().unwrap();
                if let Some(cell) = guard
                    .keys
                    .get_mut(layer_index)
                    .and_then(|layer| layer.get_mut(row))
                    .and_then(|r| r.get_mut(col))
                {
                    *cell = Some(BoundKey { label, action });
                }
                drop(guard);
                ui_wake.request_repaint();
            }
            let _ = respond.send(result);
        }
        KeymapCommand::Save { respond } => {
            let result = protocol.save_keymap().map_err(write_error_text);
            let _ = respond.send(result);
        }
        KeymapCommand::Discard { respond } => match protocol.discard_keymap() {
            Ok(snapshot) => {
                replace_matrix_content(matrix, snapshot);
                ui_wake.request_repaint();
                let _ = respond.send(Ok(()));
            }
            Err(e) => {
                let _ = respond.send(Err(write_error_text(e)));
            }
        },
        KeymapCommand::EndEditSession => protocol.end_edit_session(),
    }
}

impl Keyboard {
    pub fn new(
        protocol: Box<dyn KeyboardProtocol>,
        layout_name: String,
        config: OverlayConfig,
        ui_wake: UiWake,
    ) -> Result<Self, String> {
        let definition = protocol.get_layout_definition();

        let layout = definition
            .get_layout(&layout_name)
            .map_err(|_| "Failed to get layout".to_string())?;

        let snapshot = protocol
            .read_keymap()
            .map_err(|e| format!("Failed to read keymap: {e}"))?;
        // Kept outside the matrix so command execution can resolve labels for
        // freshly written actions without locking it.
        let layer_names: Vec<String> = snapshot
            .layers
            .iter()
            .map(|l| l.name.clone().unwrap_or_default())
            .collect();
        let matrix = KeyMatrix::from_snapshot(snapshot, definition.rows, definition.cols);

        let write_support = protocol.write_support();
        let (command_tx, command_rx) = mpsc::channel::<KeymapCommand>();

        let layer_state = Arc::new(Mutex::new(0));
        let default_layer_state = Arc::new(Mutex::new(0));
        let overlay_visibility = Arc::new(Mutex::new(VisibilityWindow::hidden(Instant::now())));
        let config = Arc::new(Mutex::new(config));
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
            overlay_visibility: Arc::clone(&overlay_visibility),
            layer_state: Arc::clone(&layer_state),
            default_layer_state: Arc::clone(&default_layer_state),
            config: Arc::clone(&config),
            alive: Arc::clone(&alive),
            command_tx,
            write_support,
            _keepalive: keepalive,
        };

        let layer_state_clone = Arc::clone(&keyboard.layer_state);
        let default_layer_state_clone = Arc::clone(&keyboard.default_layer_state);
        let visibility_clone = Arc::clone(&keyboard.overlay_visibility);
        let config_clone = Arc::clone(&keyboard.config);
        let matrix_clone = Arc::clone(&matrix);
        let alive_clone = Arc::clone(&alive);

        thread::spawn(move || {
            let mut protocol = protocol;
            // A dropped link (sleep, BLE/USB disconnect) makes `hid_read` error repeatedly.
            // Mark the connection dead after a few consecutive errors to trigger reconnect.
            const MAX_CONSECUTIVE_ERRORS: u32 = 5;
            let mut consecutive_errors: u32 = 0;
            let mut previous_layers = ActiveLayers::Base;

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
                match response.first().copied() {
                    Some(LAYER_STATE_PACKET)
                        if response.len() >= 2 && is_layer_state_packet(&response) =>
                    {
                        let size = response[1] as usize;

                        let mut default_bytes = [0u8; 4];
                        default_bytes[..size].copy_from_slice(&response[2..2 + size]);
                        let default_layer_state = u32::from_le_bytes(default_bytes);

                        let mut layer_bytes = [0u8; 4];
                        layer_bytes[..size].copy_from_slice(&response[2 + size..2 + 2 * size]);
                        let layer_state = u32::from_le_bytes(layer_bytes);

                        let config = *config_clone.lock().unwrap();
                        let active_layers = ActiveLayers::classify(
                            layer_state,
                            default_layer_state,
                            config.visible_layers,
                        );
                        *layer_state_clone.lock().unwrap() = layer_state;
                        *default_layer_state_clone.lock().unwrap() = default_layer_state;

                        let mut visibility = visibility_clone.lock().unwrap();
                        *visibility = next_visibility_window(
                            active_layers,
                            previous_layers,
                            *visibility,
                            Instant::now(),
                            config,
                        );
                        previous_layers = active_layers;
                        needs_repaint = true;
                    }
                    Some(KEY_EVENT_PACKET) if response.len() >= 4 => {
                        let row = response[1] as usize;
                        let col = response[2] as usize;
                        let pressed = response[3];
                        if let Ok(mut mat) = matrix_clone.lock() {
                            mat.set_pressed(row, col, pressed != 0);
                        }
                        needs_repaint = visibility_clone.lock().unwrap().is_visible(Instant::now());
                    }
                    _ => {}
                }

                if needs_repaint {
                    ui_wake.request_repaint();
                }

                // Commands run once per loop iteration, after the read: writes
                // and reads never race the same HID handle, and a command waits
                // at most one `hid_read` timeout.
                loop {
                    match command_rx.try_recv() {
                        Ok(command) => run_keymap_command(
                            protocol.as_mut(),
                            command,
                            &layer_names,
                            &matrix_clone,
                            &ui_wake,
                        ),
                        Err(_) => break,
                    }
                }
            }
        });

        Ok(keyboard)
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn overlay_is_visible(&self, now: Instant) -> bool {
        self.overlay_visibility.lock().unwrap().is_visible(now)
    }

    /// How long until the overlay appears or disappears on its own, for scheduling a repaint.
    pub fn overlay_changes_in(&self, now: Instant) -> Option<Duration> {
        self.overlay_visibility.lock().unwrap().changes_in(now)
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

    pub fn layer_infos(&self) -> Vec<crate::key_action::LayerInfo> {
        self.matrix.lock().unwrap().layer_infos().to_vec()
    }

    pub fn get_action(
        &self,
        layer: usize,
        row: usize,
        col: usize,
    ) -> Option<crate::key_action::KeyAction> {
        self.matrix
            .lock()
            .unwrap()
            .get_action(layer, row, col)
            .cloned()
    }

    pub fn is_key_pressed(&self, row: usize, col: usize) -> bool {
        self.matrix.lock().unwrap().is_pressed(row, col)
    }

    pub fn write_support(&self) -> WriteSupport {
        self.write_support
    }

    pub fn set_key(
        &self,
        layer_index: usize,
        row: usize,
        col: usize,
        action: KeyAction,
    ) -> mpsc::Receiver<Result<(), String>> {
        self.send_keymap_command(|respond| KeymapCommand::SetKey {
            layer_index,
            row,
            col,
            action,
            respond,
        })
    }

    pub fn save_keymap(&self) -> mpsc::Receiver<Result<(), String>> {
        self.send_keymap_command(|respond| KeymapCommand::Save { respond })
    }

    pub fn discard_keymap(&self) -> mpsc::Receiver<Result<(), String>> {
        self.send_keymap_command(|respond| KeymapCommand::Discard { respond })
    }

    /// Fire-and-forget: closes any transient write connection on the protocol.
    pub fn end_edit_session(&self) {
        let _ = self.command_tx.send(KeymapCommand::EndEditSession);
    }

    /// Queues a command for the reader thread and returns the receiver for its
    /// result. If the thread is gone (connection lost), the receiver carries an
    /// error instead of hanging.
    fn send_keymap_command(
        &self,
        build: impl FnOnce(mpsc::Sender<Result<(), String>>) -> KeymapCommand,
    ) -> mpsc::Receiver<Result<(), String>> {
        let (respond, receiver) = mpsc::channel();
        let command = build(respond);

        if let Err(send_error) = self.command_tx.send(command) {
            match send_error.0 {
                KeymapCommand::SetKey { respond, .. }
                | KeymapCommand::Save { respond }
                | KeymapCommand::Discard { respond } => {
                    let _ = respond.send(Err("Connection lost".to_string()));
                }
                KeymapCommand::EndEditSession => {}
            }
        }
        receiver
    }

    /// `HELD_MOD_SHIFT`/`HELD_MOD_RALT` bits OR'd over every pressed key's
    /// `mod_mask`. The Single-legend live preview uses this to detect "Shift/
    /// RAlt is held right now", no matter which key holds it (dedicated key,
    /// home-row mod, One-Shot-Mod, ...).
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

    pub fn set_config(&self, config: OverlayConfig) {
        *self.config.lock().unwrap() = config;
    }

    pub fn set_layout(&mut self, layout: KeyboardLayout) {
        self.layout = layout;
    }
}

#[cfg(test)]
mod tests {
    use super::ActiveLayers::{Base, Excluded, Selected};
    use super::*;

    const CONFIG: OverlayConfig = OverlayConfig {
        timeout_ms: 2000,
        activation_delay_ms: 300,
        visible_layers: u32::MAX,
    };

    /// Walks the layer-state transitions the activation delay has to survive.
    #[test]
    fn activation_delay_gates_the_overlay() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let hidden = VisibilityWindow::hidden(start);

        // Holding a layer arms the delay; the overlay only shows once it has elapsed.
        let held = next_visibility_window(Selected, Base, hidden, start, CONFIG);
        assert!(!held.is_visible(at(299)));
        assert!(held.is_visible(at(300)));
        assert_eq!(held.changes_in(start), Some(Duration::from_millis(300)));

        // A second layer added mid-hold keeps the original countdown.
        let more = next_visibility_window(Selected, Selected, held, at(200), CONFIG);
        assert!(more.is_visible(at(300)));

        // Releasing before the delay elapsed shows nothing at all.
        let tapped = next_visibility_window(Base, Selected, held, at(100), CONFIG);
        assert!(!tapped.is_visible(at(100)));

        // Releasing after it elapsed lingers for the display duration.
        let released = next_visibility_window(Base, Selected, held, at(400), CONFIG);
        assert!(released.is_visible(at(2399)));
        assert!(!released.is_visible(at(2400)));

        // A layer held while the overlay is still up must not blink it away.
        let again = next_visibility_window(Selected, Base, released, at(500), CONFIG);
        assert!(again.is_visible(at(500)));
    }

    /// Without a delay the overlay behaves as it does with the feature turned off.
    #[test]
    fn zero_delay_shows_the_overlay_right_away() {
        let start = Instant::now();
        let hidden = VisibilityWindow::hidden(start);
        let no_delay = OverlayConfig {
            activation_delay_ms: 0,
            ..CONFIG
        };

        let held = next_visibility_window(Selected, Base, hidden, start, no_delay);
        assert!(held.is_visible(start));
        assert_eq!(held.changes_in(start), None);

        // Leaving an excluded layer still must not surface the base layer.
        let excluded = next_visibility_window(Excluded, Selected, held, start, no_delay);
        assert!(!excluded.is_visible(start));
        let base = next_visibility_window(Base, Excluded, excluded, start, no_delay);
        assert!(!base.is_visible(start));
    }
}
