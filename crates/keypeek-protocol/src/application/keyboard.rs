//! Runtime keyboard facade pairing pure domain state with background communication session.

use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;

use crate::application::session::KeyboardSession;
use keypeek_core::{
    KeyMatrix, KeyPresenter, KeySpec, KeyboardDomain, KeyboardLayout, LayerInfo, LayoutKey,
    OverlayConfig,
};
use crate::protocols::{KeyboardProtocol, WriteSupport};
use crate::ui_wake::UiWake;

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
        presenter: Arc<dyn KeyPresenter>,
    ) -> Result<Self, String> {
        let definition = protocol.get_layout_definition().clone();

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

        let domain = Arc::new(KeyboardDomain::new(
            definition,
            layout_name,
            layout,
            matrix,
            config,
        ));
        let session = KeyboardSession::start(
            protocol,
            Arc::clone(&domain),
            layer_names,
            presenter,
            ui_wake,
        )?;

        Ok(Keyboard { domain, session })
    }

    // --- Session delegations ---

    pub fn poll(&self) {
        self.session.poll();
    }

    pub fn is_alive(&self) -> bool {
        self.session.is_alive()
    }

    pub fn write_support(&self) -> WriteSupport {
        self.session.write_support()
    }

    pub fn supports_live_layout_switching(&self) -> bool {
        self.session.supports_live_layout_switching()
    }

    pub fn supports_multiple_layouts(&self) -> bool {
        self.supports_live_layout_switching() && self.layout_names().len() > 1
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

    pub fn layer_names(&self) -> Vec<String> {
        self.domain.layer_names()
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

    pub fn layout_names(&self) -> Vec<String> {
        self.domain.layout_names()
    }

    pub fn active_layout_name(&self) -> String {
        self.domain.active_layout_name()
    }

    pub fn switch_layout(&self, name: &str) -> Result<(), String> {
        if !self.supports_live_layout_switching() {
            return Err("Device does not support live layout switching".to_string());
        }
        self.domain.switch_layout(name)
    }

    pub fn set_config(&self, config: OverlayConfig) {
        self.domain.set_config(config);
    }

    pub fn layout(&self) -> KeyboardLayout {
        self.domain.layout()
    }
}
