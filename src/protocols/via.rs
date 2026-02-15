use super::{qmk_json_parser, KeyboardDefinition, KeyboardProtocol};
use crate::keycode_labels::get_layout_key;
use crate::layout_key::LayoutKey;
use qmk_via_api::api::{KeyboardApi, MatrixInfo};
use std::error::Error;

pub struct ViaProtocol {
    api: KeyboardApi,
    definition: KeyboardDefinition,
}

impl ViaProtocol {
    pub fn connect(json_path: &str) -> Result<Self, Box<dyn Error>> {
        let definition = qmk_json_parser::parse_qmk_json(json_path)?;
        let api = Self::get_api(definition.vid, definition.pid)?;

        Ok(Self { api, definition })
    }

    fn get_api(vid: u16, pid: u16) -> Result<KeyboardApi, Box<dyn Error>> {
        let api = KeyboardApi::new(vid, pid, 0xff60)
            .map_err(|e| format!("Failed to connect to device ({vid:04x}:{pid:04x}): {e}"))?;

        let protocol_version = api
            .get_protocol_version()
            .map_err(|e| format!("Failed to get protocol version: {e}"))?;

        if protocol_version < 12 {
            return Err(format!(
                "Unsupported protocol version: {}. Minimum required version is 12.",
                protocol_version
            )
            .into());
        }

        Ok(api)
    }
}

impl KeyboardProtocol for ViaProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn get_layer_count(&self) -> Result<usize, Box<dyn Error>> {
        let count = self
            .api
            .get_layer_count()
            .map_err(|e| format!("Failed to get layer count: {e}"))?;
        Ok(count as usize)
    }

    fn read_all_keys(
        &self,
        layers: usize,
        rows: usize,
        cols: usize,
    ) -> Vec<Vec<Vec<Option<LayoutKey>>>> {
        let mut keys = vec![vec![vec![None; cols]; rows]; layers];
        let matrix_info = MatrixInfo {
            rows: rows as u8,
            cols: cols as u8,
        };

        for (layer, layer_keys) in keys.iter_mut().enumerate().take(layers) {
            if let Ok(raw_matrix) = self.api.read_raw_matrix(matrix_info, layer as u8) {
                for (i, &keycode) in raw_matrix.iter().enumerate() {
                    let row = i / cols;
                    let col = i % cols;
                    layer_keys[row][col] = get_layout_key(keycode);
                }
            }
        }

        keys
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.api
            .hid_read()
            .map_err(|e| format!("HID read error: {e}").into())
    }
}
