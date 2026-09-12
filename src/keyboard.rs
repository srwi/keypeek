use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::key_matrix::KeyMatrix;
use crate::key_spec::{KeySpec, LayerInfo};
use crate::layout_key::LayoutKey;
use crate::protocols::{KeyboardLayout, KeyboardProtocol, WriteSupport};
use crate::session::KeyboardSession;
use crate::ui_wake::UiWake;
use crate::visibility::VisibilityStateMachine;

pub use crate::visibility::OverlayConfig;

/// Pure domain aggregate holding keyboard layout, matrix bindings, layer states,
/// and overlay visibility. Has no background threads or communication channels.
pub struct KeyboardDomain {
    layout: Mutex<KeyboardLayout>,
    matrix: Mutex<KeyMatrix>,
    layer_state: Mutex<u32>,
    default_layer_state: Mutex<u32>,
    visibility: Mutex<VisibilityStateMachine>,
}

impl KeyboardDomain {
    pub fn new(layout: KeyboardLayout, matrix: KeyMatrix, config: OverlayConfig) -> Self {
        Self {
            layout: Mutex::new(layout),
            matrix: Mutex::new(matrix),
            layer_state: Mutex::new(0),
            default_layer_state: Mutex::new(0),
            visibility: Mutex::new(VisibilityStateMachine::new(config, Instant::now())),
        }
    }

    // --- State update methods (driven by KeyboardSession) ---

    pub fn on_layers_changed(&self, active_layers: u32, default_layers: u32, now: Instant) -> bool {
        *self.layer_state.lock().unwrap() = active_layers;
        *self.default_layer_state.lock().unwrap() = default_layers;
        self.visibility
            .lock()
            .unwrap()
            .on_layers_changed(active_layers, default_layers, now)
    }

    pub fn on_key_pressed(&self, row: usize, col: usize, pressed: bool, now: Instant) -> bool {
        if let Ok(mut mat) = self.matrix.lock() {
            mat.set_pressed(row, col, pressed);
        }
        self.visibility.lock().unwrap().is_visible(now)
    }

    pub fn layer_info(&self, index: usize) -> Option<LayerInfo> {
        self.matrix.lock().unwrap().layer_infos().get(index).cloned()
    }

    pub fn update_binding(
        &self,
        layer: usize,
        row: usize,
        col: usize,
        action: KeySpec,
        label: Option<LayoutKey>,
    ) {
        if let Ok(mut mat) = self.matrix.lock() {
            mat.update_binding(layer, row, col, action, label);
        }
    }

    // --- Domain queries ---

    pub fn overlay_is_visible(&self, now: Instant) -> bool {
        self.visibility.lock().unwrap().is_visible(now)
    }

    /// How long until the overlay appears or disappears on its own, for scheduling a repaint.
    pub fn overlay_changes_in(&self, now: Instant) -> Option<Duration> {
        self.visibility.lock().unwrap().changes_in(now)
    }

