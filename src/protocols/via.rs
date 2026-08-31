use super::qmk_common::{qmk_action_filter, qmk_read_snapshot, qmk_set_key, QmkFeatures};
use super::{
    qmk_json_parser, KeyboardDefinition, KeyboardProtocol, RawHidSubscription, SubscriptionSender,
    WriteSupport,
};
use crate::key_action::{KeyAction, KeymapSnapshot};
use qmk_via_api::api::KeyboardApi;
use std::error::Error;

pub struct ViaProtocol {
    api: KeyboardApi,
    definition: KeyboardDefinition,
    features: QmkFeatures,
}

impl ViaProtocol {
    pub fn connect(json_path: &str) -> Result<Self, Box<dyn Error>> {
        let definition = qmk_json_parser::parse_qmk_json(json_path)?;
        let api = Self::get_api(definition.vid, definition.pid)?;
        let features = QmkFeatures::probe(&api);

        Ok(Self {
            api,
            definition,
            features,
        })
    }

    fn get_api(vid: u16, pid: u16) -> Result<KeyboardApi, Box<dyn Error>> {
        // A read timeout keeps command/response round trips bounded so the HID
        // reader loop stays responsive between commands.
        let api = KeyboardApi::new(vid, pid, 0xff60, Some(250))
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
        qmk_read_snapshot(&self.api, &self.definition)
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.api
            .hid_read()
            .map_err(|e| format!("HID read error: {e}").into())
    }

    fn write_support(&self) -> WriteSupport {
        WriteSupport::Immediate
    }

    fn set_key(
        &mut self,
        _layer: &crate::key_action::LayerInfo,
        layer_index: usize,
        row: usize,
        col: usize,
        action: &KeyAction,
    ) -> Result<(), Box<dyn Error>> {
        qmk_set_key(&self.api, layer_index, row, col, action)
    }

    fn subscription_sender(&self) -> Result<Option<Box<dyn SubscriptionSender>>, Box<dyn Error>> {
        RawHidSubscription::open(self.definition.vid, self.definition.pid)
    }

    fn action_filter(&self) -> Option<super::ActionFilter> {
        qmk_action_filter(self.features)
    }
}
