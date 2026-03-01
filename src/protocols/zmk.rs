use super::zmk_rpc::ZmkData;
use super::{Key, KeyboardDefinition, KeyboardLayout, KeyboardProtocol};
use crate::layout_key::LayoutKey;
use qmk_via_api::api::KeyboardApi;
use std::error::Error;

type LayerKeys3d = Vec<Vec<Vec<Option<LayoutKey>>>>;

pub struct ZmkProtocol {
    api: KeyboardApi,
    definition: KeyboardDefinition,
    layout_keys: LayerKeys3d,
    layer_count: usize,
}

impl ZmkProtocol {
    pub fn connect_live(vid: u16, pid: u16, zmk_data: &ZmkData) -> Result<Self, Box<dyn Error>> {
        let (definition, layout_keys, layer_count) = build_from_zmk_data(vid, pid, zmk_data)?;
        let api = KeyboardApi::new(vid, pid, 0xff60).map_err(|e| {
            std::io::Error::other(format!(
                "Failed to connect HID ({vid:04x}:{pid:04x}): {e}"
            ))
        })?;

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
        Ok(self.layer_count)
    }

    fn read_all_keys(&self, _layers: usize, _rows: usize, _cols: usize) -> LayerKeys3d {
        self.layout_keys.clone()
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.api
            .hid_read()
            .map_err(|e| format!("HID read error: {e}").into())
    }
}

fn build_from_zmk_data(
    vid: u16,
    pid: u16,
    data: &ZmkData,
) -> Result<(KeyboardDefinition, LayerKeys3d, usize), Box<dyn Error>> {
    const ACTIVE_LAYOUT_NAME: &str = "active physical layout";

    let active_idx = data.physical_layouts.active_layout_index as usize;
    let proto_layouts = &data.physical_layouts.layouts;

    if proto_layouts.is_empty() {
        return Err("Device has no physical layouts".into());
    }

    let active_layout = proto_layouts
        .get(active_idx)
        .ok_or_else(|| format!("Invalid active layout index: {active_idx}"))?;
    let active_keys: Vec<Key> = active_layout
        .keys
        .iter()
        .enumerate()
        .map(|(i, k)| Key {
            row: 0,
            col: i,
            x: k.x as f32 / 100.0,
            y: k.y as f32 / 100.0,
            w: k.width as f32 / 100.0,
            h: k.height as f32 / 100.0,
        })
        .collect();
    let num_keys = active_keys.len();

    let definition = KeyboardDefinition {
        vid,
        pid,
        rows: 1,
        cols: num_keys,
        layouts: vec![KeyboardLayout {
            name: ACTIVE_LAYOUT_NAME.to_string(),
            keys: active_keys,
        }],
    };

    let layer_count = data.layer_count;
    let active_key_count = num_keys;
    let mut layout_keys_3d = Vec::with_capacity(layer_count);

    for layer_keys in &data.layout_keys {
        let mut row = vec![None; num_keys];

        for (pos, key) in layer_keys.iter().enumerate() {
            if pos >= active_key_count {
                break;
            }
            if pos < num_keys {
                row[pos] = key.clone();
            }
        }

        layout_keys_3d.push(vec![row]);
    }

    Ok((definition, layout_keys_3d, layer_count))
}
