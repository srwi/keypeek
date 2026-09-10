use super::qmk_common::{QmkFeatures, QmkProtocol};
use super::qmk_json_parser;
use qmk_via_api::api::KeyboardApi;
use std::error::Error;

pub struct ViaProtocol;

impl ViaProtocol {
    pub fn connect(json_path: &str) -> Result<QmkProtocol, Box<dyn Error>> {
        let definition = qmk_json_parser::parse_qmk_json(json_path)?;
        let api = Self::get_api(definition.vid, definition.pid)?;
        let features = QmkFeatures::probe(&api);

        Ok(QmkProtocol::new(api, definition, features))
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
