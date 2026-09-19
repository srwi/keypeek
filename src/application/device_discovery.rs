//! Extensible device discovery for keyboards using protocol-provided scanners.
//!
//! Orchestrates discovery across registered [`DeviceDriverScanner`] implementations,
//! managing shared transport snapshots (e.g. USB HID devices) and conflict resolution.

use crate::protocols::ConnectionSpec;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub base_name: String,
    pub vid: u16,
    pub pid: u16,
    pub driver_id: &'static str,
    pub protocol_label: &'static str,
    pub requires_layout_file: bool,
    pub spec: ConnectionSpec,
}

impl DiscoveredDevice {
    pub fn display_name(&self) -> String {
        format!(
            "{} ({}, {:04X}:{:04X})",
            self.base_name, self.protocol_label, self.vid, self.pid
        )
    }
}

/// Generic, protocol-agnostic snapshot of an attached USB HID interface.
#[derive(Clone, Debug)]
pub struct HidDeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub usage_page: u16,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

/// Context shared across device driver scanners during discovery.
#[derive(Default)]
pub struct DiscoveryContext {
    hid_devices: Vec<HidDeviceInfo>,
    claimed_vid_pids: HashSet<(u16, u16)>,
}

impl DiscoveryContext {
    pub fn new(hid_devices: Vec<HidDeviceInfo>) -> Self {
        Self {
            hid_devices,
            claimed_vid_pids: HashSet::new(),
        }
    }

    pub fn hid_devices(&self) -> &[HidDeviceInfo] {
        &self.hid_devices
    }

    pub fn is_claimed(&self, vid: u16, pid: u16) -> bool {
        self.claimed_vid_pids.contains(&(vid, pid))
    }

    pub fn claim(&mut self, vid: u16, pid: u16) {
        self.claimed_vid_pids.insert((vid, pid));
    }

    /// Finds the product name of an HID device matching the given VID/PID, if present.
    pub fn find_product(&self, vid: u16, pid: u16) -> Option<String> {
        self.hid_devices
            .iter()
            .find(|d| d.vendor_id == vid && d.product_id == pid)
            .and_then(|d| d.product.clone())
    }
}

/// Port for firmware adapters to discover compatible hardware.
pub trait DeviceDriverScanner: Send + Sync {
    fn scan(&self, ctx: &mut DiscoveryContext) -> Vec<DiscoveredDevice>;
}

/// Returns the standard list of registered device scanners from all firmware bundles.
pub fn default_scanners() -> Vec<Box<dyn DeviceDriverScanner>> {
    crate::firmware::default_scanners()
}

/// Runs discovery over the specified scanners and context.
pub fn discover_devices_with(
    ctx: &mut DiscoveryContext,
    scanners: &[Box<dyn DeviceDriverScanner>],
) -> Vec<DiscoveredDevice> {
    let mut devices = Vec::new();
    for scanner in scanners {
        devices.extend(scanner.scan(ctx));
    }

    devices.sort_by_cached_key(|d| d.display_name());
    devices.dedup_by(|a, b| {
        a.vid == b.vid && a.pid == b.pid && a.driver_id == b.driver_id && a.spec == b.spec
    });

    devices
}

/// Discovers all available keyboards using provided HID interfaces and default protocol scanners.
pub fn discover_devices(hid_devices: Vec<HidDeviceInfo>) -> Vec<DiscoveredDevice> {
    let mut ctx = DiscoveryContext::new(hid_devices);
    let scanners = default_scanners();
    discover_devices_with(&mut ctx, &scanners)
}

