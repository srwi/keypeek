//! Presentation layer: windows, key painters, and keymap editor.

pub mod key_paint;
pub mod key_presenter;
pub mod keymap_editor;
pub mod layout_key;
pub mod overlay_host;
pub mod overlay_window;
pub mod settings;
pub mod ui_widgets;

pub use key_paint::{KeyColors, KeyDisplay, KeyPaintStyle};
pub use key_presenter::{KeyPresenter, StandardKeyPresenter};
pub use keymap_editor::{EditorProfile, EditorState};
pub use layout_key::{Label, LayoutKey};
pub use overlay_host::OverlayHost;
pub use overlay_window::{draw_overlay_keys, OverlayView};
#[cfg(not(target_arch = "wasm32"))]
pub use overlay_window::{DesktopApp, DesktopOverlayApp, OverlayApp};
#[cfg(target_arch = "wasm32")]
pub use overlay_window::{WebApp, WebOverlayApp, OverlayApp};
pub use settings::{FileSettingsStore, MemorySettingsStore, Settings, SettingsStore};
