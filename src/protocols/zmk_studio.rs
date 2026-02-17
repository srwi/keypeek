use crate::layout_key::LayoutKey;
use crate::zmk_keycode_labels::behavior_to_layout_key;
use std::error::Error;
use std::time::Duration;
use zmk_studio_api::proto::zmk::{core, keymap};
use zmk_studio_api::{Behavior, StudioClient};

pub struct ZmkSerialDevice {
    pub port_name: String,
    pub vid: u16,
    pub pid: u16,
    pub product: Option<String>,
}

pub fn scan_serial_ports() -> Vec<ZmkSerialDevice> {
    let Ok(ports) = serialport::available_ports() else {
        return Vec::new();
    };

    ports
        .into_iter()
        .filter_map(|p| {
            if let serialport::SerialPortType::UsbPort(usb) = &p.port_type {
                Some(ZmkSerialDevice {
                    port_name: p.port_name,
                    vid: usb.vid,
                    pid: usb.pid,
                    product: usb.product.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub struct StudioData {
    pub physical_layouts: keymap::PhysicalLayouts,
    pub layout_keys: Vec<Vec<Option<LayoutKey>>>,
    pub layer_count: usize,
}

pub fn fetch_studio_data(port_name: &str) -> Result<StudioData, Box<dyn Error>> {
    let mut client = StudioClient::open_serial(port_name)
        .map_err(|e| format!("Failed to open serial port '{}': {}", port_name, e))?;

    let lock_state = client.get_lock_state()?;
    if lock_state == core::LockState::ZmkStudioCoreLockStateLocked {
        drop(client);
        return Err("DEVICE_LOCKED".into());
    }

    let physical_layouts = client.get_physical_layouts()?;

    let resolved_layers: Vec<Vec<Behavior>> = client.resolve_keymap()?;
    let layer_count = resolved_layers.len();

    let layout_keys: Vec<Vec<Option<LayoutKey>>> = resolved_layers
        .iter()
        .map(|layer| layer.iter().map(behavior_to_layout_key).collect())
        .collect();

    // Drop the serial connection and give USB time to settle before
    // the caller opens any other handle (e.g. HID).
    drop(client);
    std::thread::sleep(Duration::from_millis(100));

    Ok(StudioData {
        physical_layouts,
        layout_keys,
        layer_count,
    })
}