    pub fn get_effective_key_layer(&self, row: usize, col: usize) -> (u8, bool) {
        let layer_state = *self.layer_state.lock().unwrap();
        let default_layer_state = *self.default_layer_state.lock().unwrap();
        let matrix = self.matrix.lock().unwrap();
        matrix.effective_layer(layer_state, default_layer_state, row, col)
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

    pub fn held_mod_mask(&self) -> u16 {
        let guard = self.layout.lock().unwrap();
        let matrix = self.matrix.lock().unwrap();
        let layer_state = *self.layer_state.lock().unwrap();
        let default_layer_state = *self.default_layer_state.lock().unwrap();
        matrix.held_mod_mask(&guard.keys, layer_state, default_layer_state)
    }

    pub fn is_shift_held(&self) -> bool {
        self.held_mod_mask() & crate::layout_key::HELD_MOD_SHIFT != 0
    }

    pub fn is_ralt_held(&self) -> bool {
        self.held_mod_mask() & crate::layout_key::HELD_MOD_RALT != 0
    }

    pub fn set_config(&self, config: OverlayConfig) {
        self.visibility.lock().unwrap().set_config(config);
    }

    pub fn layout(&self) -> KeyboardLayout {
        self.layout.lock().unwrap().clone()
    }

    pub fn set_layout(&self, layout: KeyboardLayout) {
        *self.layout.lock().unwrap() = layout;
    }
}

/// Root connected keyboard aggregate, uniting pure in-memory domain state
/// (`KeyboardDomain`) with asynchronous communication and background threads
/// (`KeyboardSession`).
pub struct Keyboard {
    domain: Arc<KeyboardDomain>,
    session: KeyboardSession,
}

impl Keyboard {
    pub fn new(
        protocol: Box<dyn KeyboardProtocol>,
        layout_name: String,
        config: OverlayConfig,
        ui_wake: UiWake,
        presenter: Arc<dyn crate::key_presenter::KeyPresenter>,
    ) -> Result<Self, String> {
        let definition = protocol.get_layout_definition();

        let layout = definition
            .get_layout(&layout_name)
            .map_err(|_| "Failed to get layout".to_string())?;

        let snapshot = protocol
            .read_keymap()
            .map_err(|e| format!("Failed to read keymap: {e}"))?;

        let layer_names: Vec<String> = snapshot
            .layers
            .iter()
            .map(|l| l.name.clone().unwrap_or_default())
            .collect();

        let matrix = KeyMatrix::from_snapshot(
            snapshot,
            definition.rows,
            definition.cols,
            presenter.as_ref(),
        );

        let domain = Arc::new(KeyboardDomain::new(layout, matrix, config));
        let session = KeyboardSession::start(
            protocol,
            Arc::clone(&domain),
            layer_names,
            presenter,
            ui_wake,
        )?;

        Ok(Keyboard { domain, session })
    }

    #[allow(dead_code)]
    pub fn domain(&self) -> &KeyboardDomain {
        &self.domain
    }

    #[allow(dead_code)]
    pub fn session(&self) -> &KeyboardSession {
        &self.session
    }

    // --- Session delegations ---

    pub fn is_alive(&self) -> bool {
        self.session.is_alive()
    }

    pub fn write_support(&self) -> WriteSupport {
        self.session.write_support()
    }

    pub fn supports_live_layout_switching(&self) -> bool {
        self.session.supports_live_layout_switching()
    }

    pub fn is_action_supported(&self, action: &KeySpec) -> bool {
        self.session.is_action_supported(action)
    }

    pub fn set_key(
        &self,
        layer_index: usize,
        row: usize,
        col: usize,
        action: KeySpec,
    ) -> mpsc::Receiver<Result<(), String>> {
        self.session.set_key(layer_index, row, col, action)
    }

    pub fn save_keymap(&self) -> mpsc::Receiver<Result<(), String>> {
        self.session.save_keymap()
    }

    pub fn acquire_edit_lock(&self) -> mpsc::Receiver<Result<(), String>> {
        self.session.acquire_edit_lock()
    }

    pub fn release_edit_lock(&self) {
        self.session.release_edit_lock();
    }

    // --- Domain delegations ---

    pub fn overlay_is_visible(&self, now: Instant) -> bool {
        self.domain.overlay_is_visible(now)
    }

    pub fn overlay_changes_in(&self, now: Instant) -> Option<Duration> {
        self.domain.overlay_changes_in(now)
    }

    pub fn get_effective_key_layer(&self, row: usize, col: usize) -> (u8, bool) {
        self.domain.get_effective_key_layer(row, col)
    }

    pub fn get_key(&self, layer: usize, row: usize, col: usize) -> Option<LayoutKey> {
        self.domain.get_key(layer, row, col)
    }

    pub fn layer_infos(&self) -> Vec<LayerInfo> {
        self.domain.layer_infos()
    }

    pub fn get_action(&self, layer: usize, row: usize, col: usize) -> Option<KeySpec> {
        self.domain.get_action(layer, row, col)
    }

    pub fn is_key_pressed(&self, row: usize, col: usize) -> bool {
        self.domain.is_key_pressed(row, col)
    }

    pub fn is_shift_held(&self) -> bool {
        self.domain.is_shift_held()
    }

    pub fn is_ralt_held(&self) -> bool {
        self.domain.is_ralt_held()
    }

    pub fn set_config(&self, config: OverlayConfig) {
        self.domain.set_config(config);
    }

    pub fn layout(&self) -> KeyboardLayout {
        self.domain.layout()
    }

    pub fn set_layout(&self, layout: KeyboardLayout) {
        self.domain.set_layout(layout);
    }
}
