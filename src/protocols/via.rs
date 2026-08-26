use super::{
    qmk_json_parser, KeyboardDefinition, KeyboardProtocol, RawHidSubscription, SubscriptionSender,
};
use crate::key_action::{KeyAction, KeymapSnapshot};
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

    fn layer_count(&self) -> Result<usize, Box<dyn Error>> {
        let count = self
            .api
            .get_layer_count()
            .map_err(|e| format!("Failed to get layer count: {e}"))?;
        Ok(count as usize)
    }

    fn read_snapshot(&self) -> Result<KeymapSnapshot, Box<dyn Error>> {
        let layers = self.layer_count()?;
        let (rows, cols) = (self.definition.rows, self.definition.cols);
        let matrix_info = MatrixInfo {
            rows: rows as u8,
            cols: cols as u8,
        };

        let mut actions = vec![vec![vec![None; cols]; rows]; layers];
        for (layer, layer_actions) in actions.iter_mut().enumerate() {
            if let Ok(raw_matrix) = self.api.read_raw_matrix(matrix_info, layer as u8) {
                for (i, &keycode) in raw_matrix.iter().enumerate() {
                    let row = i / cols;
                    let col = i % cols;
                    layer_actions[row][col] = Some(KeyAction::Qmk(keycode));
                }
            }
        }

        Ok(KeymapSnapshot {
            layers: crate::key_action::LayerInfo::indexed(layers),
            actions,
        })
    }

    fn get_api(vid: u16, pid: u16) -> Result<KeyboardApi, Box<dyn Error>> {
        let api = KeyboardApi::new(vid, pid, 0xff60, None)
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

    fn read_keymap(&self) -> Result<KeymapSnapshot, Box<dyn Error>> {
        self.read_snapshot()
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.api
            .hid_read()
            .map_err(|e| format!("HID read error: {e}").into())
    }

    fn subscription_sender(&self) -> Result<Option<Box<dyn SubscriptionSender>>, Box<dyn Error>> {
        RawHidSubscription::open(self.definition.vid, self.definition.pid)
    }
}
