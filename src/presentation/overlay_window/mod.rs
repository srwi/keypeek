//! Application window coordinators and presentation views.

#[cfg(not(target_arch = "wasm32"))]
mod desktop;
#[cfg(not(target_arch = "wasm32"))]
mod ui_settings;

#[cfg(target_arch = "wasm32")]
mod web;

mod state;
mod ui_overlay;

pub use state::{SettingsState, UiState};
pub use ui_overlay::{draw_overlay_keys, OverlayView};

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::{DesktopApp, DesktopOverlayApp, OverlayApp};

#[cfg(target_arch = "wasm32")]
pub use web::{WebApp, WebOverlayApp, OverlayApp};
