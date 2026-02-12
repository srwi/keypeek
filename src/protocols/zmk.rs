//! ZMK keyboard protocol — connects via ZMK Studio (serial) for layout/keymap
//! data and via HID (usage page 0xFF60) for live keypress monitoring.
//!
//! Layout and keymap data is cached to a JSON file so that subsequent app starts
//! can skip the Studio protocol (and the physical unlock step) entirely.

use super::zmk_studio::StudioData;
use super::{Key, KeyboardDefinition, KeyboardLayout, KeyboardProtocol};
use crate::layout_key::LayoutKey;
use qmk_via_api::api::KeyboardApi;
use std::error::Error;
use std::path::PathBuf;

/// Cached ZMK data that can be serialized to/from JSON.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ZmkCache {
    pub definition: KeyboardDefinition,
    pub layout_keys: Vec<Vec<Vec<Option<LayoutKey>>>>,
    pub layer_count: usize,
}

impl ZmkCache {
    /// Path for the cache file for a given VID/PID.
    pub fn cache_path(vid: u16, pid: u16) -> PathBuf {
        PathBuf::from(format!("zmk_cache_{:04x}_{:04x}.json", vid, pid))
    }

    /// Try to load cached data from disk.
    pub fn load(vid: u16, pid: u16) -> Option<Self> {
        let path = Self::cache_path(vid, pid);
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Save cached data to disk.
    pub fn save(&self, vid: u16, pid: u16) -> Result<(), Box<dyn Error>> {
        let path = Self::cache_path(vid, pid);
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }
}

pub struct ZmkProtocol {
    api: KeyboardApi,
    definition: KeyboardDefinition,
    layout_keys: Vec<Vec<Vec<Option<LayoutKey>>>>,
    layer_count: usize,
}

impl ZmkProtocol {
    /// Connect using cached data (no Studio protocol needed, no unlock).
    /// Only opens HID for keypress monitoring.
    pub fn connect_cached(vid: u16, pid: u16) -> Result<Self, Box<dyn Error>> {
        let cache = ZmkCache::load(vid, pid)
            .ok_or_else(|| format!("No cached data for {:04x}:{:04x}", vid, pid))?;

        let api = KeyboardApi::new(vid, pid, 0xff60)
            .map_err(|e| format!("Failed to connect HID ({vid:04x}:{pid:04x}): {e}"))?;

        Ok(Self {
            api,
            definition: cache.definition,
            layout_keys: cache.layout_keys,
            layer_count: cache.layer_count,
        })
    }

    /// Connect via ZMK Studio protocol over serial to fetch layout/keymap,
    /// then cache and open HID for keypress monitoring.
    ///
    /// The device must already be unlocked. Use `zmk_studio::fetch_studio_data`
    /// which returns `DEVICE_LOCKED` if locked — the caller should handle the
    /// unlock UI flow before calling this.
    pub fn connect_studio(
        vid: u16,
        pid: u16,
        studio_data: StudioData,
    ) -> Result<Self, Box<dyn Error>> {
        let (definition, layout_keys, layer_count) =
            build_from_studio_data(vid, pid, &studio_data)?;

        // Cache the data
        let cache = ZmkCache {
            definition: definition.clone(),
            layout_keys: layout_keys.clone(),
            layer_count,
        };
        if let Err(e) = cache.save(vid, pid) {
            eprintln!("Warning: failed to save ZMK cache: {e}");
        }

        // Connect HID for keypress monitoring
        let api = KeyboardApi::new(vid, pid, 0xff60)
            .map_err(|e| format!("Failed to connect HID ({vid:04x}:{pid:04x}): {e}"))?;

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

    fn read_all_keys(
        &self,
        _layers: usize,
        _rows: usize,
        _cols: usize,
    ) -> Vec<Vec<Vec<Option<LayoutKey>>>> {
        self.layout_keys.clone()
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.api
            .hid_read()
            .map_err(|e| format!("HID read error: {e}").into())
    }
}

// ---------------------------------------------------------------------------
// Build KeyboardDefinition + LayoutKeys from Studio data
// ---------------------------------------------------------------------------

fn build_from_studio_data(
    vid: u16,
    pid: u16,
    data: &StudioData,
) -> Result<
    (
        KeyboardDefinition,
        Vec<Vec<Vec<Option<LayoutKey>>>>,
        usize,
    ),
    Box<dyn Error>,
> {
    let active_idx = data.physical_layouts.active_layout_index as usize;
    let proto_layouts = &data.physical_layouts.layouts;

    if proto_layouts.is_empty() {
        return Err("Device has no physical layouts".into());
    }

    // Build KeyboardDefinition from physical layouts
    // Use synthetic row/col: row=0, col=position_index
    let mut layouts = Vec::new();
    let mut num_keys = 0;

    for pl in proto_layouts {
        let keys: Vec<Key> = pl
            .keys
            .iter()
            .enumerate()
            .map(|(i, k)| {
                // Proto values are in centi-units (100 = 1u)
                Key {
                    row: 0,
                    col: i,
                    x: k.x as f32 / 100.0,
                    y: k.y as f32 / 100.0,
                    w: k.width as f32 / 100.0,
                    h: k.height as f32 / 100.0,
                }
            })
            .collect();

        num_keys = num_keys.max(keys.len());

        layouts.push(KeyboardLayout {
            name: if pl.name.is_empty() {
                "default".to_string()
            } else {
                pl.name.clone()
            },
            keys,
        });
    }

    let definition = KeyboardDefinition {
        vid,
        pid,
        rows: 1,
        cols: num_keys,
        layouts,
    };

    // Build layout_keys: layers × 1 row × num_keys cols
    let layer_count = data.keymap.layers.len();
    let mut layout_keys = Vec::with_capacity(layer_count);

    // Get the active layout's key count for binding alignment
    let active_key_count = if active_idx < proto_layouts.len() {
        proto_layouts[active_idx].keys.len()
    } else {
        num_keys
    };

    for layer in &data.keymap.layers {
        let mut row = vec![None; num_keys];

        for (pos, binding) in layer.bindings.iter().enumerate() {
            if pos >= active_key_count {
                break;
            }
            if pos < num_keys {
                row[pos] = data.behavior_map.binding_to_layout_key(binding);
            }
        }

        layout_keys.push(vec![row]);
    }

    Ok((definition, layout_keys, layer_count))
}
