use crate::protocols::zmk_rpc;
use qmk_via_api::scan::{scan_keyboards, KeyboardDeviceInfo};
use std::collections::HashSet;

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
    let mut hid_devices = Vec::new();

    if let Ok(keyboards) = scan_keyboards() {
        for dev in keyboards {
            let base_name = dev
                .product
                .clone()
                .unwrap_or_else(|| format!("{:04X}:{:04X}", dev.vendor_id, dev.product_id));
            let kind = if is_vial_device(&dev) {
                DeviceKind::Vial
            } else {
                DeviceKind::Qmk
            };

            hid_devices.push(DiscoveredDevice {
                base_name,
                vid: dev.vendor_id,
                pid: dev.product_id,
                serial_port: None,
                ble_device_id: None,
                kind,
            });
        }
    }

    let mut devices = hid_devices.clone();
    let mut zmk_vid_pid = HashSet::new();

    for sp in zmk_rpc::scan_serial_ports() {
        let base_name = hid_devices
            .iter()
            .find(|d| d.vid == sp.vid && d.pid == sp.pid)
            .map(|d| d.base_name.clone())
            .unwrap_or_else(|| {
                sp.product
                    .unwrap_or_else(|| format!("{:04X}:{:04X}", sp.vid, sp.pid))
            });
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
            if let Some(hid) = hid_devices
                .iter()
                .find(|d| d.kind != DeviceKind::Vial && is_possible_ble_match(d, &ble.display_name))
            {
                devices.push(DiscoveredDevice {
                    base_name: hid.base_name.clone(),
                    vid: hid.vid,
                    pid: hid.pid,
                    serial_port: None,
                    ble_device_id: Some(ble.device_id),
                    kind: DeviceKind::Zmk,
                });
                zmk_vid_pid.insert((hid.vid, hid.pid));
            }
        }
    }

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

fn is_possible_ble_match(hid: &DiscoveredDevice, ble_name: &str) -> bool {
    let hid_name = hid.base_name.to_ascii_lowercase();
    let ble_name = ble_name.to_ascii_lowercase();
    hid_name.contains(&ble_name) || ble_name.contains(&hid_name)
}

fn is_vial_device(dev: &KeyboardDeviceInfo) -> bool {
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
