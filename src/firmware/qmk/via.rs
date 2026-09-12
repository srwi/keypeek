use super::common::{QmkFeatures, QmkProtocol};
use super::json_parser;
use crate::protocols::DeviceError;
use qmk_via_api::api::KeyboardApi;

pub struct ViaProtocol;

impl ViaProtocol {
    pub fn connect(json_path: &str) -> Result<QmkProtocol, DeviceError> {
        let definition = json_parser::parse_qmk_json(json_path)
            .map_err(|e| DeviceError::Protocol(e.to_string()))?;
        let api = Self::get_api(definition.vid, definition.pid)?;
        let features = QmkFeatures::probe(&api);

        Ok(QmkProtocol::new(api, definition, features))
    }

    fn get_api(vid: u16, pid: u16) -> Result<KeyboardApi, DeviceError> {
        // A read timeout keeps command/response round trips bounded so the HID
        // reader loop stays responsive between commands.
        let api = KeyboardApi::new(vid, pid, 0xff60, Some(250)).map_err(|e| {
            DeviceError::Transport(format!(
                "Failed to connect to device ({vid:04x}:{pid:04x}): {e}"
            ))
        })?;

        let protocol_version = api
            .get_protocol_version()
            .map_err(|e| DeviceError::Protocol(format!("Failed to get protocol version: {e}")))?;

        if protocol_version < 12 {
            return Err(DeviceError::Unsupported(format!(
                "Unsupported protocol version: {protocol_version}. Minimum required version is 12."
            )));
        }

        Ok(api)
    }
}
