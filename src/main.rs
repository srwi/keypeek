#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod connection;
mod device_discovery;
mod hid_labels;
mod key_matrix;
mod key_paint;
mod key_presenter;
mod key_spec;
mod keyboard;
mod keymap_editor;
mod layout_key;
mod os_layout;
mod overlay_window;
mod platform;
mod protocols;
mod session;
mod settings;
mod tray;
mod ui_wake;
mod ui_widgets;
mod visibility;

use device_discovery::discover_devices;
use settings::Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    os_layout::init();
    let settings = Settings::load().unwrap_or_default();
    let available_devices = discover_devices();
    platform::run(settings, available_devices)
}
