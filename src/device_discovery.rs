use crate::protocols::zmk_rpc;
use std::collections::HashSet;

const VIA_USAGE_PAGE: u16 = 0xff60;

/// Identifiers for the virtual keyboard. They must match `resources/mock_keyboard.json`
/// and are deliberately outside the ranges real boards use.
const MOCK_VID: u16 = 0xF00D;
const MOCK_PID: u16 = 0xF00D;

struct HidInfo {
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    manufacturer: Option<String>,
    product: Option<String>,
    serial_number: Option<String>,
}

fn scan_all_hid() -> Vec<HidInfo> {
    let Ok(api) = hidapi::HidApi::new() else {
        return Vec::new();
    };
    api.device_list()
        .map(|d| HidInfo {
            vendor_id: d.vendor_id(),
            product_id: d.product_id(),
            usage_page: d.usage_page(),
            manufacturer: d.manufacturer_string().map(|s| s.to_string()),
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
    Mock,
}

impl DeviceKind {
    pub fn label(self) -> &'static str {
        match self {
            DeviceKind::Zmk => "ZMK",
            DeviceKind::Vial => "Vial",
            DeviceKind::Qmk => "QMK",
            DeviceKind::Mock => "Mock",
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
        let protocol = match self.kind {
            DeviceKind::Qmk | DeviceKind::Vial | DeviceKind::Mock => self.kind.label(),
            DeviceKind::Zmk => match (&self.serial_port, &self.ble_device_id) {
                (Some(_), None) => "ZMK Serial",
                _ => "ZMK BLE",
            },
        };

        format!(
            "{} ({}, {:04X}:{:04X})",
            self.base_name, protocol, self.vid, self.pid
        )
    }
}

pub fn discover_devices() -> Vec<DiscoveredDevice> {
    let all_hid: Vec<HidInfo> = scan_all_hid();

    let mut devices: Vec<DiscoveredDevice> = Vec::new();
    let mut zmk_vid_pid: HashSet<(u16, u16)> = HashSet::new();
    let mut non_zmk_via_vid_pid: HashSet<(u16, u16)> = HashSet::new();

    {
        let mut seen_via: HashSet<(u16, u16)> = HashSet::new();
        for dev in &all_hid {
            if dev.usage_page != VIA_USAGE_PAGE {
                continue;
            }
            if !seen_via.insert((dev.vendor_id, dev.product_id)) {
                continue; // Duplicate interface for same device
            }
            let base_name = dev
                .product
                .clone()
                .unwrap_or_else(|| format!("{:04X}:{:04X}", dev.vendor_id, dev.product_id));
            let kind = if is_vial_device(dev) {
                DeviceKind::Vial
            } else if is_probable_zmk_hid(dev) {
                DeviceKind::Zmk
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
            if kind == DeviceKind::Zmk {
                zmk_vid_pid.insert((dev.vendor_id, dev.product_id));
            } else {
                non_zmk_via_vid_pid.insert((dev.vendor_id, dev.product_id));
            }
        }
    }

    for sp in zmk_rpc::scan_serial_ports() {
        if non_zmk_via_vid_pid.contains(&(sp.vid, sp.pid)) {
            continue;
        }

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

    if let Ok(ble_devices) = zmk_rpc::scan_ble_devices() {
        for ble in ble_devices {
            if let Some(hid) = find_matching_hid_for_ble(&all_hid, &ble.display_name) {
                if non_zmk_via_vid_pid.contains(&(hid.vendor_id, hid.product_id)) {
                    continue;
                }

                // If a serial transport exists for the same board, prefer serial and hide BLE.
                // This avoids platform-specific BLE RPC instability when USB and BLE are both active.
                if zmk_vid_pid.contains(&(hid.vendor_id, hid.product_id)) {
                    let has_serial = devices.iter().any(|d| {
                        d.kind == DeviceKind::Zmk
                            && d.vid == hid.vendor_id
                            && d.pid == hid.product_id
                            && d.serial_port.is_some()
                    });
                    if !has_serial {
                        if let Some(existing) = devices.iter_mut().find(|d| {
                            d.kind == DeviceKind::Zmk
                                && d.vid == hid.vendor_id
                                && d.pid == hid.product_id
                                && d.serial_port.is_none()
                        }) {
                            existing.ble_device_id = Some(ble.device_id.clone());
                        }
                    }
                    continue;
                }

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

    // Drop any non-ZMK entry whose VID+PID is covered by a ZMK transport.
    devices.retain(|d| d.kind == DeviceKind::Zmk || !zmk_vid_pid.contains(&(d.vid, d.pid)));

    // Drop ZMK entries that have no connectable transport (neither BLE nor serial).
    // This can happen when a ZMK device is detected via HID but BLE discovery failed
    // (e.g. Bluetooth adapter off, permissions denied).
    devices.retain(|d| {
        d.kind != DeviceKind::Zmk || d.ble_device_id.is_some() || d.serial_port.is_some()
    });

    devices.sort_by_cached_key(|d| d.display_name());
    devices.dedup_by(|a, b| {
        a.vid == b.vid
            && a.pid == b.pid
            && a.kind == b.kind
            && a.serial_port == b.serial_port
            && a.ble_device_id == b.ble_device_id
    });

    if cfg!(debug_assertions) {
        devices.push(mock_device());
    }

    devices
}

fn mock_device() -> DiscoveredDevice {
    DiscoveredDevice {
        base_name: "Virtual Keyboard".to_string(),
        vid: MOCK_VID,
        pid: MOCK_PID,
        serial_port: None,
        ble_device_id: None,
        kind: DeviceKind::Mock,
    }
}

fn is_possible_ble_match(hid: &HidInfo, ble_name: &str) -> bool {
    let hid_name = hid
        .product
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let ble_name = ble_name.to_ascii_lowercase();
    if hid_name.is_empty() || ble_name.is_empty() {
        return false;
    }

    if hid_name.contains(&ble_name) || ble_name.contains(&hid_name) {
        return true;
    }

    let hid_norm = normalize_name_for_match(&hid_name);
    let ble_norm = normalize_name_for_match(&ble_name);
    !hid_norm.is_empty()
        && !ble_norm.is_empty()
        && (hid_norm.contains(&ble_norm) || ble_norm.contains(&hid_norm))
}

fn find_matching_hid_for_ble<'a>(all_hid: &'a [HidInfo], ble_name: &str) -> Option<&'a HidInfo> {
    // Prefer non-VIA HID interfaces when available, but fall back to VIA interfaces.
    // On macOS, BLE keyboards can be exposed only through a VIA usage-page interface.
    all_hid
        .iter()
        .find(|d| d.usage_page != VIA_USAGE_PAGE && is_possible_ble_match(d, ble_name))
        .or_else(|| {
            all_hid
                .iter()
                .find(|d| d.usage_page == VIA_USAGE_PAGE && is_possible_ble_match(d, ble_name))
        })
}

fn normalize_name_for_match(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

fn is_vial_device(dev: &HidInfo) -> bool {
    dev.serial_number
        .as_deref()
        .is_some_and(|s| s.to_ascii_lowercase().starts_with("vial:"))
}

fn is_probable_zmk_hid(dev: &HidInfo) -> bool {
    dev.manufacturer
        .as_deref()
        .is_some_and(|m| m.to_ascii_lowercase().contains("zmk"))
        || dev
            .product
            .as_deref()
            .is_some_and(|p| p.to_ascii_lowercase().contains("zmk"))
}

#[cfg(test)]
mod tests {
    use super::{
        find_matching_hid_for_ble, is_possible_ble_match, is_probable_zmk_hid, mock_device,
        DeviceKind, DiscoveredDevice, HidInfo, VIA_USAGE_PAGE,
    };

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
        assert_eq!(board.display_name(), "Board (ZMK BLE, 1234:ABCD)");
    }

    #[test]
    fn kind_labels_match_expected_ui_text() {
        assert_eq!(DeviceKind::Zmk.label(), "ZMK");
        assert_eq!(DeviceKind::Vial.label(), "Vial");
        assert_eq!(DeviceKind::Qmk.label(), "QMK");
        assert_eq!(DeviceKind::Mock.label(), "Mock");
    }

    #[test]
    fn mock_device_name_follows_the_same_shape_as_real_devices() {
        assert_eq!(
            mock_device().display_name(),
            "Virtual Keyboard (Mock, F00D:F00D)"
        );
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

    #[test]
    fn ble_match_prefers_non_via_interface() {
        let via_hid = HidInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            manufacturer: None,
            product: Some("Corne".to_string()),
            serial_number: None,
        };
        let non_via_hid = HidInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: 0x0001,
            manufacturer: None,
            product: Some("Corne".to_string()),
            serial_number: None,
        };

        let hid = [via_hid, non_via_hid];
        let match_hid = find_matching_hid_for_ble(&hid, "Corne");
        assert_eq!(match_hid.map(|h| h.usage_page), Some(0x0001));
    }

    #[test]
    fn ble_match_falls_back_to_via_interface() {
        let via_hid = HidInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            manufacturer: None,
            product: Some("Corne".to_string()),
            serial_number: None,
        };

        let hid = [via_hid];
        let match_hid = find_matching_hid_for_ble(&hid, "Corne");
        assert_eq!(match_hid.map(|h| h.usage_page), Some(VIA_USAGE_PAGE));
    }

    #[test]
    fn ble_match_handles_backend_decorated_name() {
        let hid = HidInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            manufacturer: None,
            product: Some("Corne".to_string()),
            serial_number: None,
        };

        assert!(is_possible_ble_match(&hid, "Corne [{\"uuid\":\"abc\"}]"));
    }

    #[test]
    fn probable_zmk_hid_detects_zmk_project_manufacturer() {
        let hid = HidInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            manufacturer: Some("ZMK Project".to_string()),
            product: Some("Iskra Numpad".to_string()),
            serial_number: None,
        };
        assert!(is_probable_zmk_hid(&hid));
    }
}
