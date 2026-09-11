use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::key_matrix::{BoundKey, KeyMatrix};
use crate::key_spec::{KeySpec, LayerInfo};
use crate::layout_key::LayoutKey;
use crate::protocols::{DeviceEvent, KeyboardLayout, KeyboardProtocol, WriteSupport};
use crate::ui_wake::UiWake;

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
    /// The protocol, shared with the command-execution thread. Capability
    /// queries lock it briefly; commands hold it for the duration of the
    /// protocol round-trip.
    protocol: Arc<Mutex<Box<dyn KeyboardProtocol>>>,
    layout: Mutex<KeyboardLayout>,
    overlay_visibility: Arc<Mutex<VisibilityWindow>>,
    matrix: Arc<Mutex<KeyMatrix>>,
    layer_state: Arc<Mutex<u32>>,
    default_layer_state: Arc<Mutex<u32>>,
    config: Arc<Mutex<OverlayConfig>>,
    alive: Arc<AtomicBool>,
    command_tx: mpsc::Sender<KeymapCommand>,
}

/// A keymap command for the protocol, executed on the reader thread so writes
/// and reads never race the same HID handle.
pub enum KeymapCommand {
    /// Opens the transient write session ahead of the first write (ZMK).
    OpenEditSession {
        respond: mpsc::Sender<Result<(), String>>,
    },
    SetKey {
        layer_index: usize,
        row: usize,
        col: usize,
        action: KeySpec,
        respond: mpsc::Sender<Result<(), String>>,
    },
    Save {
        respond: mpsc::Sender<Result<(), String>>,
    },
    /// Fire-and-forget; closes any transient write connection.
    EndEditSession,
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
        KeymapCommand::OpenEditSession { respond } => {
            let result = protocol.open_edit_session().map_err(|e| e.to_string());
            let _ = respond.send(result);
        }
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
                        .map_err(|e| e.to_string())
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
            let result = protocol.save_keymap().map_err(|e| e.to_string());
            let _ = respond.send(result);
        }
        KeymapCommand::EndEditSession => protocol.end_edit_session(),
    }
}

