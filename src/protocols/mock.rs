//! A virtual keyboard backed by a checked-in fixture, for developing without hardware.
//!
//! It reports a fixed layout and keymap, and cycles its momentary layer state on a timer
//! so layer-change rendering can be exercised. The mock device is only registered
//! during discovery in debug builds (`cfg!(debug_assertions)` in `device_discovery`).

use super::{KeyboardDefinition, KeyboardProtocol, WriteSupport};
use crate::key_action::{KeyAction, KeymapSnapshot};
use crate::qmk_keycode_labels::constants::{
    QK_DEF_LAYER, QK_LAYER_TAP_TOGGLE, QK_MOMENTARY, QK_ONE_SHOT_LAYER, QK_TO, QK_TOGGLE_LAYER,
};
use qmk_via_api::keycodes::Keycode;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Range;
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
    let layer: u16 = argument.strip_suffix(')')?.trim().parse().ok()?;

    let range: Range<u16> = match behavior.trim() {
        "MO" => QK_MOMENTARY,
        "TO" => QK_TO,
        "TG" => QK_TOGGLE_LAYER,
        "OSL" => QK_ONE_SHOT_LAYER,
        "TT" => QK_LAYER_TAP_TOGGLE,
        "DF" => QK_DEF_LAYER,
        _ => return None,
    };

    let code = range.start.checked_add(layer)?;
    range.contains(&code).then_some(code)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_action::{KeyAction, LayerInfo};
    use crate::key_matrix::KeyMatrix;
    use crate::keyboard::{Keyboard, OverlayConfig};
    use crate::protocols::WriteSupport;
    use crate::qmk_keycode_labels::get_layout_key;
    use crate::ui_wake::UiWake;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::time::Instant;

    /// Layer 0, index 39 (row 3, col 3) holds `MO(1)` in the fixture.
    const FIXTURE_MO_KEY: (usize, usize, usize) = (0, 3, 3);

    #[test]
    fn mo_fixture_key_resolves_action_and_label() {
        let mock = MockProtocol::connect().expect("fixture should load");
        let snapshot = mock.read_keymap().unwrap();
        let definition = mock.get_layout_definition();
        let matrix = KeyMatrix::from_snapshot(snapshot, definition.rows, definition.cols);

        let (layer, row, col) = FIXTURE_MO_KEY;
        assert_eq!(
            matrix.get_action(layer, row, col),
            Some(&KeyAction::Qmk(0x5221))
        );
        assert_eq!(
            matrix.get_key(layer, row, col).cloned(),
            get_layout_key(0x5221)
        );
        assert_eq!(
            matrix.layer_infos(),
            LayerInfo::indexed(matrix.get_num_layers()).as_slice()
        );
    }

    /// Sends a write through the real `Keyboard` actor and waits for its result.
    fn write_via_actor(
        keyboard: &Keyboard,
        layer_index: usize,
        row: usize,
        col: usize,
        action: KeyAction,
    ) -> Result<(), String> {
        keyboard
            .set_key(layer_index, row, col, action)
            .recv_timeout(Duration::from_secs(10))
            .expect("actor thread should not drop the result receiver")
    }

    /// Layer 1 is all-transparent in the fixture, so every cell there is a
    /// binding slot whose label starts out `None`.
    #[test]
    fn writes_update_action_and_label_through_the_actor() {
        let protocol = MockProtocol::with_tick_interval(Duration::from_millis(5)).unwrap();
        let layout_name = protocol.get_layout_definition().layouts[0].name.clone();

        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&wakes);
        let ui_wake = UiWake::new(Arc::new(move || {
            counter.fetch_add(1, Ordering::Relaxed);
        }));

        let config = OverlayConfig {
            timeout_ms: -1,
            activation_delay_ms: 0,
            visible_layers: u32::MAX,
        };
        let keyboard = Keyboard::new(Box::new(protocol), layout_name, config, ui_wake).unwrap();
        assert_eq!(keyboard.write_support(), WriteSupport::Immediate);

        // A plain keycode.
        let code = Keycode::KC_A as u16;
        assert_eq!(
            write_via_actor(&keyboard, 1, 0, 0, KeyAction::Qmk(code)),
            Ok(())
        );
        assert_eq!(keyboard.get_action(1, 0, 0), Some(KeyAction::Qmk(code)));
        assert_eq!(keyboard.get_key(1, 0, 0), get_layout_key(code));

        // A momentary-layer key.
        let mo_code = QK_MOMENTARY.start + 2;
        assert_eq!(
            write_via_actor(&keyboard, 1, 0, 1, KeyAction::Qmk(mo_code)),
            Ok(())
        );
        assert_eq!(keyboard.get_action(1, 0, 1), Some(KeyAction::Qmk(mo_code)));
        assert_eq!(keyboard.get_key(1, 0, 1), get_layout_key(mo_code));

        // A transparent binding: no label anymore, but still an editable slot.
        let transparent_code = Keycode::KC_TRANSPARENT as u16;
        assert_eq!(
            write_via_actor(&keyboard, 1, 0, 1, KeyAction::Qmk(transparent_code)),
            Ok(())
        );
        assert_eq!(
            keyboard.get_action(1, 0, 1),
            Some(KeyAction::Qmk(transparent_code))
        );
        assert_eq!(keyboard.get_key(1, 0, 1), None);

        // Out-of-range writes fail with an error instead of panicking.
        assert!(write_via_actor(&keyboard, 9, 0, 0, KeyAction::Qmk(code)).is_err());
        assert!(keyboard.get_action(9, 0, 0).is_none());

        // The mock persists immediately; save is a no-op and discard is unsupported,
        // both reported through the same receiver pattern.
        let _ = keyboard.save_keymap().recv_timeout(Duration::from_secs(10));
        assert!(keyboard
            .discard_keymap()
            .recv_timeout(Duration::from_secs(10))
            .expect("discard receiver should resolve")
            .is_err());
        assert!(wakes.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn fixture_loads_and_matches_its_matrix() {
        let mock = MockProtocol::connect().expect("fixture should load");
        let definition = mock.get_layout_definition();

        assert!(mock.layers.len() > 1);
        for layer in &mock.layers {
            assert_eq!(layer.len(), definition.rows * definition.cols);
        }
        assert!(!definition.layouts.is_empty());
    }

    #[test]
    fn fixture_layouts_stay_inside_the_matrix() {
        let mock = MockProtocol::connect().unwrap();
        let definition = mock.get_layout_definition();

        for layout in &definition.layouts {
            for key in &layout.keys {
                assert!(
                    key.row < definition.rows && key.col < definition.cols,
                    "layout '{}' key ({}, {}) is outside the {}x{} matrix",
                    layout.name,
                    key.row,
                    key.col,
                    definition.rows,
                    definition.cols
                );
            }
        }
    }

    #[test]
    fn resolves_names_shorthands_and_hex() {
        assert_eq!(resolve_keycode("KC_A"), Ok(Keycode::KC_A as u16));
        assert_eq!(resolve_keycode(" KC_ENTER "), Ok(Keycode::KC_ENTER as u16));
        assert_eq!(resolve_keycode("MO(1)"), Ok(QK_MOMENTARY.start + 1));
        assert_eq!(resolve_keycode("TO(2)"), Ok(QK_TO.start + 2));
        assert_eq!(resolve_keycode("0x2004"), Ok(0x2004));
        assert!(resolve_keycode("KC_NOPE").is_err());
        assert!(resolve_keycode("MO(999)").is_err());
    }

    #[test]
    fn layer_packet_matches_the_firmware_wire_format() {
        let packet = layer_packet(1, 0b100);
        assert_eq!(packet, vec![0xff, 4, 1, 0, 0, 0, 0b100, 0, 0, 0]);
    }

    #[test]
    fn cycle_starts_above_the_base_layer_and_returns_to_it() {
        assert_eq!(layer_state_cycle(3), vec![0b10, 0b100, 0]);
        assert_eq!(layer_state_cycle(1), vec![0]);
    }

    #[test]
    fn momentary_mask_ignores_unrepresentable_layers() {
        assert_eq!(momentary_mask(0), 0);
        assert_eq!(momentary_mask(31), 1 << 31);
        assert_eq!(momentary_mask(32), 0);
    }

    /// Guards the packet format against `Keyboard`'s real parser rather than against this
    /// module's own idea of it: a malformed packet would leave the effective layer stuck.
    #[test]
    fn cycling_layers_reaches_the_keyboard() {
        let protocol = MockProtocol::with_tick_interval(Duration::from_millis(10)).unwrap();
        let layout = protocol.get_layout_definition().layouts[0].name.clone();

        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&wakes);
        let ui_wake = UiWake::new(Arc::new(move || {
            counter.fetch_add(1, Ordering::Relaxed);
        }));

        // A negative timeout means "never hide", keeping the overlay timer out of the way.
        let config = OverlayConfig {
            timeout_ms: -1,
            activation_delay_ms: 0,
            visible_layers: u32::MAX,
        };
        let keyboard = Keyboard::new(Box::new(protocol), layout, config, ui_wake).unwrap();

        // Row 0, col 1 is mapped on every layer in the fixture, so the effective layer
        // there is exactly the layer the mock currently reports.
        let mut seen = HashSet::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && seen.len() < 3 {
            seen.insert(keyboard.get_effective_key_layer(0, 1).0);
            thread::sleep(Duration::from_millis(1));
        }

        assert_eq!(
            seen,
            HashSet::from([0, 1, 2]),
            "cycled layers seen: {seen:?}"
        );
        assert!(wakes.load(Ordering::Relaxed) > 0, "no repaints requested");
        assert!(keyboard.is_alive());
    }
}
