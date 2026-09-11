pub mod kle_parser;
pub mod layout_geometry;
pub mod mock;
pub mod qmk_codec;
pub mod qmk_common;
pub mod qmk_discovery;
pub mod qmk_json_parser;
pub(crate) mod qmk_keycode_labels;
pub mod via;
pub mod vial;
pub mod zmk;
pub mod zmk_codec;
pub mod zmk_discovery;
pub(crate) mod zmk_keycode_labels;
pub mod zmk_rpc;

use std::error::Error;
use std::fmt;
use std::sync::{mpsc, Arc};

use self::mock::MockProtocol;
use self::via::ViaProtocol;
use self::vial::VialProtocol;
use self::zmk::ZmkProtocol;

/// Unified domain error for keyboard communication, configuration, and driver operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    /// Device is locked (e.g. an unlock key combination is required).
    DeviceLocked,
    /// Physical or transport connection error (e.g. serial port, BLE, HID I/O).
    Transport(String),
    /// Protocol communication error or unexpected payload.
    Protocol(String),
    /// Feature, operation, or binding not supported by this keyboard or protocol.
    Unsupported(String),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceLocked => write!(
                f,
                "Device is locked. Unlock it on the keyboard (e.g. with its unlock key \
                 combination), then try again."
            ),
            Self::Transport(msg) => write!(f, "Transport error: {msg}"),
            Self::Protocol(msg) => write!(f, "Protocol error: {msg}"),
            Self::Unsupported(msg) => write!(f, "Unsupported: {msg}"),
        }
    }
}

impl Error for DeviceError {}

impl From<Box<dyn Error>> for DeviceError {
    fn from(err: Box<dyn Error>) -> Self {
        if let Some(device_err) = err.downcast_ref::<DeviceError>() {
            return device_err.clone();
        }
        Self::Protocol(err.to_string())
    }
}

impl From<String> for DeviceError {
    fn from(msg: String) -> Self {
        Self::Protocol(msg)
    }
}

impl From<&str> for DeviceError {
    fn from(msg: &str) -> Self {
        Self::Protocol(msg.to_string())
    }
}

/// Strongly-typed events emitted by a keyboard driver or protocol adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    /// Layer bitmasks changed.
    LayersChanged {
        active_layers: u32,
        default_layers: u32,
    },
    /// A physical key at (row, col) was pressed or released.
    KeyPressed {
        row: usize,
        col: usize,
        pressed: bool,
    },
    /// The physical connection was dropped.
    Disconnected(String),
}

/// A layer packet's size field is `sizeof(layer_state_t)` and at most 4 bytes.
const MAX_LAYER_STATE_BYTES: usize = 4;
/// Leading byte of a layer-state packet, followed by a size and two bitmasks.
const LAYER_STATE_PACKET: u8 = 0xff;
/// Leading byte of a key event packet, followed by `row`, `col`, `pressed`.
const KEY_EVENT_PACKET: u8 = 0xF1;

/// Decodes raw HID packets emitted by KeyPeek companion firmware modules
/// (QMK/Vial `srwi/keypeek_layer_notify` and ZMK raw-HID adapter).
pub fn decode_raw_hid_packet(response: &[u8]) -> Option<DeviceEvent> {
    match response.first().copied() {
        Some(LAYER_STATE_PACKET) if response.len() >= 2 => {
            let size = response[1] as usize;
            if size != 0 && size <= MAX_LAYER_STATE_BYTES && 2 + 2 * size <= response.len() {
                let mut default_bytes = [0u8; 4];
                default_bytes[..size].copy_from_slice(&response[2..2 + size]);
                let default_layers = u32::from_le_bytes(default_bytes);

                let mut layer_bytes = [0u8; 4];
                layer_bytes[..size].copy_from_slice(&response[2 + size..2 + 2 * size]);
                let active_layers = u32::from_le_bytes(layer_bytes);

                Some(DeviceEvent::LayersChanged {
                    active_layers,
                    default_layers,
                })
            } else {
                None
            }
        }
        Some(KEY_EVENT_PACKET) if response.len() >= 4 => {
            let row = response[1] as usize;
            let col = response[2] as usize;
            let pressed = response[3] != 0;
            Some(DeviceEvent::KeyPressed { row, col, pressed })
        }
        _ => None,
    }
}

