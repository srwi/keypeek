//! Pure in-memory keyboard domain aggregate and layout switching state machine.

use std::sync::Mutex;
use std::time::Duration;
use web_time::Instant;

use crate::domain::key_matrix::KeyMatrix;
use crate::domain::key_spec::{KeySpec, LayerInfo};
use crate::domain::layout::{KeyboardDefinition, KeyboardLayout};
use crate::domain::visibility::{OverlayConfig, VisibilityStateMachine};
use crate::layout_key::LayoutKey;

/// Pure domain aggregate holding keyboard layout, matrix bindings, layer states,
/// and overlay visibility. Has no background threads or communication channels.
pub struct KeyboardDomain {
    definition: KeyboardDefinition,
    active_layout_name: Mutex<String>,
    layout: Mutex<KeyboardLayout>,
    matrix: Mutex<KeyMatrix>,
    layer_state: Mutex<u32>,
    default_layer_state: Mutex<u32>,
    visibility: Mutex<VisibilityStateMachine>,
}

impl KeyboardDomain {
    pub fn new(
        definition: KeyboardDefinition,
        active_layout_name: String,
        layout: KeyboardLayout,
        matrix: KeyMatrix,
        config: OverlayConfig,
    ) -> Self {
        Self {
            definition,
            active_layout_name: Mutex::new(active_layout_name),
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
        self.matrix
            .lock()
            .unwrap()
            .layer_infos()
            .get(index)
            .cloned()
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

    pub fn layer_names(&self) -> Vec<String> {
        self.layer_infos()
            .iter()
            .map(|l| l.name.clone().unwrap_or_default())
            .collect()
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

    pub fn layout_names(&self) -> Vec<String> {
        self.definition.get_layout_names()
    }

    pub fn active_layout_name(&self) -> String {
        self.active_layout_name.lock().unwrap().clone()
    }

    pub fn switch_layout(&self, name: &str) -> Result<(), String> {
        let next_layout = self.definition.get_layout(name)?;
        *self.layout.lock().unwrap() = next_layout;
        *self.active_layout_name.lock().unwrap() = name.to_string();
        Ok(())
    }

    pub fn layout(&self) -> KeyboardLayout {
        self.layout.lock().unwrap().clone()
    }

    pub fn layer_state(&self) -> u32 {
        *self.layer_state.lock().unwrap()
    }

    pub fn default_layer_state(&self) -> u32 {
        *self.default_layer_state.lock().unwrap()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn create_test_domain() -> KeyboardDomain {
        let definition = KeyboardDefinition {
            vid: 0x1234,
            pid: 0x5678,
            rows: 1,
            cols: 2,
            layouts: vec![
                KeyboardLayout {
                    name: "Default".to_string(),
                    keys: vec![],
                },
                KeyboardLayout {
                    name: "Alternative".to_string(),
                    keys: vec![],
                },
            ],
        };
        let layout = definition.get_layout("Default").unwrap();
        let matrix = KeyMatrix::from_snapshot(
            crate::key_spec::KeymapSnapshot {
                layers: vec![],
                actions: vec![],
            },
            1,
            2,
            &crate::key_presenter::StandardKeyPresenter,
        );
        let config = OverlayConfig {
            timeout_ms: 2000,
            activation_delay_ms: 300,
            visible_layers: u32::MAX,
        };
        KeyboardDomain::new(definition, "Default".to_string(), layout, matrix, config)
    }

    #[test]
    fn test_layout_names_and_active_layout() {
        let domain = create_test_domain();
        assert_eq!(domain.layout_names(), vec!["Default", "Alternative"]);
        assert_eq!(domain.active_layout_name(), "Default");
    }

    #[test]
    fn test_switch_layout_success_and_failure() {
        let domain = create_test_domain();
        assert!(domain.switch_layout("Alternative").is_ok());
        assert_eq!(domain.active_layout_name(), "Alternative");
        assert_eq!(domain.layout().name, "Alternative");

        let err = domain.switch_layout("NonExistent");
        assert!(err.is_err());
        assert_eq!(domain.active_layout_name(), "Alternative");
    }
}
