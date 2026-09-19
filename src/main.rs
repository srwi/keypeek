#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

pub mod application;
pub mod domain;
mod firmware;
mod hid_labels;
mod os_layout;
mod platform;
pub mod presentation;
mod protocols;

pub use application::{connection, device_discovery, session, ui_wake};
pub use domain::{key_matrix, key_spec, layout, visibility};
pub use presentation::{
    key_paint, key_presenter, keymap_editor, layout_key, overlay_window, settings, ui_widgets,
};

use std::sync::Arc;

use application::device_discovery::discover_devices;
use settings::FileSettingsStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    os_layout::init();
    let settings_store = Arc::new(FileSettingsStore::default());
    let available_devices = discover_devices();
    platform::run(settings_store, available_devices)
}
