use crate::protocols::zmk_rpc;
use std::collections::HashSet;

const VIA_USAGE_PAGE: u16 = 0xff60;

/// Minimal HID device info used only for device discovery.
/// Separate from `qmk_via_api::KeyboardDeviceInfo` which is VIA-specific.
struct HidInfo {
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    product: Option<String>,
    serial_number: Option<String>,
}

/// Scan every HID device on the system.  This is the integration-layer scan
/// used only for device discovery; it is distinct from qmk_via_api's
/// VIA-specific scan which is used for actual keyboard communication.
fn scan_all_hid() -> Vec<HidInfo> {
    let Ok(api) = hidapi::HidApi::new() else {
        return Vec::new();
    };
    api.device_list()
        .map(|d| HidInfo {
            vendor_id: d.vendor_id(),
            product_id: d.product_id(),
            usage_page: d.usage_page(),
            product: d.product_string().map(|s| s.to_string()),
            serial_number: d.serial_number().map(|s| s.to_string()),
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    Zmk,
    Vial,
    Qmk,
}

impl DeviceKind {
    pub fn label(self) -> &'static str {
        match self {
            DeviceKind::Zmk => "ZMK",
            DeviceKind::Vial => "Vial",
            DeviceKind::Qmk => "QMK",
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiscoveredDevice {
    pub base_name: String,
    pub vid: u16,
    pub pid: u16,
    pub serial_port: Option<String>,
    pub ble_device_id: Option<String>,
    pub kind: DeviceKind,
}

impl DiscoveredDevice {
    pub fn display_name(&self) -> String {
        let kind_label = match self.kind {
            DeviceKind::Zmk => match (&self.serial_port, &self.ble_device_id) {
                (_, Some(_)) => "ZMK BLE",
                (Some(_), None) => "ZMK Serial",
                (None, None) => "ZMK",
            },
            _ => self.kind.label(),
        };
        format!(
            "{} ({}, {:04X}:{:04X})",
            self.base_name, kind_label, self.vid, self.pid
        )
    }
}

pub fn discover_devices() -> Vec<DiscoveredDevice> {
    // Scan every HID device on the system, regardless of usage page.
    let all_hid: Vec<HidInfo> = scan_all_hid();

    let mut devices: Vec<DiscoveredDevice> = Vec::new();
    let mut zmk_vid_pid: HashSet<(u16, u16)> = HashSet::new();

    // ── VIA/Vial/QMK (USB HID, usage page 0xFF60) ──────────────────────────
    // Deduplicate by VID+PID — the same keyboard exposes multiple HID
    // interfaces (keyboard, consumer, etc.) and all share the same VID/PID.
    {
        let mut seen_via: HashSet<(u16, u16)> = HashSet::new();
        for dev in &all_hid {
            if dev.usage_page != VIA_USAGE_PAGE {
                continue;
            }
            if !seen_via.insert((dev.vendor_id, dev.product_id)) {
                continue; // duplicate interface for same device
            }
            let base_name = dev
                .product
                .clone()
                .unwrap_or_else(|| format!("{:04X}:{:04X}", dev.vendor_id, dev.product_id));
            let kind = if is_vial_device(dev) {
                DeviceKind::Vial
            } else {
                DeviceKind::Qmk
            };
            devices.push(DiscoveredDevice {
                base_name,
                vid: dev.vendor_id,
                pid: dev.product_id,
                serial_port: None,
                ble_device_id: None,
                kind,
            });
        }
    }

    // ── ZMK over serial ─────────────────────────────────────────────────────
    for sp in zmk_rpc::scan_serial_ports() {
        // Prefer the product name from HID if the keyboard is also visible there.
        let base_name = all_hid
            .iter()
            .find(|d| d.vendor_id == sp.vid && d.product_id == sp.pid)
            .and_then(|d| d.product.clone())
            .or(sp.product)
            .unwrap_or_else(|| format!("{:04X}:{:04X}", sp.vid, sp.pid));
        devices.push(DiscoveredDevice {
            base_name: format!("{} [{}]", base_name, sp.port_name),
            vid: sp.vid,
            pid: sp.pid,
            serial_port: Some(sp.port_name),
            ble_device_id: None,
            kind: DeviceKind::Zmk,
        });
        zmk_vid_pid.insert((sp.vid, sp.pid));
    }

    // ── ZMK over BLE ─────────────────────────────────────────────────────────
    // Query the platform's BLE stack for devices exposing the ZMK Studio
    // service.  On Windows, bluest queries the WinRT device store so paired
    // keyboards are found even when not advertising; on Linux/macOS this
    // performs an advertisement scan.  We then match device names against
    // HID product strings to retrieve VID/PID for the combined entry.
    if let Ok(ble_devices) = zmk_rpc::scan_ble_devices() {
        for ble in ble_devices {
            // Match by name against any non-Vial HID entry (not just VIA, since
            // ZMK keyboards don't expose VIA).
            if let Some(hid) = all_hid.iter().find(|d| {
                d.usage_page != VIA_USAGE_PAGE && is_possible_ble_match(d, &ble.display_name)
            }) {
                devices.push(DiscoveredDevice {
                    base_name: hid
                        .product
                        .clone()
                        .unwrap_or_else(|| ble.display_name.clone()),
                    vid: hid.vendor_id,
                    pid: hid.product_id,
                    serial_port: None,
                    ble_device_id: Some(ble.device_id),
                    kind: DeviceKind::Zmk,
                });
                zmk_vid_pid.insert((hid.vendor_id, hid.product_id));
            }
        }
    }

    // Drop any raw QMK entry whose VID+PID is covered by a ZMK transport.
    devices.retain(|d| match d.kind {
        DeviceKind::Qmk => !zmk_vid_pid.contains(&(d.vid, d.pid)),
        _ => true,
    });

    devices.sort_by(|a, b| a.display_name().cmp(&b.display_name()));
    devices.dedup_by(|a, b| {
        a.vid == b.vid
            && a.pid == b.pid
            && a.kind == b.kind
            && a.serial_port == b.serial_port
            && a.ble_device_id == b.ble_device_id
    });

    devices
}

/// Fuzzy name match between a HID product name and a BLE device name to
/// correlate a ZMK BLE device with its HID entry.
fn is_possible_ble_match(hid: &HidInfo, ble_name: &str) -> bool {
    let hid_name = hid
        .product
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let ble_name = ble_name.to_ascii_lowercase();
    !hid_name.is_empty() && (hid_name.contains(&ble_name) || ble_name.contains(&hid_name))
}

fn is_vial_device(dev: &HidInfo) -> bool {
    dev.serial_number
        .as_deref()
        .is_some_and(|s| s.to_ascii_lowercase().starts_with("vial:"))
}

#[cfg(test)]
mod tests {
    use super::{DeviceKind, DiscoveredDevice};

    #[test]
    fn display_name_uses_kind_label() {
        let board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0x1234,
            pid: 0xABCD,
            serial_port: None,
            ble_device_id: None,
            kind: DeviceKind::Zmk,
        };
        assert_eq!(board.display_name(), "Board (ZMK, 1234:ABCD)");
    }

    #[test]
    fn kind_labels_match_expected_ui_text() {
        assert_eq!(DeviceKind::Zmk.label(), "ZMK");
        assert_eq!(DeviceKind::Vial.label(), "Vial");
        assert_eq!(DeviceKind::Qmk.label(), "QMK");
    }

    #[test]
    fn display_name_for_other_kinds() {
        let vial_board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0,
            pid: 0,
            serial_port: None,
            ble_device_id: None,
            kind: DeviceKind::Vial,
        };
        let qmk_board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0x0A0B,
            pid: 0x0C0D,
            serial_port: None,
            ble_device_id: None,
            kind: DeviceKind::Qmk,
        };
        assert_eq!(vial_board.display_name(), "Board (Vial, 0000:0000)");
        assert_eq!(qmk_board.display_name(), "Board (QMK, 0A0B:0C0D)");
    }

    #[test]
    fn zmk_transport_label_variants() {
        let serial = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 1,
            pid: 2,
            serial_port: Some("COM3".to_string()),
            ble_device_id: None,
            kind: DeviceKind::Zmk,
        };
        let ble = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 1,
            pid: 2,
            serial_port: None,
            ble_device_id: Some("id".to_string()),
            kind: DeviceKind::Zmk,
        };
        assert!(serial.display_name().contains("ZMK Serial"));
        assert!(ble.display_name().contains("ZMK BLE"));
    }
}
