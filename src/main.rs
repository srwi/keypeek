#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod connection;
mod device_discovery;
mod key_matrix;
mod keyboard;
mod layout_key;
mod overlay_window;
mod platform;
mod protocols;
mod qmk_keycode_labels;
mod settings;
mod tray;
mod ui_wake;
mod zmk_keycode_labels;

use device_discovery::discover_devices;
use settings::Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::load().unwrap_or_default();
    let available_devices = discover_devices();
    platform::run(settings, available_devices)
}
