//! A virtual keyboard backed by a checked-in fixture, for developing without hardware.
//!
//! It reports a fixed layout and keymap, and cycles its momentary layer state on a timer
//! so layer-change rendering can be exercised. The mock device is only registered
//! during discovery in debug builds (`cfg!(debug_assertions)` in `device_discovery`).

use crate::layout::KeyboardDefinition;
use crate::protocols::{ConnectionSpec, DeviceError, DeviceEvent, KeyboardProtocol, WriteSupport};
use crate::key_spec::{KeySpec, KeymapSnapshot, LayerInfo};
use qmk_via_api::keycodes::Keycode;
use qmk_via_api::QmkLayerOp;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

const FIXTURE: &str = include_str!("../../../resources/mock_keyboard.json");

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
    tick_interval: Duration,
}

impl MockProtocol {
    pub fn connect() -> Result<Self, DeviceError> {
        Self::with_tick_interval(TICK_INTERVAL)
    }

    /// The interval is a parameter so tests can cycle layers without waiting on the
    /// human-paced default.
    fn with_tick_interval(tick_interval: Duration) -> Result<Self, DeviceError> {
        let fixture: MockFixture = serde_json::from_str(FIXTURE)
            .map_err(|e| DeviceError::Protocol(format!("Invalid mock keyboard fixture: {e}")))?;

        let (rows, cols) = (fixture.definition.rows, fixture.definition.cols);
        if rows == 0 || cols == 0 {
            return Err(DeviceError::Protocol(
                "Mock keyboard fixture has an empty matrix".to_string(),
            ));
        }
        if fixture.layers.is_empty() {
            return Err(DeviceError::Protocol(
                "Mock keyboard fixture has no layers".to_string(),
            ));
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
                    resolve_keycode(name).map_err(|e| {
                        DeviceError::Protocol(format!("Mock keyboard layer {index}: {e}"))
                    })
                })
                .collect::<Result<Vec<u16>, DeviceError>>()?;
            layers.push(codes);
        }

        Ok(Self {
            definition: fixture.definition,
            layer_states: layer_state_cycle(layers.len()),
            layers,
            tick_interval,
        })
    }
}

impl KeyboardProtocol for MockProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn read_keymap(&self) -> Result<KeymapSnapshot, DeviceError> {
        let (rows, cols) = (self.definition.rows, self.definition.cols);
        let mut actions = vec![vec![vec![None; cols]; rows]; self.layers.len()];

        for (layer, codes) in self.layers.iter().enumerate() {
            for (i, &keycode) in codes.iter().enumerate() {
                let (row, col) = (i / cols, i % cols);
                if row < rows {
                    actions[layer][row][col] = Some(crate::firmware::qmk::codec::qmk_to_keyspec(keycode));
                }
            }
        }

        Ok(KeymapSnapshot {
            layers: LayerInfo::indexed(self.layers.len()),
            actions,
        })
    }

    fn subscribe_events(&mut self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError> {
        let (event_tx, event_rx) = mpsc::channel();
        let tick_interval = self.tick_interval;
        let layer_states = self.layer_states.clone();

        thread::spawn(move || {
            let mut tick = 0;
            loop {
                thread::sleep(tick_interval);
                let index = tick % layer_states.len();
                tick += 1;
                let event = DeviceEvent::LayersChanged {
                    active_layers: layer_states[index],
                    default_layers: DEFAULT_LAYER_STATE,
                };
                if event_tx.send(event).is_err() {
                    break;
                }
            }
        });

        Ok(event_rx)
    }

    fn write_support(&self) -> WriteSupport {
        WriteSupport::Immediate
    }

    fn supports_live_layout_switching(&self) -> bool {
        true
    }

    fn set_key(
        &mut self,
        _layer: &LayerInfo,
        layer_index: usize,
        row: usize,
        col: usize,
        spec: &KeySpec,
    ) -> Result<(), DeviceError> {
        let keycode = crate::firmware::qmk::codec::keyspec_to_qmk(spec)?;

        let Some(layer) = self.layers.get_mut(layer_index) else {
            return Err(DeviceError::Protocol(format!(
                "Mock has no layer {layer_index}"
            )));
        };
        let index = row * self.definition.cols + col;
        let Some(cell) = layer.get_mut(index) else {
            return Err(DeviceError::Protocol(format!(
                "Mock key position {row}:{col} is outside the matrix"
            )));
        };
        *cell = keycode;
        Ok(())
    }
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

/// Identifiers for the virtual keyboard. They must match `resources/mock_keyboard.json`
/// and are deliberately outside the ranges real boards use.
pub const MOCK_VID: u16 = 0xF00D;
pub const MOCK_PID: u16 = 0xF00D;

/// Constructs the mock virtual keyboard descriptor.
pub fn mock_device() -> crate::device_discovery::DiscoveredDevice {
    crate::device_discovery::DiscoveredDevice {
        base_name: "Virtual Keyboard".to_string(),
        vid: MOCK_VID,
        pid: MOCK_PID,
        driver_id: "mock",
        protocol_label: "Mock",
        requires_layout_file: false,
        spec: ConnectionSpec::Mock,
    }
}

/// Scanner for the virtual/mock keyboard.
pub struct MockScanner;

impl crate::device_discovery::DeviceDriverScanner for MockScanner {
    fn scan(
        &self,
        _ctx: &mut crate::device_discovery::DiscoveryContext,
    ) -> Vec<crate::device_discovery::DiscoveredDevice> {
        vec![mock_device()]
    }
}
