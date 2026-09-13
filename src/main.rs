#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

pub mod application;
pub mod domain;
pub mod presentation;
mod firmware;
mod hid_labels;
mod os_layout;
mod platform;
mod protocols;

pub mod keyboard {
    pub use crate::application::keyboard::Keyboard;
    pub use crate::domain::keyboard::KeyboardDomain;
    pub use crate::domain::visibility::OverlayConfig;
}

pub use application::{connection, device_discovery, session};
pub use domain::{key_matrix, key_spec, layout, visibility};
pub use presentation::{
    key_paint, key_presenter, keymap_editor, layout_key, overlay_window, settings, tray, ui_wake,
    ui_widgets,
};

use application::device_discovery::discover_devices;
use settings::Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    os_layout::init();
    let settings = Settings::load().unwrap_or_default();
    let available_devices = discover_devices();
    platform::run(settings, available_devices)
}
