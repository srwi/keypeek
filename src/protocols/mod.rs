pub mod kle_parser;
pub mod layout_geometry;
pub mod mock;
pub mod qmk_json_parser;
pub mod via;
pub mod vial;
pub mod zmk;
pub mod zmk_rpc;

use qmk_via_api::api::KeyboardApi;
use std::error::Error;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use self::mock::MockProtocol;
use self::via::ViaProtocol;
use self::vial::VialProtocol;
use self::zmk::ZmkProtocol;

pub use self::zmk_rpc::DeviceLocked;

pub const KEYPEEK_SUBSCRIBE_MARKER: u8 = 0xC0;
pub const KEYPEEK_SUBSCRIBE_ACTIVE: u8 = 0xA1;
pub const KEYPEEK_SUBSCRIBE_INACTIVE: u8 = 0xA0;

/// Writes one dynamic-keymap keycode through the VIA protocol.
///
/// A layer-state packet arriving between `set_key`'s send and its single
/// response read makes the crate report `BadCommandResponse` even though the
/// write usually applied; a matching `get_key` readback confirms success.
pub(crate) fn qmk_set_key_with_retry(
    api: &KeyboardApi,
    layer_index: usize,
    row: usize,
    col: usize,
    code: u16,
) -> Result<(), Box<dyn Error>> {
    match api.set_key(layer_index as u8, row as u8, col as u8, code) {
        Ok(_) => Ok(()),
        Err(qmk_via_api::Error::BadCommandResponse(_)) => {
            for _ in 0..3 {
                thread::sleep(Duration::from_millis(50));
                if let Ok(readback) = api.get_key(layer_index as u8, row as u8, col as u8) {
                    if readback == code {
                        return Ok(());
                    }
                }
            }
            Err("Failed to set key: the device did not confirm the write".into())
        }
        Err(e) => Err(format!("Failed to set key: {e}").into()),
    }
}

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

pub trait KeyboardProtocol: Send {
    fn get_layout_definition(&self) -> &KeyboardDefinition;

    fn read_keymap(&self) -> Result<crate::key_action::KeymapSnapshot, Box<dyn Error>>;

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>>;

    fn write_support(&self) -> WriteSupport {
        WriteSupport::None
    }

    /// Writes one binding. `layer` carries the stable ZMK layer id (`layer_index`
    /// is the position in the layer list, which QMK keys off instead).
    fn set_key(
        &mut self,
        _layer: &crate::key_action::LayerInfo,
        _layer_index: usize,
        _row: usize,
        _col: usize,
        _action: &crate::key_action::KeyAction,
    ) -> Result<(), Box<dyn Error>> {
        Err("not supported".into())
    }

    /// ZMK: persist pending writes. Immediate protocols: `Ok(())`.
    fn save_keymap(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// Opens the transient write session ahead of the first write (ZMK Studio
    /// client), so the first key change does not wait on a connection.
    /// Protocols without a session are already ready.
    fn open_edit_session(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// Closes any transient write connection (ZMK Studio client).
    fn end_edit_session(&mut self) {}

    fn subscription_sender(&self) -> Result<Option<Box<dyn SubscriptionSender>>, Box<dyn Error>> {
        Ok(None)
    }

    fn reopener(&self) -> Option<Arc<dyn Reopener>> {
        None
    }

    fn action_filter(
        &self,
    ) -> Option<Arc<dyn Fn(&crate::key_action::KeyAction) -> bool + Send + Sync>> {
        None
    }
}

pub trait Reopener: Send + Sync {
    fn reopen(&self) -> Result<Box<dyn KeyboardProtocol>, Box<dyn Error>>;
}

pub trait SubscriptionSender: Send {
    fn set_active(&self, active: bool) -> Result<(), Box<dyn Error>>;
}

pub struct RawHidSubscription {
    api: KeyboardApi,
}

impl RawHidSubscription {
    pub fn open(vid: u16, pid: u16) -> Result<Option<Box<dyn SubscriptionSender>>, Box<dyn Error>> {
        let api = KeyboardApi::new(vid, pid, 0xff60, None).map_err(|e| {
            format!(
                "Could not open the RAW HID interface ({vid:04x}:{pid:04x}) to subscribe to \
                 layer events: {e}. The overlay cannot follow layer changes without it."
            )
        })?;
        Ok(Some(Box::new(Self { api })))
    }
}

impl SubscriptionSender for RawHidSubscription {
    fn set_active(&self, active: bool) -> Result<(), Box<dyn Error>> {
        let value = if active {
            KEYPEEK_SUBSCRIBE_ACTIVE
        } else {
            KEYPEEK_SUBSCRIBE_INACTIVE
        };
        self.api
            .hid_send(vec![KEYPEEK_SUBSCRIBE_MARKER, value])
            .map_err(|e| format!("Subscription keepalive write error: {e}").into())
    }
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

pub fn connect_protocol(
    spec: &ConnectionSpec,
) -> Result<Box<dyn KeyboardProtocol>, Box<dyn Error>> {
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