/// Pumps raw HID responses through `decode_raw_hid_packet` and sends resulting `DeviceEvent`s.
/// Retries on transient errors and emits `DeviceEvent::Disconnected` after consecutive failures.
pub fn pump_hid_reader<F>(mut read: F, event_tx: mpsc::Sender<DeviceEvent>, disconnect_msg: &str)
where
    F: FnMut() -> Result<Option<Vec<u8>>, String>,
{
    const MAX_CONSECUTIVE_ERRORS: u32 = 5;
    let mut consecutive_errors: u32 = 0;
    loop {
        match read() {
            Ok(Some(bytes)) => {
                consecutive_errors = 0;
                if let Some(event) = decode_raw_hid_packet(&bytes) {
                    if event_tx.send(event).is_err() {
                        break;
                    }
                }
            }
            Ok(None) => {
                // Timeout / no data, continue
            }
            Err(_) => {
                consecutive_errors += 1;
                if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    let _ = event_tx.send(DeviceEvent::Disconnected(disconnect_msg.to_string()));
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    }
}

pub type ActionFilter = Arc<dyn Fn(&crate::key_spec::KeySpec) -> bool + Send + Sync>;

pub type Row = usize;
pub type Column = usize;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Key {
    pub row: Row,
    pub col: Column,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Rotation angle in degrees, clockwise around the key's center.
    #[serde(default)]
    pub r: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyboardLayout {
    pub name: String,
    pub keys: Vec<Key>,
}

impl KeyboardLayout {
    pub fn get_dimensions(&self) -> (f32, f32) {
        let max_x = self.keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
        let max_y = self.keys.iter().map(|k| k.y + k.h).fold(0.0, f32::max);
        (max_x, max_y)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyboardDefinition {
    pub vid: u16,
    pub pid: u16,
    pub rows: usize,
    pub cols: usize,
    pub layouts: Vec<KeyboardLayout>,
}

impl KeyboardDefinition {
    pub fn get_layout_names(&self) -> Vec<String> {
        self.layouts.iter().map(|l| l.name.clone()).collect()
    }

    pub fn get_layout(&self, layout_name: &str) -> Result<KeyboardLayout, String> {
        self.layouts
            .iter()
            .find(|l| l.name == layout_name)
            .cloned()
            .ok_or_else(|| format!("Layout '{}' not found.", layout_name))
    }
}

/// How a protocol persists keymap writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteSupport {
    /// The protocol cannot write keymaps.
    None,
    /// Every write persists at once (QMK/Vial/mock).
    Immediate,
    /// Writes live in RAM until `save_keymap` persists them (ZMK).
    Session,
}

/// Which firmware vocabulary the device's users are familiar with, for UI
/// labels where firmware communities use different names for the same concept
/// (e.g. QMK "One-Shot Mod" vs ZMK "Sticky Key"). `Neutral` uses firmware-
/// agnostic wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Terminology {
    #[default]
    Neutral,
    Qmk,
    Zmk,
}

pub trait KeyboardProtocol: Send {
    fn get_layout_definition(&self) -> &KeyboardDefinition;

    fn read_keymap(&self) -> Result<crate::key_spec::KeymapSnapshot, DeviceError>;

    /// Subscribes to live layer-state and key-press events emitted by the device.
    /// The adapter manages its own background reading and keepalive heartbeats.
    fn subscribe_events(&mut self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError>;

    fn write_support(&self) -> WriteSupport {
        WriteSupport::None
    }

    /// Writes one binding. `layer` carries the stable ZMK layer id (`layer_index`
    /// is the position in the layer list, which QMK keys off instead).
    fn set_key(
        &mut self,
        _layer: &crate::key_spec::LayerInfo,
        _layer_index: usize,
        _row: usize,
        _col: usize,
        _spec: &crate::key_spec::KeySpec,
    ) -> Result<(), DeviceError> {
        Err(DeviceError::Unsupported("write not supported".to_string()))
    }

    /// ZMK: persist pending writes. Immediate protocols: `Ok(())`.
    fn save_keymap(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// Opens the transient write session ahead of the first write (ZMK Studio
    /// client), so the first key change does not wait on a connection.
    /// Protocols without a session are already ready.
    fn open_edit_session(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// Closes any transient write connection (ZMK Studio client).
    fn end_edit_session(&mut self) {}

    fn reopener(&self) -> Option<Arc<dyn Reopener>> {
        None
    }

    fn action_filter(&self) -> Option<ActionFilter> {
        None
    }

    /// Whether the device accepts raw firmware keycodes as hex input.
    fn supports_raw_keycode_entry(&self) -> bool {
        false
    }

    /// Whether the layout can be switched while connected.
    fn supports_live_layout_switching(&self) -> bool {
        false
    }

    /// The firmware vocabulary users of this device are familiar with.
    fn terminology(&self) -> Terminology {
        Terminology::Neutral
    }

    /// Parses a raw firmware keycode into a domain [`KeySpec`]. Only called
    /// when [`KeyboardProtocol::supports_raw_keycode_entry`] is `true`.
    fn parse_raw_keycode(&self, _code: u16) -> Option<crate::key_spec::KeySpec> {
        None
    }
}

pub trait Reopener: Send + Sync {
    fn reopen(&self) -> Result<Box<dyn KeyboardProtocol>, DeviceError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZmkTransportConfig {
    Serial(String),
    Ble(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionSpec {
    Via {
        json_path: String,
    },
    Vial {
        vid: u16,
        pid: u16,
    },
    Zmk {
        vid: u16,
        pid: u16,
        transport: ZmkTransportConfig,
    },
    Mock,
}

pub fn connect_protocol(spec: &ConnectionSpec) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
    match spec {
        ConnectionSpec::Via { json_path } => {
            let protocol = ViaProtocol::connect(json_path)?;
            Ok(Box::new(protocol))
        }
        ConnectionSpec::Vial { vid, pid } => {
            let protocol = VialProtocol::connect(*vid, *pid)?;
            Ok(Box::new(protocol))
        }
        ConnectionSpec::Zmk {
            vid,
            pid,
            transport,
        } => {
            let zmk_transport = match transport {
                ZmkTransportConfig::Serial(port_name) => {
                    zmk_rpc::ZmkTransport::SerialPort(port_name.clone())
                }
                ZmkTransportConfig::Ble(device_id) => {
                    zmk_rpc::ZmkTransport::BleDevice(device_id.clone())
                }
            };
            let protocol = ZmkProtocol::connect_live(*vid, *pid, &zmk_transport)?;
            Ok(Box::new(protocol))
        }
        ConnectionSpec::Mock => {
            let protocol = MockProtocol::connect()?;
            Ok(Box::new(protocol))
        }
    }
}

/// Returns all known protocol-specific search tokens for a standard HID usage.
pub fn protocol_search_tokens_for_hid(page: u16, id: u16) -> Vec<String> {
    let mut tokens = Vec::new();
    tokens.extend(qmk_codec::qmk_search_tokens_for_hid(page, id));
    tokens.extend(zmk_codec::zmk_search_tokens_for_hid(page, id));
    tokens
}

/// Enumerates all USB HID keyboard usages (Page 0x07) supported across protocols.
pub fn all_keyboard_usages() -> Vec<u16> {
    let mut set = std::collections::BTreeSet::new();
    for id in zmk_codec::zmk_all_keyboard_usages() {
        set.insert(id);
    }
    for id in qmk_codec::qmk_all_basic_usages() {
        set.insert(id);
    }
    set.into_iter().collect()
}

/// Enumerates all USB HID consumer usages (Page 0x0C) supported across protocols.
pub fn all_consumer_usages() -> Vec<u16> {
    let mut set = std::collections::BTreeSet::new();
    for id in zmk_codec::zmk_all_consumer_usages() {
        set.insert(id);
    }
    for id in qmk_codec::qmk_all_media_usages() {
        set.insert(id);
    }
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_layer_state_packet() {
        // Valid 0xff layer packet: [0xff, size=4, default_state (le), layer_state (le)]
        let mut packet = vec![0xff, 4];
        packet.extend_from_slice(&1u32.to_le_bytes()); // default layer = 1
        packet.extend_from_slice(&4u32.to_le_bytes()); // active layer = 4

        let event = decode_raw_hid_packet(&packet);
        assert_eq!(
            event,
            Some(DeviceEvent::LayersChanged {
                active_layers: 4,
                default_layers: 1,
            })
        );
    }

    #[test]
    fn test_decode_layer_state_invalid_size() {
        // Size 0 is invalid
        let packet = vec![0xff, 0, 1, 0, 0, 0, 4, 0, 0, 0];
        assert_eq!(decode_raw_hid_packet(&packet), None);

        // Size > 4 is invalid
        let packet = vec![0xff, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(decode_raw_hid_packet(&packet), None);

        // Truncated buffer
        let packet = vec![0xff, 4, 1, 0];
        assert_eq!(decode_raw_hid_packet(&packet), None);
    }

    #[test]
    fn test_decode_key_event_packet() {
        // Key press: [0xF1, row, col, pressed]
        let packet = vec![0xF1, 2, 5, 1];
        assert_eq!(
            decode_raw_hid_packet(&packet),
            Some(DeviceEvent::KeyPressed {
                row: 2,
                col: 5,
                pressed: true,
            })
        );

        // Key release
        let packet = vec![0xF1, 2, 5, 0];
        assert_eq!(
            decode_raw_hid_packet(&packet),
            Some(DeviceEvent::KeyPressed {
                row: 2,
                col: 5,
                pressed: false,
            })
        );
    }

    #[test]
    fn test_decode_unknown_packet() {
        let packet = vec![0x00, 1, 2, 3];
        assert_eq!(decode_raw_hid_packet(&packet), None);

        let empty: [u8; 0] = [];
        assert_eq!(decode_raw_hid_packet(&empty), None);
    }
}
