pub mod kle_parser;
pub mod qmk_json_parser;
pub mod via;
pub mod vial;
pub mod zmk;
pub mod zmk_studio;

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

fn parse_zmk_config(config: &str) -> Result<(u16, u16, &str), Box<dyn Error>> {
    let (vid_pid, serial_port) = config
        .split_once('|')
        .ok_or("Invalid ZMK config format: expected 'vid:pid|serial_port'")?;
    let (vid, pid) = parse_vid_pid(vid_pid)?;
    Ok((vid, pid, serial_port))
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
            let (vid, pid, _serial_port) = parse_zmk_config(protocol_config)?;
            match ZmkProtocol::connect_cached(vid, pid) {
                Ok(protocol) => Ok(Box::new(protocol)),
                Err(_) => Err(
                    "No cached ZMK data. Use the settings window to connect via ZMK Studio.".into(),
                ),
            }
        }
    }
}
