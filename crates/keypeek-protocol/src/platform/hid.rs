//! Platform-level USB HID enumeration and transport using native `hidapi`.

use crate::application::device_discovery::HidDeviceInfo;
use crate::protocols::{DeviceError, RawHidTransport};
use hidapi::{HidApi, HidDevice};
use std::time::Duration;
use web_time::Instant;

/// Scans all currently enumerated USB HID interfaces on the system using `hidapi`.
pub fn scan_all_hid() -> Vec<HidDeviceInfo> {
    let Ok(api) = HidApi::new() else {
        return Vec::new();
    };
    api.device_list()
        .map(|d| HidDeviceInfo {
            vendor_id: d.vendor_id(),
            product_id: d.product_id(),
            usage_page: d.usage_page(),
            product: d.product_string().map(|s| s.to_string()),
            serial_number: d.serial_number().map(|s| s.to_string()),
        })
        .collect()
}

/// Native desktop USB/BLE HID transport wrapping [`hidapi::HidDevice`].
pub struct NativeHidTransport {
    device: HidDevice,
}

impl NativeHidTransport {
    pub fn new(device: HidDevice) -> Self {
        Self { device }
    }
}

impl RawHidTransport for NativeHidTransport {
    fn write_output_report(&mut self, data: &[u8]) -> Result<(), DeviceError> {
        // Keyboard companion modules and raw HID interfaces expect 32-byte buffers.
        // On Windows/USB, hidapi expects a leading Report ID (0x00 for unnumbered reports).
        if !data.is_empty() && data.len() <= 32 {
            let mut padded = [0u8; 33];
            padded[1..1 + data.len()].copy_from_slice(data);
            self.device
                .write(&padded)
                .map_err(|e| DeviceError::Transport(e.to_string()))?;
        } else {
            self.device
                .write(data)
                .map_err(|e| DeviceError::Transport(e.to_string()))?;
        }
        Ok(())
    }

    fn read_input_report(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, DeviceError> {
        let mut buffer = [0u8; 64];
        let timeout_ms = timeout.as_millis() as i32;
        match self.device.read_timeout(&mut buffer, timeout_ms) {
            Ok(read) if read > 0 => Ok(Some(buffer[..read].to_vec())),
            Ok(_) => Ok(None),
            Err(e) => Err(DeviceError::Transport(e.to_string())),
        }
    }
}

/// Opens a raw HID interface matching the specified VID, PID, and usage page.
pub fn open_hid_transport(
    vid: u16,
    pid: u16,
    usage_page: u16,
) -> Result<Box<dyn RawHidTransport>, DeviceError> {
    let api =
        HidApi::new().map_err(|e| DeviceError::Transport(format!("hidapi init failed: {e}")))?;
    let path = api
        .device_list()
        .find(|d| d.vendor_id() == vid && d.product_id() == pid && d.usage_page() == usage_page)
        .map(|d| d.path().to_owned())
        .ok_or_else(|| {
            DeviceError::Transport(format!(
                "could not find HID interface for {vid:04x}:{pid:04x} usage 0x{usage_page:04x}"
            ))
        })?;

    let device = api
        .open_path(&path)
        .map_err(|e| DeviceError::Transport(e.to_string()))?;

    Ok(Box::new(NativeHidTransport::new(device)))
}

/// Waits for an HID interface matching the specified VID/PID and usage page to appear.
pub fn wait_for_hid_reappearance(
    vid: u16,
    pid: u16,
    usage_page: u16,
    timeout: Duration,
) -> Result<(), String> {
    // On Linux BLE, the HID node can temporarily disappear while HoG/GATT activity settles; wait
    // for the matching HID interface to reappear before reconnecting via hidapi.
    let deadline = Instant::now() + timeout;
    let mut device_present_without_usage = false;
    while Instant::now() < deadline {
        let api = HidApi::new().map_err(|e| format!("hidapi init failed: {e}"))?;
        let mut matched = false;
        for d in api.device_list() {
            if d.vendor_id() == vid && d.product_id() == pid {
                if d.usage_page() == usage_page {
                    matched = true;
                    break;
                }
                device_present_without_usage = true;
            }
        }
        if matched {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    if device_present_without_usage {
        return Err("Please re-pair the keyboard to refresh the HID descriptor.".to_string());
    }

    Err(format!(
        "HID interface did not reappear in {} ms for {:04x}:{:04x} usage 0x{:04x}",
        timeout.as_millis(),
        vid,
        pid,
        usage_page
    ))
}

/// Waits for an HID interface to appear within `timeout` and opens it.
pub fn wait_and_open_hid_transport(
    vid: u16,
    pid: u16,
    usage_page: u16,
    timeout: Duration,
) -> Result<Box<dyn RawHidTransport>, DeviceError> {
    wait_for_hid_reappearance(vid, pid, usage_page, timeout).map_err(DeviceError::Transport)?;
    open_hid_transport(vid, pid, usage_page)
}
