//! Device discovery scanner for ZMK keyboards (Serial and BLE).

use super::qmk_discovery::VIA_USAGE_PAGE;
use super::zmk_rpc;
use crate::device_discovery::{
    DeviceDriverScanner, DeviceKind, DiscoveredDevice, DiscoveryContext, HidDeviceInfo,
};

/// Scanner for ZMK keyboards via Serial and Bluetooth LE.
pub struct ZmkScanner;

impl DeviceDriverScanner for ZmkScanner {
    fn scan(&self, ctx: &mut DiscoveryContext) -> Vec<DiscoveredDevice> {
        let mut devices = Vec::new();

        for sp in zmk_rpc::scan_serial_ports() {
            let base_name = ctx
                .find_product(sp.vid, sp.pid)
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
            ctx.claim(sp.vid, sp.pid);
        }

        if let Ok(ble_devices) = zmk_rpc::scan_ble_devices() {
            for ble in ble_devices {
                let matched = find_matching_hid_for_ble(ctx.hid_devices(), &ble.display_name)
                    .map(|hid| (hid.vendor_id, hid.product_id, hid.product.clone()));

                if let Some((vid, pid, product)) = matched {
                    if ctx.is_claimed(vid, pid) {
                        let has_serial = devices.iter().any(|d| {
                            d.kind == DeviceKind::Zmk
                                && d.vid == vid
                                && d.pid == pid
                                && d.serial_port.is_some()
                        });
                        if !has_serial {
                            if let Some(existing) = devices.iter_mut().find(|d| {
                                d.kind == DeviceKind::Zmk
                                    && d.vid == vid
                                    && d.pid == pid
                                    && d.serial_port.is_none()
                            }) {
                                existing.ble_device_id = Some(ble.device_id.clone());
                            }
                        }
                        continue;
                    }

                    devices.push(DiscoveredDevice {
                        base_name: product.unwrap_or_else(|| ble.display_name.clone()),
                        vid,
                        pid,
                        serial_port: None,
                        ble_device_id: Some(ble.device_id),
                        kind: DeviceKind::Zmk,
                    });
                    ctx.claim(vid, pid);
                }
            }
        }

        devices
    }
}

pub fn is_possible_ble_match(hid: &HidDeviceInfo, ble_name: &str) -> bool {
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

pub fn find_matching_hid_for_ble<'a>(
    all_hid: &'a [HidDeviceInfo],
    ble_name: &str,
) -> Option<&'a HidDeviceInfo> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ble_match_prefers_non_via_interface() {
        let via_hid = HidDeviceInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            product: Some("Corne".to_string()),
            serial_number: None,
        };
        let non_via_hid = HidDeviceInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: 0x0001,
            product: Some("Corne".to_string()),
            serial_number: None,
        };

        let hid = [via_hid, non_via_hid];
        let match_hid = find_matching_hid_for_ble(&hid, "Corne");
        assert_eq!(match_hid.map(|h| h.usage_page), Some(0x0001));
    }

    #[test]
    fn ble_match_falls_back_to_via_interface() {
        let via_hid = HidDeviceInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            product: Some("Corne".to_string()),
            serial_number: None,
        };

        let hid = [via_hid];
        let match_hid = find_matching_hid_for_ble(&hid, "Corne");
        assert_eq!(match_hid.map(|h| h.usage_page), Some(VIA_USAGE_PAGE));
    }

    #[test]
    fn ble_match_handles_backend_decorated_name() {
        let hid = HidDeviceInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            usage_page: VIA_USAGE_PAGE,
            product: Some("Corne".to_string()),
            serial_number: None,
        };

        assert!(is_possible_ble_match(&hid, "Corne [{\"uuid\":\"abc\"}]"));
    }
}
