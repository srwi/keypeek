use super::zmk_parser::parse_zmk_config_dir;
use super::{KeyboardDefinition, KeyboardProtocol};
use crate::layout_key::LayoutKey;
use qmk_via_api::api::KeyboardApi;
use std::error::Error;

pub struct ZmkProtocol {
    api: KeyboardApi,
    definition: KeyboardDefinition,
    layout_keys: Vec<Vec<Vec<Option<LayoutKey>>>>,
    layer_count: usize,
}

impl ZmkProtocol {
    pub fn connect(vid: u16, pid: u16, config_dir: &str) -> Result<Self, Box<dyn Error>> {
        // Parse physical layout and keymap from the ZMK config directory
        let (physical_layout, keymap) = parse_zmk_config_dir(config_dir)?;

        // Build keyboard definition from the physical layout
        let definition = physical_layout.to_keyboard_definition(vid, pid);

        // Build the key matrix placed at proper (row, col) positions
        let layout_keys = keymap.to_matrix(
            &physical_layout.keys,
            physical_layout.rows,
            physical_layout.cols,
        );

        let layer_count = keymap.layer_order.len();

        // Connect to the HID device
        let api = KeyboardApi::new(vid, pid, 0xff60)
            .map_err(|e| format!("Failed to connect to ZMK device ({vid:04x}:{pid:04x}): {e}"))?;

        Ok(Self {
            api,
            definition,
            layout_keys,
            layer_count,
        })
    }
}

impl KeyboardProtocol for ZmkProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn get_layer_count(&self) -> Result<usize, Box<dyn Error>> {
        // TODO: Get layer count from firmware via ZMK Studio protocol
        Ok(self.layer_count)
    }

    fn read_all_keycodes(&self, _layers: usize, _rows: usize, _cols: usize) -> Vec<Vec<Vec<u16>>> {
        // TODO: Implement via ZMK Studio protocol for live keymap sync
        Vec::new()
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.api
            .hid_read()
            .map_err(|e| format!("HID read error: {e}").into())
    }

    fn get_layout_keys(&self) -> Option<Vec<Vec<Vec<Option<LayoutKey>>>>> {
        Some(self.layout_keys.clone())
    }
}
