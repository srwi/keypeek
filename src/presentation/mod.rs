//! Presentation layer: windows, key painters, and keymap editor.

pub mod key_paint;
pub mod key_presenter;
pub mod keymap_editor;
pub mod layout_key;
pub mod overlay_window;
pub mod settings;
pub mod ui_widgets;

pub use key_presenter::{KeyPresenter, StandardKeyPresenter};
pub use layout_key::{Label, LayoutKey};
pub use overlay_window::OverlayApp;
pub use settings::{FileSettingsStore, MemorySettingsStore, Settings, SettingsStore};
