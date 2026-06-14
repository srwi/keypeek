use crate::connection::ConnectionTask;
use crate::device_discovery::DiscoveredDevice;
use crate::keyboard::Keyboard;
use crate::protocols::{ConnectionSpec, KeyboardDefinition};
use crate::settings::{ProtocolType, Settings};

use eframe::egui;
use egui_file_dialog::FileDialog;
use std::time::Instant;

pub struct LabelGalleys {
    pub symbol: Option<std::sync::Arc<egui::Galley>>,
    pub text: Option<std::sync::Arc<egui::Galley>>,
    pub behavior: Option<std::sync::Arc<egui::Galley>>,
    pub argument: Option<std::sync::Arc<egui::Galley>>,
}

pub enum AppConnectionState {
    Disconnected,
    Connected { keyboard: Keyboard },
    Reconnecting { next_attempt_at: Instant },
}

#[derive(Clone)]
pub enum ZmkTransportDraft {
    Serial { port_name: Option<String> },
    Ble { device_id: Option<String> },
}

#[derive(Clone)]
pub enum ConnectionDraft {
    Via { json_path: String },
    Vial,
    Zmk { transport: ZmkTransportDraft },
}

impl ConnectionDraft {
    pub fn protocol_type(&self) -> ProtocolType {
        match self {
            ConnectionDraft::Via { .. } => ProtocolType::Via,
            ConnectionDraft::Vial => ProtocolType::Vial,
            ConnectionDraft::Zmk { .. } => ProtocolType::Zmk,
        }
    }
}

pub struct UiState {
    pub settings_visible: bool,
    pub settings_error: Option<String>,
    pub settings_warning: Option<String>,
    pub mouse_passthrough: Option<bool>,
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
    pub last_spec: Option<ConnectionSpec>,
    pub connected_definition: Option<KeyboardDefinition>,
    pub layout_names: Vec<String>,
    pub active_layout_name: String,
    pub draft_layout_name: String,
}

pub struct ConnectDraftState {
    pub available_devices: Vec<DiscoveredDevice>,
    pub selected_device_index: Option<usize>,
    pub draft: ConnectionDraft,
    pub pending_connect: Option<ConnectionTask>,
}
