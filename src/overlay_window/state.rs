use crate::connection::ConnectionTask;
use crate::device_discovery::DiscoveredDevice;
use crate::keyboard::Keyboard;
use crate::protocols::KeyboardDefinition;
use crate::settings::{ProtocolType, Settings};

use eframe::egui;
use egui_file_dialog::FileDialog;

pub struct LabelGalleys {
    pub symbol: Option<std::sync::Arc<egui::Galley>>,
    pub text: Option<std::sync::Arc<egui::Galley>>,
}

pub enum AppConnectionState {
    Disconnected,
    Connected { keyboard: Keyboard },
}

#[derive(Clone)]
pub enum ZmkTransportDraft {
    Serial { port_name: Option<String> },
    Ble { device_id: Option<String> },
}

pub struct UiState {
    pub settings_visible: bool,
    pub settings_error: Option<String>,
    pub settings_warning: Option<String>,
    #[cfg(target_os = "macos")]
    pub macos_maximized: bool,
    pub file_dialog: FileDialog,
}

pub struct SettingsState {
    pub active: Settings,
    pub draft: Settings,
}

pub struct SessionState {
    pub connection: AppConnectionState,
    pub ever_connected: bool,
    pub connected_definition: Option<KeyboardDefinition>,
    pub layout_names: Vec<String>,
}

pub struct ConnectDraftState {
    pub available_devices: Vec<DiscoveredDevice>,
    pub selected_device_index: Option<usize>,
    pub protocol_type: ProtocolType,
    pub json_path: String,
    pub zmk_transport: ZmkTransportDraft,
    pub pending_connect: Option<ConnectionTask>,
}
