#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

pub mod application;
pub mod domain;
mod firmware;
mod hid_labels;
mod key_paint;
mod key_presenter;
mod keyboard;
mod keymap_editor;
mod layout_key;
mod os_layout;
mod overlay_window;
mod platform;
mod protocols;
mod settings;
mod tray;
mod ui_wake;
mod ui_widgets;

#[allow(unused_imports)]
pub use application::{connection, device_discovery, session};
#[allow(unused_imports)]
pub use domain::{key_matrix, key_spec, layout, visibility};

use application::device_discovery::discover_devices;
use settings::Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    os_layout::init();
    let settings = Settings::load().unwrap_or_default();
    let available_devices = discover_devices();
    platform::run(settings, available_devices)
}