impl Keyboard {
    pub fn new(
        mut protocol: Box<dyn KeyboardProtocol>,
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

        let event_rx = protocol
            .subscribe_events()
            .map_err(|e| format!("Failed to subscribe to keyboard events: {e}"))?;

        let (command_tx, command_rx) = mpsc::channel::<KeymapCommand>();

        let layer_state = Arc::new(Mutex::new(0));
        let default_layer_state = Arc::new(Mutex::new(0));
        let overlay_visibility = Arc::new(Mutex::new(VisibilityWindow::hidden(Instant::now())));
        let config = Arc::new(Mutex::new(config));
        let matrix = Arc::new(Mutex::new(matrix));
        let alive = Arc::new(AtomicBool::new(true));
        let protocol = Arc::new(Mutex::new(protocol));

        let keyboard = Keyboard {
            protocol: Arc::clone(&protocol),
            layout: Mutex::new(layout),
            matrix: Arc::clone(&matrix),
            overlay_visibility: Arc::clone(&overlay_visibility),
            layer_state: Arc::clone(&layer_state),
            default_layer_state: Arc::clone(&default_layer_state),
            config: Arc::clone(&config),
            alive: Arc::clone(&alive),
            command_tx,
        };

        let layer_state_clone = Arc::clone(&keyboard.layer_state);
        let default_layer_state_clone = Arc::clone(&keyboard.default_layer_state);
        let visibility_clone = Arc::clone(&keyboard.overlay_visibility);
        let config_clone = Arc::clone(&keyboard.config);
        let matrix_clone = Arc::clone(&matrix);
        let alive_clone = Arc::clone(&alive);
        let ui_wake_events = ui_wake.clone();

        // 1. Live event consumer loop (pure domain logic)
        thread::spawn(move || {
            let mut previous_layers = ActiveLayers::Base;
            while let Ok(event) = event_rx.recv() {
                let needs_repaint = match event {
                    DeviceEvent::LayersChanged {
                        active_layers,
                        default_layers,
                    } => {
                        let config = *config_clone.lock().unwrap();
                        let active = ActiveLayers::classify(
                            active_layers,
                            default_layers,
                            config.visible_layers,
                        );
                        *layer_state_clone.lock().unwrap() = active_layers;
                        *default_layer_state_clone.lock().unwrap() = default_layers;

                        let mut visibility = visibility_clone.lock().unwrap();
                        *visibility = next_visibility_window(
                            active,
                            previous_layers,
                            *visibility,
                            Instant::now(),
                            config,
                        );
                        previous_layers = active;
                        true
                    }
                    DeviceEvent::KeyPressed { row, col, pressed } => {
                        if let Ok(mut mat) = matrix_clone.lock() {
                            mat.set_pressed(row, col, pressed);
                        }
                        visibility_clone.lock().unwrap().is_visible(Instant::now())
                    }
                    DeviceEvent::Disconnected(_) => {
                        alive_clone.store(false, Ordering::Relaxed);
                        ui_wake_events.request_repaint();
                        break;
                    }
                };

                if needs_repaint {
                    ui_wake_events.request_repaint();
                }
            }
            alive_clone.store(false, Ordering::Relaxed);
            ui_wake_events.request_repaint();
        });

        // 2. Command execution loop (runs writes on dedicated worker)
        let matrix_for_cmd = Arc::clone(&matrix);
        let ui_wake_cmd = ui_wake;
        thread::spawn(move || {
            while let Ok(command) = command_rx.recv() {
                let mut protocol = protocol.lock().unwrap();
                run_keymap_command(
                    protocol.as_mut(),
                    command,
                    &layer_names,
                    &matrix_for_cmd,
                    &ui_wake_cmd,
                );
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

    fn effective_layer_from_matrix(
        matrix: &KeyMatrix,
        layer_state: u32,
        default_layer_state: u32,
        row: usize,
        col: usize,
    ) -> (u8, bool) {
        let num_layers = matrix.get_num_layers().min(32);
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

    pub fn get_effective_key_layer(&self, row: usize, col: usize) -> (u8, bool) {
        let layer_state = *self.layer_state.lock().unwrap();
        let default_layer_state = *self.default_layer_state.lock().unwrap();
        let matrix = self.matrix.lock().unwrap();
        Self::effective_layer_from_matrix(&matrix, layer_state, default_layer_state, row, col)
    }

    pub fn get_key(&self, layer: usize, row: usize, col: usize) -> Option<LayoutKey> {
        self.matrix
            .lock()
            .unwrap()
            .get_key(layer, row, col)
            .cloned()
    }

    pub fn layer_infos(&self) -> Vec<LayerInfo> {
        self.matrix.lock().unwrap().layer_infos().to_vec()
    }

    pub fn get_action(&self, layer: usize, row: usize, col: usize) -> Option<KeySpec> {
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
        self.protocol.lock().unwrap().write_support()
    }

    /// Whether the device accepts raw firmware keycodes as hex input.
    pub fn supports_raw_keycode_entry(&self) -> bool {
        self.protocol.lock().unwrap().supports_raw_keycode_entry()
    }

    /// Parses a raw firmware keycode into a domain [`KeySpec`]. Only meaningful
    /// when [`Keyboard::supports_raw_keycode_entry`] is `true`.
    pub fn parse_raw_keycode(&self, code: u16) -> Option<KeySpec> {
        self.protocol.lock().unwrap().parse_raw_keycode(code)
    }

    /// Whether the layout can be switched while connected.
    pub fn supports_live_layout_switching(&self) -> bool {
        self.protocol.lock().unwrap().supports_live_layout_switching()
    }

    pub fn set_key(
        &self,
        layer_index: usize,
        row: usize,
        col: usize,
        action: KeySpec,
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

    /// Opens the write session in the background to prepare for key editing.
    pub fn open_edit_session(&self) -> mpsc::Receiver<Result<(), String>> {
        self.send_keymap_command(|respond| KeymapCommand::OpenEditSession { respond })
    }

    /// Closes any active edit session on the keyboard protocol.
    pub fn end_edit_session(&self) {
        let _ = self.command_tx.send(KeymapCommand::EndEditSession);
    }

    /// Sends a command to the keyboard communication thread and returns a response receiver.
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
                | KeymapCommand::OpenEditSession { respond } => {
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
        let guard = self.layout.lock().unwrap();
        let matrix = self.matrix.lock().unwrap();
        let layer_state = *self.layer_state.lock().unwrap();
        let default_layer_state = *self.default_layer_state.lock().unwrap();

        guard.keys.iter().fold(0u16, |acc, key| {
            if !matrix.is_pressed(key.row, key.col) {
                return acc;
            }
            let (effective_layer, _) = Self::effective_layer_from_matrix(
                &matrix,
                layer_state,
                default_layer_state,
                key.row,
                key.col,
            );
            let mask = matrix
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

    pub fn is_action_supported(&self, action: &KeySpec) -> bool {
        let filter = self.protocol.lock().unwrap().action_filter();
        filter.is_none_or(|filter| filter(action))
    }

    pub fn set_config(&self, config: OverlayConfig) {
        *self.config.lock().unwrap() = config;
    }

    pub fn layout(&self) -> KeyboardLayout {
        self.layout.lock().unwrap().clone()
    }

    pub fn set_layout(&self, layout: KeyboardLayout) {
        *self.layout.lock().unwrap() = layout;
    }
}

#[cfg(test)]
mod tests {
    use super::ActiveLayers::{Base, Excluded, Selected};
    use super::*;

    fn mock_keyboard() -> Keyboard {
        let protocol = Box::new(crate::protocols::mock::MockProtocol::connect().unwrap());
        let layout_name = protocol.get_layout_definition().layouts[0].name.clone();
        Keyboard::new(
            protocol,
            layout_name,
            CONFIG,
            UiWake::new(Arc::new(|| ())),
        )
        .unwrap()
    }

    /// Capability queries delegate live to the protocol instead of being
    /// snapshotted at construction.
    #[test]
    fn keyboard_delegates_raw_keycode_capabilities() {
        let keyboard = mock_keyboard();
        assert!(keyboard.supports_raw_keycode_entry());
        assert_eq!(
            keyboard.parse_raw_keycode(0x0004),
            Some(KeySpec::KeyPress {
                key: crate::key_spec::HidKey::keyboard(0x04),
                modifiers: Default::default(),
            })
        );
    }

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
