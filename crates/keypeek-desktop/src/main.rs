#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::sync::Arc;

use keypeek_presentation::FileSettingsStore;
use keypeek_protocol::{discover_devices, os_layout, scan_all_hid};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    os_layout::init();
    let settings_store = Arc::new(FileSettingsStore::default());
    let available_devices = discover_devices(scan_all_hid());
    keypeek_desktop::run(settings_store, available_devices)
}
