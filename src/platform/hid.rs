//! Platform-level USB HID enumeration using native `hidapi`.

use crate::application::device_discovery::HidDeviceInfo;

/// Scans all currently enumerated USB HID interfaces on the system using `hidapi`.
pub fn scan_all_hid() -> Vec<HidDeviceInfo> {
    let Ok(api) = hidapi::HidApi::new() else {
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
