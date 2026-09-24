pub mod key_paint;
pub mod keymap_editor;
pub mod overlay_host;
pub mod overlay_window;
pub mod settings;
pub mod ui_widgets;

pub use key_paint::*;
pub use overlay_host::*;
pub use overlay_window::*;
pub use settings::*;
pub use ui_widgets::*;

/// Registers Phosphor icons into egui font definitions.
pub fn add_phosphor_to_fonts(fonts: &mut egui::FontDefinitions) {
    fonts.font_data.insert(
        "phosphor".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(
            egui_phosphor::Variant::Regular.font_bytes(),
        )),
    );

    if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        let index = 1.min(font_keys.len());
        font_keys.insert(index, "phosphor".to_owned());
    }
}
