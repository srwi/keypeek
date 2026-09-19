#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::sync::Arc;

use keypeek::application::device_discovery::discover_devices;
use keypeek::presentation::settings::FileSettingsStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    keypeek::os_layout::init();
    let settings_store = Arc::new(FileSettingsStore::default());
    let available_devices = discover_devices(keypeek::platform::scan_all_hid());
    keypeek::platform::run(settings_store, available_devices)
}
