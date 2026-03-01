pub mod kle_parser;
pub mod qmk_json_parser;
pub mod via;
pub mod vial;
pub mod zmk;
pub mod zmk_rpc;

use crate::layout_key::LayoutKey;
use crate::settings::ProtocolType;
use std::error::Error;

use self::via::ViaProtocol;
use self::vial::VialProtocol;
use self::zmk::ZmkProtocol;

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

pub trait KeyboardProtocol: Send {
    fn get_layout_definition(&self) -> &KeyboardDefinition;

    fn get_layer_count(&self) -> Result<usize, Box<dyn Error>>;

    fn read_all_keys(
        &self,
        layers: usize,
        rows: usize,
        cols: usize,
    ) -> Vec<Vec<Vec<Option<LayoutKey>>>>;

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>>;
}

pub fn parse_vid_pid(s: &str) -> Result<(u16, u16), Box<dyn Error>> {
    let (vid_str, pid_str) = s
        .split_once(':')
        .ok_or_else(|| format!("Invalid VID:PID format: '{s}'"))?;
    let vid =
        u16::from_str_radix(vid_str, 16).map_err(|e| format!("Invalid VID '{vid_str}': {e}"))?;
    let pid =
        u16::from_str_radix(pid_str, 16).map_err(|e| format!("Invalid PID '{pid_str}': {e}"))?;
    Ok((vid, pid))
}

pub fn format_vid_pid(vid: u16, pid: u16) -> String {
    format!("{:04x}:{:04x}", vid, pid)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZmkTransportConfig {
    Serial(String),
    Ble(String),
}

pub fn parse_zmk_config(config: &str) -> Result<(u16, u16, ZmkTransportConfig), Box<dyn Error>> {
    let (vid_pid, transport_raw) = config.split_once('|').ok_or(
        "Invalid ZMK config format: expected 'vid:pid|serial:<port>' or 'vid:pid|ble:<device_id>'",
    )?;
    let (vid, pid) = parse_vid_pid(vid_pid)?;

    let (transport_kind, transport_value) = transport_raw
        .split_once(':')
        .ok_or("Invalid ZMK transport format: expected 'serial:<port>' or 'ble:<device_id>'")?;
    if transport_value.is_empty() {
        return Err("ZMK transport value cannot be empty".into());
    }

    let transport = match transport_kind {
        "serial" => ZmkTransportConfig::Serial(transport_value.to_string()),
        "ble" => ZmkTransportConfig::Ble(transport_value.to_string()),
        _ => {
            return Err(format!(
                "Invalid ZMK transport kind '{transport_kind}', expected 'serial' or 'ble'"
            )
            .into())
        }
    };

    Ok((vid, pid, transport))
}

pub fn format_zmk_config(vid: u16, pid: u16, transport: &ZmkTransportConfig) -> String {
    match transport {
        ZmkTransportConfig::Serial(port) => format!("{:04x}:{:04x}|serial:{port}", vid, pid),
        ZmkTransportConfig::Ble(device_id) => format!("{:04x}:{:04x}|ble:{device_id}", vid, pid),
    }
}

pub fn connect_protocol(
    protocol_type: ProtocolType,
    protocol_config: &str,
) -> Result<Box<dyn KeyboardProtocol>, Box<dyn Error>> {
    match protocol_type {
        ProtocolType::Via => {
            let protocol = ViaProtocol::connect(protocol_config)?;
            Ok(Box::new(protocol))
        }
        ProtocolType::Vial => {
            let (vid, pid) = parse_vid_pid(protocol_config)?;
            let protocol = VialProtocol::connect(vid, pid)?;
            Ok(Box::new(protocol))
        }
        ProtocolType::Zmk => {
            let (vid, pid, transport) = parse_zmk_config(protocol_config)?;
            let zmk_transport = match transport {
                ZmkTransportConfig::Serial(port_name) => {
                    zmk_rpc::ZmkTransport::SerialPort(port_name)
                }
                ZmkTransportConfig::Ble(device_id) => zmk_rpc::ZmkTransport::BleDevice(device_id),
            };
            let zmk_data = zmk_rpc::fetch_zmk_data(&zmk_transport)?;
            let protocol = ZmkProtocol::connect_live(vid, pid, &zmk_data)?;
            Ok(Box::new(protocol))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{format_zmk_config, parse_zmk_config, ZmkTransportConfig};

    #[test]
    fn parse_zmk_serial_config() {
        let (vid, pid, transport) = parse_zmk_config("1234:abcd|serial:/dev/ttyACM0").unwrap();
        assert_eq!(vid, 0x1234);
        assert_eq!(pid, 0xabcd);
        assert_eq!(
            transport,
            ZmkTransportConfig::Serial("/dev/ttyACM0".to_string())
        );
    }

    #[test]
    fn parse_zmk_ble_config() {
        let (vid, pid, transport) = parse_zmk_config("1111:2222|ble:peripheral-1").unwrap();
        assert_eq!(vid, 0x1111);
        assert_eq!(pid, 0x2222);
        assert_eq!(
            transport,
            ZmkTransportConfig::Ble("peripheral-1".to_string())
        );
    }

    #[test]
    fn parse_zmk_invalid_config_rejects_untagged() {
        assert!(parse_zmk_config("1234:5678|COM3").is_err());
    }

    #[test]
    fn parse_zmk_invalid_transport_kind() {
        assert!(parse_zmk_config("1234:5678|usb:abc").is_err());
    }

    #[test]
    fn format_zmk_configs() {
        let serial = format_zmk_config(
            0x1234,
            0xabcd,
            &ZmkTransportConfig::Serial("COM3".to_string()),
        );
        let ble = format_zmk_config(
            0x1234,
            0xabcd,
            &ZmkTransportConfig::Ble("device-123".to_string()),
        );
        assert_eq!(serial, "1234:abcd|serial:COM3");
        assert_eq!(ble, "1234:abcd|ble:device-123");
    }
}
