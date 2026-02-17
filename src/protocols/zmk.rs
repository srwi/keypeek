use super::zmk_studio::StudioData;
use super::{Key, KeyboardDefinition, KeyboardLayout, KeyboardProtocol};
use crate::layout_key::LayoutKey;
use qmk_via_api::api::KeyboardApi;
use std::error::Error;
use std::path::PathBuf;

type LayerKeys3d = Vec<Vec<Vec<Option<LayoutKey>>>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ZmkCache {
    pub definition: KeyboardDefinition,
    pub layout_keys: LayerKeys3d,
    pub layer_count: usize,
}

impl ZmkCache {
    pub fn cache_path(vid: u16, pid: u16) -> PathBuf {
        PathBuf::from(format!("zmk_cache_{:04x}_{:04x}.json", vid, pid))
    }

    pub fn load(vid: u16, pid: u16) -> Option<Self> {
        let path = Self::cache_path(vid, pid);
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save(&self, vid: u16, pid: u16) -> Result<(), Box<dyn Error>> {
        let path = Self::cache_path(vid, pid);
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }
}

pub fn save_and_get_layout_names(
    vid: u16,
    pid: u16,
    studio_data: &StudioData,
) -> Result<Vec<String>, Box<dyn Error>> {
    let (definition, layout_keys, layer_count) = build_from_studio_data(vid, pid, studio_data)?;

    let cache = ZmkCache {
        definition: definition.clone(),
        layout_keys,
        layer_count,
    };
    cache.save(vid, pid)?;

    Ok(definition.get_layout_names())
}

pub struct ZmkProtocol {
    api: KeyboardApi,
    definition: KeyboardDefinition,
    layout_keys: LayerKeys3d,
    layer_count: usize,
}

impl ZmkProtocol {
    pub fn connect_cached(vid: u16, pid: u16) -> Result<Self, Box<dyn Error>> {
        let cache = ZmkCache::load(vid, pid)
            .ok_or_else(|| format!("No cached data for {:04x}:{:04x}", vid, pid))?;

        let mut last_err = String::new();
        for attempt in 0..5 {
            if attempt > 0 {
                let delay = std::time::Duration::from_millis(300 * attempt as u64);
                eprintln!(
                    "HID connect attempt {} failed, retrying in {:?}...",
                    attempt, delay
                );
                std::thread::sleep(delay);
            }
            match KeyboardApi::new(vid, pid, 0xff60) {
                Ok(api) => {
                    return Ok(Self {
                        api,
                        definition: cache.definition,
                        layout_keys: cache.layout_keys,
                        layer_count: cache.layer_count,
                    });
                }
                Err(e) => {
                    last_err = format!("{e}");
                }
            }
        }

        Err(
            format!("Failed to connect HID ({vid:04x}:{pid:04x}) after 5 attempts: {last_err}")
                .into(),
        )
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

fn build_from_studio_data(
    vid: u16,
    pid: u16,
    data: &StudioData,
) -> Result<(KeyboardDefinition, LayerKeys3d, usize), Box<dyn Error>> {
    const ACTIVE_LAYOUT_NAME: &str = "active physical layout";

    let active_idx = data.physical_layouts.active_layout_index as usize;
    let proto_layouts = &data.physical_layouts.layouts;

    if proto_layouts.is_empty() {
        return Err("Device has no physical layouts".into());
    }

    // Build KeyboardDefinition from currently active physical layout only.
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