/// Constructs a mock virtual keyboard descriptor.
#[cfg(test)]
pub fn mock_device() -> DiscoveredDevice {
    crate::firmware::mock::mock_device()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_uses_protocol_label() {
        let board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0x1234,
            pid: 0xABCD,
            driver_id: "zmk",
            protocol_label: "ZMK BLE",
            requires_layout_file: false,
            spec: ConnectionSpec::Mock,
        };
        assert_eq!(board.display_name(), "Board (ZMK BLE, 1234:ABCD)");
    }

    #[test]
    fn mock_device_name_follows_the_same_shape_as_real_devices() {
        assert_eq!(
            mock_device().display_name(),
            "Virtual Keyboard (Mock, F00D:F00D)"
        );
    }

    #[test]
    fn display_name_for_various_protocols() {
        let vial_board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0,
            pid: 0,
            driver_id: "vial",
            protocol_label: "Vial",
            requires_layout_file: false,
            spec: ConnectionSpec::Mock,
        };
        let qmk_board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0x0A0B,
            pid: 0x0C0D,
            driver_id: "via",
            protocol_label: "QMK",
            requires_layout_file: true,
            spec: ConnectionSpec::Mock,
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
            driver_id: "zmk",
            protocol_label: "ZMK Serial",
            requires_layout_file: false,
            spec: ConnectionSpec::Mock,
        };
        let ble = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 1,
            pid: 2,
            driver_id: "zmk",
            protocol_label: "ZMK BLE",
            requires_layout_file: false,
            spec: ConnectionSpec::Mock,
        };
        assert!(serial.display_name().contains("ZMK Serial"));
        assert!(ble.display_name().contains("ZMK BLE"));
    }

    struct TestScanner {
        devices: Vec<DiscoveredDevice>,
    }

    impl DeviceDriverScanner for TestScanner {
        fn scan(&self, ctx: &mut DiscoveryContext) -> Vec<DiscoveredDevice> {
            let mut result = Vec::new();
            for d in &self.devices {
                if !ctx.is_claimed(d.vid, d.pid) {
                    ctx.claim(d.vid, d.pid);
                    result.push(d.clone());
                }
            }
            result
        }
    }

    #[test]
    fn discover_devices_with_deduplicates_and_sorts() {
        let mut ctx = DiscoveryContext::default();
        let scanner1: Box<dyn DeviceDriverScanner> = Box::new(TestScanner {
            devices: vec![
                DiscoveredDevice {
                    base_name: "B Keyboard".to_string(),
                    vid: 0x0002,
                    pid: 0x0002,
                    driver_id: "via",
                    protocol_label: "QMK",
                    requires_layout_file: true,
                    spec: ConnectionSpec::Mock,
                },
                DiscoveredDevice {
                    base_name: "A Keyboard".to_string(),
                    vid: 0x0001,
                    pid: 0x0001,
                    driver_id: "via",
                    protocol_label: "QMK",
                    requires_layout_file: true,
                    spec: ConnectionSpec::Mock,
                },
            ],
        });

        let scanner2: Box<dyn DeviceDriverScanner> = Box::new(TestScanner {
            devices: vec![
                // Already claimed VID/PID 0x0001:0x0001 by scanner1, should be skipped
                DiscoveredDevice {
                    base_name: "A Keyboard Duplicate".to_string(),
                    vid: 0x0001,
                    pid: 0x0001,
                    driver_id: "vial",
                    protocol_label: "Vial",
                    requires_layout_file: false,
                    spec: ConnectionSpec::Mock,
                },
                DiscoveredDevice {
                    base_name: "C Keyboard".to_string(),
                    vid: 0x0003,
                    pid: 0x0003,
                    driver_id: "zmk",
                    protocol_label: "ZMK",
                    requires_layout_file: false,
                    spec: ConnectionSpec::Mock,
                },
            ],
        });

        let scanners = vec![scanner1, scanner2];
        let discovered = discover_devices_with(&mut ctx, &scanners);

        assert_eq!(discovered.len(), 3);
        assert_eq!(discovered[0].base_name, "A Keyboard");
        assert_eq!(discovered[1].base_name, "B Keyboard");
        assert_eq!(discovered[2].base_name, "C Keyboard");
    }
}
