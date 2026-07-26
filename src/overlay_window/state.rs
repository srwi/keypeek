use crate::connection::ConnectionTask;
use crate::device_discovery::{DiscoveredDevice, DiscoveryTask};
use crate::keyboard::Keyboard;
use crate::protocols::{ConnectionSpec, KeyboardDefinition, Reopener};
use crate::settings::{ProtocolType, Settings};

use egui_file_dialog::FileDialog;
use std::sync::Arc;
use std::time::Instant;

pub struct LabelGalleys {
    pub symbol: Option<std::sync::Arc<egui::Galley>>,
    pub text: Option<std::sync::Arc<egui::Galley>>,
    pub behavior: Option<std::sync::Arc<egui::Galley>>,
    pub argument: Option<std::sync::Arc<egui::Galley>>,
}

/// Resolved colors for painting a single key, derived from its layer, kind, and state.
pub struct KeyColors {
    pub fill: egui::Color32,
    pub border: egui::Color32,
    pub border_thickness: f32,
    pub font: egui::Color32,
}

pub enum AppConnectionState {
    Disconnected,
    Connected {
        keyboard: Keyboard,
    },
    Reconnecting {
        next_attempt_at: Instant,
        /// `None` retries forever, which is what a mid-session drop gets; startup
        /// auto-connect is bounded so an absent keyboard stops being chased.
        attempts_left: Option<u32>,
    },
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
    Mock,
}

impl ConnectionDraft {
    pub fn protocol_type(&self) -> ProtocolType {
        match self {
            ConnectionDraft::Via { .. } => ProtocolType::Via,
            ConnectionDraft::Vial => ProtocolType::Vial,
            ConnectionDraft::Zmk { .. } => ProtocolType::Zmk,
            ConnectionDraft::Mock => ProtocolType::Via,
        }
    }
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
    pub connected_definition: Option<KeyboardDefinition>,
    pub layout_names: Vec<String>,
    pub active_layout_name: String,
    pub draft_layout_name: String,
}

impl SessionState {
    pub fn disconnected() -> Self {
        Self {
            connection: AppConnectionState::Disconnected,
            ever_connected: false,
            last_spec: None,
            reopen: None,
            connected_definition: None,
            layout_names: Vec::new(),
            active_layout_name: String::new(),
            draft_layout_name: String::new(),
        }
    }

    /// `ever_connected` survives: it keeps closing the settings window from quitting the app.
    pub fn clear_connection(&mut self) {
        let ever_connected = self.ever_connected;
        *self = Self::disconnected();
        self.ever_connected = ever_connected;
    }
}

pub struct ConnectDraftState {
    pub available_devices: Vec<DiscoveredDevice>,
    pub selected_device_index: Option<usize>,
    pub draft: ConnectionDraft,
    pub pending_connect: Option<ConnectionTask>,
    pub discovery: Option<DiscoveryTask>,
}
