use crate::key_action::{KeyAction, KeymapSnapshot, LayerInfo};
use crate::layout_key::LayoutKey;

pub struct BoundKey {
    pub action: KeyAction,
    /// `None` = transparent binding (renders as fall-through, still editable).
    pub label: Option<LayoutKey>,
}

pub struct KeyMatrix {
    pub keys: Vec<Vec<Vec<Option<BoundKey>>>>,
    pub layers: Vec<LayerInfo>,
    pub pressed: Vec<Vec<bool>>,
}

impl KeyMatrix {
    pub fn from_snapshot(snapshot: KeymapSnapshot, rows: usize, cols: usize) -> Self {
        // Unnamed layers stay empty strings so the label fallback inside
        // `behavior_to_layout_key` applies, exactly as the ZMK protocol passes
        // names today.
        let layer_names: Vec<String> = snapshot
            .layers
            .iter()
            .map(|l| l.name.clone().unwrap_or_default())
            .collect();

        let keys = snapshot
            .actions
            .into_iter()
            .map(|layer| {
                layer
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|cell| {
                                cell.map(|action| BoundKey {
                                    label: action.resolve_label(&layer_names),
                                    action,
                                })
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();

        KeyMatrix {
            keys,
            layers: snapshot.layers,
            pressed: vec![vec![false; cols]; rows],
        }
    }

    pub fn get_num_layers(&self) -> usize {
        self.keys.len()
    }

    pub fn layer_infos(&self) -> &[LayerInfo] {
        &self.layers
    }

    /// The rendered label. `None` covers both an absent binding slot and a
    /// transparent binding (fall-through to lower layers).
    pub fn get_key(&self, layer: usize, row: usize, col: usize) -> Option<&LayoutKey> {
        self.keys
            .get(layer)
            .and_then(|l| l.get(row))
            .and_then(|r| r.get(col))
            .and_then(|k| k.as_ref())
            .and_then(|b| b.label.as_ref())
    }

    pub fn get_action(&self, layer: usize, row: usize, col: usize) -> Option<&KeyAction> {
        self.keys
            .get(layer)
            .and_then(|l| l.get(row))
            .and_then(|r| r.get(col))
            .and_then(|k| k.as_ref())
            .map(|b| &b.action)
    }

    pub fn is_transparent(&self, layer: usize, row: usize, col: usize) -> bool {
        self.keys
            .get(layer)
            .and_then(|l| l.get(row))
            .and_then(|r| r.get(col))
            .map(|k| k.as_ref().map(|b| b.label.is_none()).unwrap_or(true))
            .unwrap_or(true)
    }

    pub fn is_pressed(&self, row: usize, col: usize) -> bool {
        self.pressed
            .get(row)
            .and_then(|r| r.get(col))
            .copied()
            .unwrap_or(false)
    }

    pub fn set_pressed(&mut self, row: usize, col: usize, value: bool) {
        if let Some(r) = self.pressed.get_mut(row) {
            if col < r.len() {
                r[col] = value;
            }
        }
    }
}
