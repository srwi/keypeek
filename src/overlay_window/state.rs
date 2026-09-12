use crate::settings::Settings;
use egui_file_dialog::FileDialog;

pub struct UiState {
    pub settings_visible: bool,
    pub settings_error: Option<String>,
    pub settings_warning: Option<String>,
    pub mouse_passthrough: Option<bool>,
    pub file_dialog: FileDialog,
}

pub struct SettingsState {
    pub active: Settings,
    pub draft: Settings,
}
