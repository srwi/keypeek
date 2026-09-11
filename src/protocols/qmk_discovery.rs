//! Device discovery scanner for QMK (VIA) and Vial keyboards.

use std::collections::HashSet;

use crate::device_discovery::{
    DeviceDriverScanner, DeviceKind, DiscoveredDevice, DiscoveryContext, HidDeviceInfo,
};

/// Standard VIA RAW HID usage page (0xff60).
pub const VIA_USAGE_PAGE: u16 = 0xff60;

/// Scanner for QMK (VIA) and Vial keyboards over USB HID.
pub struct QmkScanner;

impl DeviceDriverScanner for QmkScanner {
    fn scan(&self, ctx: &mut DiscoveryContext) -> Vec<DiscoveredDevice> {
        let mut devices = Vec::new();
        let mut seen_via: HashSet<(u16, u16)> = HashSet::new();

        for dev in ctx.hid_devices() {
            if dev.usage_page != VIA_USAGE_PAGE {
                continue;
            }
            if !seen_via.insert((dev.vendor_id, dev.product_id)) {
                continue;
            }
            if ctx.is_claimed(dev.vendor_id, dev.product_id) {
                continue;
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

        for dev in &devices {
            ctx.claim(dev.vid, dev.pid);
        }

        devices
    }
}

pub fn is_vial_device(dev: &HidDeviceInfo) -> bool {
    dev.serial_number
        .as_deref()
        .is_some_and(|s| s.to_ascii_lowercase().starts_with("vial:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vial_device() {
        let vial = HidDeviceInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            product: Some("Vial Board".to_string()),
            serial_number: Some("vial:12345678".to_string()),
        };
        assert!(is_vial_device(&vial));

        let qmk = HidDeviceInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            product: Some("QMK Board".to_string()),
            serial_number: Some("12345678".to_string()),
        };
        assert!(!is_vial_device(&qmk));
    }

    #[test]
    fn test_qmk_scanner_filters_non_via_and_claimed() {
        let mut ctx = DiscoveryContext::new(vec![
            HidDeviceInfo {
                vendor_id: 0x1111,
                product_id: 0x2222,
                usage_page: 0x0001,
                product: Some("Keyboard Generic".to_string()),
                serial_number: None,
            },
            HidDeviceInfo {
                vendor_id: 0x3333,
                product_id: 0x4444,
                usage_page: VIA_USAGE_PAGE,
                product: Some("My QMK Board".to_string()),
                serial_number: None,
            },
            HidDeviceInfo {
                vendor_id: 0x5555,
                product_id: 0x6666,
                usage_page: VIA_USAGE_PAGE,
                product: Some("Claimed ZMK Board".to_string()),
                serial_number: None,
            },
        ]);

        ctx.claim(0x5555, 0x6666);

        let scanner = QmkScanner;
        let devices = scanner.scan(&mut ctx);

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].vid, 0x3333);
        assert_eq!(devices[0].pid, 0x4444);
        assert_eq!(devices[0].kind, DeviceKind::Qmk);
    }
}
