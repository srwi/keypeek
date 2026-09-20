pub mod application;
pub mod domain;
pub mod firmware;
pub mod hid_labels;
pub mod os_layout;
pub mod platform;
pub mod presentation;
pub mod protocols;

#[cfg(test)]
pub mod test_utils;

pub use application::{connection, device_discovery, session, ui_wake};
pub use domain::{key_matrix, key_spec, layout, visibility};
pub use presentation::{
    key_paint, key_presenter, keymap_editor, layout_key, overlay_host, overlay_window, settings,
    ui_widgets,
};

#[cfg(target_arch = "wasm32")]
pub use platform::web::*;
