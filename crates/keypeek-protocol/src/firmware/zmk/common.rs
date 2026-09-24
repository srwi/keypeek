use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Mutex};

use zmk_studio_api::proto::zmk::behaviors::GetBehaviorDetailsResponse;
use zmk_studio_api::proto::zmk::keymap::{self, BehaviorBinding};
use zmk_studio_api::{
    decode_pointing_coords, typed_params, BacklightCommand, Behavior, BehaviorBindingParametersSet,
    BehaviorRole, BluetoothCommand, ExternalPowerCommand, HidUsage, MouseButton, OutputSelection,
    ResolvedLayer, UnderglowCommand,
};

use crate::protocols::ActionFilter;
use keypeek_core::geometry::flattened_top_left_after_center_rotation;
use keypeek_core::{Key, KeySpec, KeyboardDefinition, KeyboardLayout, KeymapSnapshot, LayerInfo};

use super::codec as zmk_codec;

pub struct ZmkData {
    pub physical_layouts: keymap::PhysicalLayouts,
    pub resolved_layers: Vec<ResolvedLayer>,
    pub supported_behaviors: HashSet<BehaviorRole>,
    pub behavior_metadata: HashMap<BehaviorRole, Vec<BehaviorBindingParametersSet>>,
}

pub struct ZmkLayout {
    pub definition: KeyboardDefinition,
    pub snapshot: Mutex<KeymapSnapshot>,
    pub supported_behaviors: HashSet<BehaviorRole>,
    pub behavior_metadata: HashMap<BehaviorRole, Vec<BehaviorBindingParametersSet>>,
}

pub fn zmk_action_filter(
    supported: HashSet<BehaviorRole>,
    metadata: HashMap<BehaviorRole, Vec<BehaviorBindingParametersSet>>,
) -> Option<ActionFilter> {
    Some(Arc::new(move |spec| {
        match zmk_codec::keyspec_to_zmk(spec) {
            Ok(behavior) => match behavior.role() {
                Some(role) => {
                    if !supported.is_empty() && !supported.contains(&role) {
                        return false;
                    }
                    metadata
                        .get(&role)
                        .is_none_or(|sets| behavior.matches_metadata(sets))
                }
                None => false,
            },
            Err(_) => false,
        }
    }))
}

/// Resolves a single protobuf `BehaviorBinding` into a high-level `Behavior` enum
/// using the known behavior role or custom behavior parameter metadata.
pub fn resolve_binding(
    b: &BehaviorBinding,
    role: Option<BehaviorRole>,
    custom_details: Option<&GetBehaviorDetailsResponse>,
) -> Behavior {
    let b_id = b.behavior_id as u32;
    match role {
        Some(BehaviorRole::KeyPress) => Behavior::KeyPress(HidUsage::from_encoded(b.param1)),
        Some(BehaviorRole::KeyToggle) => Behavior::KeyToggle(HidUsage::from_encoded(b.param1)),
        Some(BehaviorRole::LayerTap) => Behavior::LayerTap {
            layer_id: b.param1,
            tap: HidUsage::from_encoded(b.param2),
        },
        Some(BehaviorRole::ModTap) => Behavior::ModTap {
            hold: HidUsage::from_encoded(b.param1),
            tap: HidUsage::from_encoded(b.param2),
        },
        Some(BehaviorRole::StickyKey) => Behavior::StickyKey(HidUsage::from_encoded(b.param1)),
        Some(BehaviorRole::StickyLayer) => Behavior::StickyLayer { layer_id: b.param1 },
        Some(BehaviorRole::MomentaryLayer) => Behavior::MomentaryLayer { layer_id: b.param1 },
        Some(BehaviorRole::ToggleLayer) => Behavior::ToggleLayer { layer_id: b.param1 },
        Some(BehaviorRole::ToLayer) => Behavior::ToLayer { layer_id: b.param1 },
        Some(BehaviorRole::Bluetooth) => {
            Behavior::Bluetooth(BluetoothCommand::from_raw(b.param1, b.param2))
        }
        Some(BehaviorRole::ExternalPower) => {
            Behavior::ExternalPower(ExternalPowerCommand::from_raw(b.param1))
        }
        Some(BehaviorRole::OutputSelection) => {
            Behavior::OutputSelection(OutputSelection::from_raw(b.param1))
        }
        Some(BehaviorRole::Backlight) => {
            Behavior::Backlight(BacklightCommand::from_raw(b.param1, b.param2))
        }
        Some(BehaviorRole::Underglow) => {
            Behavior::Underglow(UnderglowCommand::from_raw(b.param1, b.param2))
        }
        Some(BehaviorRole::MouseKeyPress) => {
            Behavior::MouseKeyPress(MouseButton::from_raw(b.param1))
        }
        Some(BehaviorRole::MouseMove) => {
            let (x, y) = decode_pointing_coords(b.param1);
            Behavior::MouseMove { x, y }
        }
        Some(BehaviorRole::MouseScroll) => {
            let (x, y) = decode_pointing_coords(b.param1);
            Behavior::MouseScroll { x, y }
        }
        Some(BehaviorRole::CapsWord) => Behavior::CapsWord,
        Some(BehaviorRole::KeyRepeat) => Behavior::KeyRepeat,
        Some(BehaviorRole::Reset) => Behavior::Reset,
        Some(BehaviorRole::Bootloader) => Behavior::Bootloader,
        Some(BehaviorRole::SoftOff) => Behavior::SoftOff,
        Some(BehaviorRole::StudioUnlock) => Behavior::StudioUnlock,
        Some(BehaviorRole::GraveEscape) => Behavior::GraveEscape,
        Some(BehaviorRole::Transparent) => Behavior::Transparent,
        Some(BehaviorRole::None) => Behavior::None,
        None => match custom_details {
            Some(details) => {
                let (param1, param2) = typed_params(&details.metadata, b.param1, b.param2);
                Behavior::Custom {
                    behavior_id: b_id,
                    display_name: details.display_name.clone(),
                    param1,
                    param2,
                }
            }
            None => Behavior::Unknown {
                behavior_id: b.behavior_id,
                param1: b.param1,
                param2: b.param2,
            },
        },
    }
}

