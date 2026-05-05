use crate::layout_key::LayoutKey;
use crate::zmk_keycode_labels::behavior_to_layout_key;
use std::error::Error;
use std::io::{Read, Write};
use std::time::Duration;
use zmk_studio_api::proto::zmk::{core, keymap};
use zmk_studio_api::transport::{BleDiscoveryMode, PlatformBleTransport};
use zmk_studio_api::{Behavior, StudioClient};

pub struct ZmkSerialDevice {
    pub port_name: String,
    pub vid: u16,
    pub pid: u16,
    pub product: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZmkBleDevice {
    pub device_id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZmkTransport {
    SerialPort(String),
    BleDevice(String),
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

pub fn scan_ble_devices() -> Result<Vec<ZmkBleDevice>, Box<dyn Error>> {
    if !bluetooth_scan_available() {
        return Ok(Vec::new());
    }

    let devices =
        StudioClient::<PlatformBleTransport>::list_ble_devices_with_mode(BleDiscoveryMode::Any)?;
    Ok(devices
        .into_iter()
        .map(|device| {
            let device_id = device.device_id;
            // Use local_name when available so matching against HID product strings
            // is stable across platforms (display_name may include backend-specific IDs).
            let display_name = device
                .local_name
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| device_id.clone());
            ZmkBleDevice {
                device_id,
                display_name,
            }
        })
        .collect())
}

#[cfg(target_os = "windows")]
fn bluetooth_scan_available() -> bool {
    windows_bluetooth_radio_is_on().unwrap_or(true)
}

#[cfg(not(target_os = "windows"))]
fn bluetooth_scan_available() -> bool {
    true
}

#[cfg(target_os = "windows")]
fn windows_bluetooth_radio_is_on() -> windows::core::Result<bool> {
    use windows::Devices::Bluetooth::BluetoothAdapter;
    use windows::Devices::Radios::RadioState;

    let Some(adapter) = BluetoothAdapter::GetDefaultAsync()?.join().ok() else {
        return Ok(false);
    };
    let radio = adapter.GetRadioAsync()?.join()?;
    Ok(radio.State()? == RadioState::On)
}

pub struct ZmkData {
    pub physical_layouts: keymap::PhysicalLayouts,
    pub layout_keys: Vec<Vec<Option<LayoutKey>>>,
    pub layer_count: usize,
}

pub fn fetch_zmk_data(transport: &ZmkTransport) -> Result<ZmkData, Box<dyn Error>> {
    match transport {
        ZmkTransport::SerialPort(port_name) => {
            let client = StudioClient::open_serial(port_name)
                .map_err(|e| format!("Failed to open serial port '{}': {}", port_name, e))?;
            fetch_zmk_data_from_client(client)
        }
        ZmkTransport::BleDevice(device_id) => open_zmk_ble_and_fetch(device_id),
    }
}

fn open_zmk_ble_and_fetch(device_id: &str) -> Result<ZmkData, Box<dyn Error>> {
    let client = StudioClient::<PlatformBleTransport>::open_ble(device_id)
        .map_err(|e| format!("Failed to connect to BLE device '{device_id}': {e}"))?;

    fetch_zmk_data_from_client(client)
}

fn fetch_zmk_data_from_client<T: Read + Write>(
    mut client: StudioClient<T>,
) -> Result<ZmkData, Box<dyn Error>> {
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

    // Drop the ZMK RPC connection and give transport time to settle before
    // the caller opens any other handle (e.g. HID).
    drop(client);
    std::thread::sleep(Duration::from_millis(100));

    Ok(ZmkData {
        physical_layouts,
        layout_keys,
        layer_count,
    })
}
