use crate::key_presenter::KeyPresenter;
use crate::key_spec::{KeySpec, KeymapSnapshot, LayerInfo};
use crate::layout_key::LayoutKey;

pub struct BoundKey {
    pub action: KeySpec,
    /// `None` = transparent binding (renders as fall-through, still editable).
    pub label: Option<LayoutKey>,
}

pub struct KeyMatrix {
    pub keys: Vec<Vec<Vec<Option<BoundKey>>>>,
    pub layers: Vec<LayerInfo>,
    pub pressed: Vec<Vec<bool>>,
}

impl KeyMatrix {
    pub fn from_snapshot(
        snapshot: KeymapSnapshot,
        rows: usize,
        cols: usize,
        presenter: &dyn KeyPresenter,
    ) -> Self {
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
                                    label: presenter.present_key(&action, &layer_names),
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

    pub fn get_action(&self, layer: usize, row: usize, col: usize) -> Option<&KeySpec> {
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

    /// Determines the effective layer for the key at `(row, col)` given current
    /// momentary and default layer bitmasks, taking layer transparency into account.
    pub fn effective_layer(
        &self,
        layer_state: u32,
        default_layer_state: u32,
        row: usize,
        col: usize,
    ) -> (u8, bool) {
        let num_layers = self.get_num_layers().min(32);
        let mut active_layer_above = false;

        for i in (1..num_layers).rev() {
            let layer_mask = 1u32 << (i as u32);
            let is_active_default_layer = (default_layer_state & layer_mask) != 0;
            let is_active_momentary_layer = (layer_state & layer_mask) != 0;
            if (is_active_momentary_layer || is_active_default_layer)
                && !self.is_transparent(i, row, col)
            {
                return (i as u8, is_active_default_layer && active_layer_above);
            }
            active_layer_above |= is_active_momentary_layer;
        }

        (0, active_layer_above)
    }

    /// `HELD_MOD_SHIFT`/`HELD_MOD_RALT` bits OR'd over every pressed key's
    /// `mod_mask`.
    pub fn held_mod_mask(
        &self,
        layout_keys: &[crate::domain::layout::Key],
        layer_state: u32,
        default_layer_state: u32,
    ) -> u16 {
        layout_keys.iter().fold(0u16, |acc, key| {
            if !self.is_pressed(key.row, key.col) {
                return acc;
            }
            let (effective_layer, _) =
                self.effective_layer(layer_state, default_layer_state, key.row, key.col);
            let mask = self
                .get_key(effective_layer as usize, key.row, key.col)
                .and_then(|k| k.mod_mask)
                .unwrap_or(0);
            acc | mask
        })
    }

    /// Updates a key binding and its rendered label in the matrix.
    pub fn update_binding(
        &mut self,
        layer: usize,
        row: usize,
        col: usize,
        action: KeySpec,
        label: Option<LayoutKey>,
    ) {
        if let Some(cell) = self
            .keys
            .get_mut(layer)
            .and_then(|l| l.get_mut(row))
            .and_then(|r| r.get_mut(col))
        {
            *cell = Some(BoundKey { label, action });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::key_spec::Modifiers;
    use crate::domain::layout::Key;
    use crate::key_spec::HidKey;
    use crate::layout_key::{Label, LayoutKey, HELD_MOD_SHIFT};

    #[test]
    fn effective_layer_falls_through_transparent_keys() {
        let mut matrix = KeyMatrix {
            keys: vec![
                // Layer 0: non-transparent key at (0, 0)
                vec![vec![Some(BoundKey {
                    action: KeySpec::KeyPress {
                        key: HidKey::keyboard(0x04),
                        modifiers: Modifiers::from_hid_mask(0),
                    },
                    label: Some(LayoutKey {
                        tap: Label::new("A"),
                        ..Default::default()
                    }),
                })]],
                // Layer 1: transparent key at (0, 0)
                vec![vec![Some(BoundKey {
                    action: KeySpec::Transparent,
                    label: None,
                })]],
            ],
            layers: vec![
                LayerInfo {
                    id: 0,
                    name: Some("Base".into()),
                },
                LayerInfo {
                    id: 1,
                    name: Some("Fn".into()),
                },
            ],
            pressed: vec![vec![false]],
        };

        // Layer 0 active only -> resolves to layer 0
        let (layer, shadow) = matrix.effective_layer(0, 1, 0, 0);
        assert_eq!(layer, 0);
        assert!(!shadow);

        // Layer 1 active, but transparent -> falls through to layer 0
        let (layer, _) = matrix.effective_layer(1 << 1, 1, 0, 0);
        assert_eq!(layer, 0);

        // Update layer 1 to be opaque
        matrix.update_binding(
            1,
            0,
            0,
            KeySpec::KeyPress {
                key: HidKey::keyboard(0x05),
                modifiers: Modifiers::from_hid_mask(0),
            },
            Some(LayoutKey {
                tap: Label::new("B"),
                ..Default::default()
            }),
        );

        // Now layer 1 resolves directly
        let (layer, _) = matrix.effective_layer(1 << 1, 1, 0, 0);
        assert_eq!(layer, 1);
    }

    #[test]
    fn held_mod_mask_aggregates_pressed_key_modifiers() {
        let matrix = KeyMatrix {
            keys: vec![vec![vec![
                Some(BoundKey {
                    action: KeySpec::KeyPress {
                        key: HidKey::keyboard(0xE1),
                        modifiers: Modifiers::from_hid_mask(0),
                    },
                    label: Some(LayoutKey {
                        tap: Label::new("Shift"),
                        mod_mask: Some(HELD_MOD_SHIFT),
                        ..Default::default()
                    }),
                }),
                Some(BoundKey {
                    action: KeySpec::KeyPress {
                        key: HidKey::keyboard(0x04),
                        modifiers: Modifiers::from_hid_mask(0),
                    },
                    label: Some(LayoutKey {
                        tap: Label::new("A"),
                        ..Default::default()
                    }),
                }),
            ]]],
            layers: vec![LayerInfo {
                id: 0,
                name: Some("Base".into()),
            }],
            pressed: vec![vec![false, false]],
        };

        let layout_keys = vec![
            Key {
                row: 0,
                col: 0,
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
                r: 0.0,
            },
            Key {
                row: 0,
                col: 1,
                x: 1.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
                r: 0.0,
            },
        ];

        // Nothing pressed -> mask is 0
        assert_eq!(matrix.held_mod_mask(&layout_keys, 0, 1), 0);

        let mut matrix = matrix;
        // Press Shift
        matrix.set_pressed(0, 0, true);
        assert_eq!(matrix.held_mod_mask(&layout_keys, 0, 1), HELD_MOD_SHIFT);

        // Also press 'A' -> mask still has Shift
        matrix.set_pressed(0, 1, true);
        assert_eq!(matrix.held_mod_mask(&layout_keys, 0, 1), HELD_MOD_SHIFT);

        // Release Shift -> mask becomes 0
        matrix.set_pressed(0, 0, false);
        assert_eq!(matrix.held_mod_mask(&layout_keys, 0, 1), 0);
    }
}
