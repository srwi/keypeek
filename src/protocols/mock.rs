//! A virtual keyboard backed by a checked-in fixture, for developing without hardware.
//!
//! It reports a fixed layout and keymap, and cycles its momentary layer state on a timer
//! so layer-change rendering can be exercised. The mock device is only registered
//! during discovery in debug builds (`cfg!(debug_assertions)` in `device_discovery`).

use super::{KeyboardDefinition, KeyboardProtocol, WriteSupport};
use crate::key_action::{KeyAction, KeymapSnapshot};
use qmk_via_api::keycodes::Keycode;
use qmk_via_api::QmkLayerOp;
use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

const FIXTURE: &str = include_str!("../../resources/mock_keyboard.json");

/// How long each layer in the cycle is held.
const TICK_INTERVAL: Duration = Duration::from_millis(1500);

/// Layer 0 is the base layer, matching a firmware default of `default_layer_state == 1`.
const DEFAULT_LAYER_STATE: u32 = 1;

/// `layer_state` is a bitmask, so only layers below 32 are representable.
const MAX_LAYERS: u32 = 32;

#[derive(serde::Deserialize)]
struct MockFixture {
    definition: KeyboardDefinition,
    /// One entry per layer, each holding `rows * cols` keycodes in row-major order.
    layers: Vec<Vec<String>>,
}

pub struct MockProtocol {
    definition: KeyboardDefinition,
    layers: Vec<Vec<u16>>,
    /// The `layer_state` masks emitted by successive `hid_read` calls, cycled in order.
    layer_states: Vec<u32>,
    tick: AtomicUsize,
    tick_interval: Duration,
}

impl MockProtocol {
    pub fn connect() -> Result<Self, Box<dyn Error>> {
        Self::with_tick_interval(TICK_INTERVAL)
    }

    /// The interval is a parameter so tests can cycle layers without waiting on the
    /// human-paced default.
    fn with_tick_interval(tick_interval: Duration) -> Result<Self, Box<dyn Error>> {
        let fixture: MockFixture = serde_json::from_str(FIXTURE)
            .map_err(|e| format!("Invalid mock keyboard fixture: {e}"))?;

        let (rows, cols) = (fixture.definition.rows, fixture.definition.cols);
        if rows == 0 || cols == 0 {
            return Err("Mock keyboard fixture has an empty matrix".into());
        }
        if fixture.layers.is_empty() {
            return Err("Mock keyboard fixture has no layers".into());
        }

        let mut layers = Vec::with_capacity(fixture.layers.len());
        for (index, layer) in fixture.layers.iter().enumerate() {
            if layer.len() != rows * cols {
                return Err(format!(
                    "Mock keyboard layer {index} has {} keycodes, expected {} ({rows}x{cols})",
                    layer.len(),
                    rows * cols
                )
                .into());
            }
            let codes = layer
                .iter()
                .map(|name| {
                    resolve_keycode(name)
                        .map_err(|e| format!("Mock keyboard layer {index}: {e}").into())
                })
                .collect::<Result<Vec<u16>, Box<dyn Error>>>()?;
            layers.push(codes);
        }

        Ok(Self {
            definition: fixture.definition,
            layer_states: layer_state_cycle(layers.len()),
            layers,
            tick: AtomicUsize::new(0),
            tick_interval,
        })
    }
}

impl KeyboardProtocol for MockProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn read_keymap(&self) -> Result<KeymapSnapshot, Box<dyn Error>> {
        let (rows, cols) = (self.definition.rows, self.definition.cols);
        let mut actions = vec![vec![vec![None; cols]; rows]; self.layers.len()];

        for (layer, codes) in self.layers.iter().enumerate() {
            for (i, &keycode) in codes.iter().enumerate() {
                let (row, col) = (i / cols, i % cols);
                if row < rows {
                    actions[layer][row][col] = Some(KeyAction::Qmk(keycode));
                }
            }
        }

        Ok(KeymapSnapshot {
            layers: crate::key_action::LayerInfo::indexed(self.layers.len()),
            actions,
        })
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        thread::sleep(self.tick_interval);
        let index = self.tick.fetch_add(1, Ordering::Relaxed) % self.layer_states.len();
        Ok(layer_packet(DEFAULT_LAYER_STATE, self.layer_states[index]))
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
        let keycode = match action {
            KeyAction::Qmk(code) => *code,
            KeyAction::Zmk(_) => return Err("Cannot apply a ZMK behavior to the mock".into()),
        };

        let Some(layer) = self.layers.get_mut(layer_index) else {
            return Err(format!("Mock has no layer {layer_index}").into());
        };
        let index = row * self.definition.cols + col;
        let Some(cell) = layer.get_mut(index) else {
            return Err(format!("Mock key position {row}:{col} is outside the matrix").into());
        };
        *cell = keycode;
        Ok(())
    }
}

/// Builds the layer-change packet that [`crate::keyboard::Keyboard`] expects: a `0xff`
/// marker, the width of `layer_state_t`, then the default and momentary layer bitmasks.
fn layer_packet(default_layer_state: u32, layer_state: u32) -> Vec<u8> {
    let mut packet = vec![0xff, 4];
    packet.extend_from_slice(&default_layer_state.to_le_bytes());
    packet.extend_from_slice(&layer_state.to_le_bytes());
    packet
}

/// Cycles the momentary layers, starting above the base layer so the overlay is visible
/// immediately, then resting on the base layer before repeating.
fn layer_state_cycle(layer_count: usize) -> Vec<u32> {
    let above_base = (1..layer_count as u32).map(momentary_mask);
    above_base.chain(std::iter::once(0)).collect()
}

fn momentary_mask(layer: u32) -> u32 {
    if layer == 0 || layer >= MAX_LAYERS {
        0
    } else {
        1 << layer
    }
}

/// Accepts a QMK keycode name (`KC_A`), a layer shorthand (`MO(1)`), or raw hex (`0x2004`).
fn resolve_keycode(name: &str) -> Result<u16, String> {
    let name = name.trim();

    if let Some(digits) = name.strip_prefix("0x").or_else(|| name.strip_prefix("0X")) {
        return u16::from_str_radix(digits, 16)
            .map_err(|_| format!("invalid hex keycode '{name}'"));
    }

    if let Some(code) = resolve_layer_shorthand(name) {
        return Ok(code);
    }

    keycode_names()
        .get(name)
        .copied()
        .ok_or_else(|| format!("unknown keycode '{name}'"))
}

fn resolve_layer_shorthand(name: &str) -> Option<u16> {
    let (behavior, argument) = name.split_once('(')?;
    let layer: u8 = argument.strip_suffix(')')?.trim().parse().ok()?;

    let op = match behavior.trim() {
        "MO" => QmkLayerOp::Momentary,
        "TO" => QmkLayerOp::To,
        "TG" => QmkLayerOp::Toggle,
        "OSL" => QmkLayerOp::OneShot,
        "TT" => QmkLayerOp::TapToggle,
        "DF" => QmkLayerOp::Default,
        _ => return None,
    };

    op.encode(layer)
}

/// Inverts the `Keycode` enum into a name lookup. `Keycode` exposes number-to-variant
/// conversion and variant-to-name, so walking the numeric range recovers every name
/// without a hand-maintained table.
fn keycode_names() -> &'static HashMap<String, u16> {
    static NAMES: OnceLock<HashMap<String, u16>> = OnceLock::new();
    NAMES.get_or_init(|| {
        (0..=u16::MAX)
            .filter_map(|code| {
                Keycode::try_from(code)
                    .ok()
                    .map(|keycode| (keycode.as_ref().to_string(), code))
            })
            .collect()
    })
}
