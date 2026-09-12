use crate::connection::ConnectionTask;
use crate::device_discovery::DiscoveredDevice;
use crate::keyboard::Keyboard;
use crate::protocols::{ConnectionSpec, Reopener};
use crate::settings::Settings;

use egui_file_dialog::FileDialog;
use std::sync::Arc;
use std::time::Instant;

pub enum AppConnectionState {
    Disconnected,
    /// Shared so the overlay can draw and the editor can write through the same
    /// `Keyboard` while other UI code mutates app state.
    Connected {
        keyboard: Arc<Keyboard>,
        profile: Arc<dyn crate::keymap_editor::EditorProfile>,
    },
    Reconnecting {
        next_attempt_at: Instant,
    },
}

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

pub struct SessionState {
    pub connection: AppConnectionState,
    pub ever_connected: bool,
    pub last_spec: Option<ConnectionSpec>,
    pub reopen: Option<Arc<dyn Reopener>>,
    pub preferred_layout_name: Option<String>,
}

pub struct ConnectDraftState {
    pub available_devices: Vec<DiscoveredDevice>,
    pub selected_device_index: Option<usize>,
    pub layout_file_path: String,
    pub pending_connect: Option<ConnectionTask>,
}