pub fn build_from_zmk_data(vid: u16, pid: u16, data: ZmkData) -> Result<ZmkLayout, Box<dyn Error>> {
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
        .map(|(i, k)| {
            let w = k.width as f32 / 100.0;
            let h = k.height as f32 / 100.0;

            let x = k.x as f32 / 100.0;
            let y = k.y as f32 / 100.0;

            // Position is where the key's center lands after rotating around the pivot;
            // the rotation itself is applied at render time via `r`.
            let angle_deg = k.r as f32 / 100.0;
            let pivot_x = if k.rx == 0 { k.x } else { k.rx } as f32 / 100.0;
            let pivot_y = if k.ry == 0 { k.y } else { k.ry } as f32 / 100.0;
            let (x, y) =
                flattened_top_left_after_center_rotation(x, y, w, h, angle_deg, pivot_x, pivot_y);

            Key {
                row: 0,
                col: i,
                x,
                y,
                w,
                h,
                r: angle_deg,
            }
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

    let snapshot = snapshot_from_resolved(&data.resolved_layers, num_keys);

    Ok(ZmkLayout {
        definition,
        snapshot: Mutex::new(snapshot),
        supported_behaviors: data.supported_behaviors,
        behavior_metadata: data.behavior_metadata,
    })
}

/// Builds a keymap snapshot from the device's resolved layers. Bindings beyond
/// `num_keys` are dropped and short layers are padded with `None`, matching the
/// matrix dimensions.
pub fn snapshot_from_resolved(resolved: &[ResolvedLayer], num_keys: usize) -> KeymapSnapshot {
    let layers = resolved
        .iter()
        .map(|layer| LayerInfo {
            id: layer.id,
            name: (!layer.name.is_empty()).then(|| layer.name.clone()),
        })
        .collect();

    let actions = resolved
        .iter()
        .map(|layer| {
            let mut row: Vec<Option<KeySpec>> = layer
                .bindings
                .iter()
                .map(|b| Some(zmk_codec::zmk_to_keyspec(b)))
                .collect();
            row.resize(num_keys, None);
            vec![row]
        })
        .collect();

    KeymapSnapshot { layers, actions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zmk_studio_api::proto::zmk::keymap::{KeyPhysicalAttrs, PhysicalLayout};
    use zmk_studio_api::{Behavior, HidUsage, Keycode};

    #[test]
    fn test_build_from_zmk_data_constructs_layout_and_snapshot() {
        let physical_layouts = keymap::PhysicalLayouts {
            active_layout_index: 0,
            layouts: vec![PhysicalLayout {
                name: "Standard 2-Key".to_string(),
                keys: vec![
                    KeyPhysicalAttrs {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                        r: 0,
                        rx: 0,
                        ry: 0,
                    },
                    KeyPhysicalAttrs {
                        x: 100,
                        y: 0,
                        width: 100,
                        height: 100,
                        r: 0,
                        rx: 0,
                        ry: 0,
                    },
                ],
            }],
        };

        let resolved_layers = vec![
            ResolvedLayer {
                id: 0,
                name: "BASE".to_string(),
                bindings: vec![
                    Behavior::KeyPress(HidUsage::from(Keycode::A)),
                    Behavior::KeyPress(HidUsage::from(Keycode::B)),
                ],
            },
            ResolvedLayer {
                id: 1,
                name: "NAV".to_string(),
                bindings: vec![Behavior::Transparent, Behavior::None],
            },
        ];

        let mut supported_behaviors = HashSet::new();
        supported_behaviors.insert(BehaviorRole::KeyPress);

        let data = ZmkData {
            physical_layouts,
            resolved_layers,
            supported_behaviors,
            behavior_metadata: HashMap::new(),
        };

        let zmk_layout = build_from_zmk_data(0x1D50, 0x615E, data).expect("should build layout");
        assert_eq!(zmk_layout.definition.vid, 0x1D50);
        assert_eq!(zmk_layout.definition.pid, 0x615E);
        assert_eq!(zmk_layout.definition.cols, 2);

        let snapshot = zmk_layout.snapshot.lock().unwrap();
        assert_eq!(snapshot.layers.len(), 2);
        assert_eq!(snapshot.layers[0].name.as_deref(), Some("BASE"));
        assert_eq!(snapshot.layers[1].name.as_deref(), Some("NAV"));

        assert_eq!(snapshot.actions.len(), 2);
        assert_eq!(snapshot.actions[0][0].len(), 2);
        assert!(snapshot.actions[0][0][0].is_some());
    }
}
