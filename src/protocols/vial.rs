use super::{kle_parser, KeyboardDefinition, KeyboardProtocol};
use crate::keycode_labels::get_layout_key;
use crate::layout_key::LayoutKey;
use qmk_via_api::api::{KeyboardApi, MatrixInfo};
use std::error::Error;

const VIAL_PREFIX: u8 = 0xFE;

#[repr(u8)]
enum VialCommand {
    KeyboardId = 0x00,
    Size = 0x01,
    Def = 0x02,
}

pub struct VialProtocol {
    api: KeyboardApi,
    definition: KeyboardDefinition,
}

impl VialProtocol {
    pub fn connect(vid: u16, pid: u16) -> Result<Self, Box<dyn Error>> {
        let api = KeyboardApi::new(vid, pid, 0xff60)
            .map_err(|e| format!("Failed to connect to device ({vid:04x}:{pid:04x}): {e}"))?;

        Self::init_from_api(api, vid, pid)
    }

    fn init_from_api(api: KeyboardApi, vid: u16, pid: u16) -> Result<Self, Box<dyn Error>> {
        let (protocol_version, _keyboard_uid) = Self::get_keyboard_id(&api)?;

        if protocol_version == 0 {
            return Err("Device does not support VIAL protocol".into());
        }

        let definition = Self::fetch_definition(&api, vid, pid)?;

        Ok(Self { api, definition })
    }

    fn vial_command(
        api: &KeyboardApi,
        cmd: VialCommand,
        data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut msg = vec![0u8; 32];
        msg[0] = VIAL_PREFIX;
        msg[1] = cmd as u8;

        let copy_len = data.len().min(30);
        msg[2..2 + copy_len].copy_from_slice(&data[..copy_len]);

        api.hid_send(msg)
            .map_err(|e| format!("VIAL write error: {e}"))?;

        api.hid_read()
            .map_err(|e| format!("VIAL read error: {e}").into())
    }

    fn vial_get_def_block(api: &KeyboardApi, block: u32) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut msg = vec![0u8; 32];
        msg[0] = VIAL_PREFIX;
        msg[1] = VialCommand::Def as u8;
        msg[2..6].copy_from_slice(&block.to_le_bytes());

        api.hid_send(msg)
            .map_err(|e| format!("VIAL write error: {e}"))?;

        api.hid_read()
            .map_err(|e| format!("VIAL read error: {e}").into())
    }

    fn get_keyboard_id(api: &KeyboardApi) -> Result<(u32, [u8; 8]), Box<dyn Error>> {
        let response = Self::vial_command(api, VialCommand::KeyboardId, &[])?;

        let protocol_version =
            u32::from_le_bytes([response[0], response[1], response[2], response[3]]);

        let mut uid = [0u8; 8];
        uid.copy_from_slice(&response[4..12]);

        Ok((protocol_version, uid))
    }

    fn get_definition_size(api: &KeyboardApi) -> Result<u32, Box<dyn Error>> {
        let response = Self::vial_command(api, VialCommand::Size, &[])?;
        let size = u32::from_le_bytes([response[0], response[1], response[2], response[3]]);
        Ok(size)
    }

    fn fetch_definition(
        api: &KeyboardApi,
        vid: u16,
        pid: u16,
    ) -> Result<KeyboardDefinition, Box<dyn Error>> {
        let size = Self::get_definition_size(api)? as usize;

        if size == 0 {
            return Err("VIAL definition size is 0".into());
        }

        // Fetch compressed definition in chunks
        let mut compressed = Vec::with_capacity(size);
        let mut block: u32 = 0;

        while compressed.len() < size {
            let response = Self::vial_get_def_block(api, block)?;

            let remaining = size - compressed.len();
            let chunk_size = remaining.min(32);
            compressed.extend_from_slice(&response[..chunk_size]);

            block += 1;
        }

        let mut decompressed = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&compressed);
            lzma_rs::xz_decompress(&mut cursor, &mut decompressed)
                .map_err(|e| format!("Failed to decompress VIAL definition: {e}"))?;
        }

        let json_str = String::from_utf8(decompressed)
            .map_err(|e| format!("VIAL definition is not valid UTF-8: {e}"))?;

        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse VIAL definition JSON: {e}"))?;

        kle_parser::parse_vial_definition(&json, vid, pid)
    }
}

impl KeyboardProtocol for VialProtocol {
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
