use crate::device_discovery::DiscoveredDevice;
use crate::platform::OverlayHost;
use crate::settings::Settings;
use crate::ui_wake::UiWake;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

mod connection_flow;
mod settings_sync;
mod state;
mod ui_overlay;
mod ui_settings;
use state::{
    AppConnectionState, ConnectDraftState, ConnectionDraft, SessionState, SettingsState, UiState,
};

pub struct OverlayApp {
    _tray: crate::tray::Tray,
    settings_requested: Arc<AtomicBool>,
    ui_wake: UiWake,
    ui: UiState,
    settings: SettingsState,
    session: SessionState,
    connect: ConnectDraftState,
}

impl OverlayApp {
    pub fn new(
        tray: crate::tray::Tray,
        settings_requested: Arc<AtomicBool>,
        ui_wake: UiWake,
        base_settings: Settings,
        available_devices: Vec<DiscoveredDevice>,
    ) -> Self {
        Self {
            _tray: tray,
            settings_requested,
            ui_wake,
            ui: UiState {
                settings_visible: true,
                settings_error: None,
                settings_warning: None,
                mouse_passthrough: None,
                file_dialog: egui_file_dialog::FileDialog::new(),
            },
            settings: SettingsState {
                active: base_settings.clone(),
                draft: base_settings,
            },
            session: SessionState {
                connection: AppConnectionState::Disconnected,
                ever_connected: false,
                last_spec: None,
                reopen: None,
                connected_definition: None,
                layout_names: Vec::new(),
                active_layout_name: String::new(),
                draft_layout_name: String::new(),
            },
            connect: ConnectDraftState {
                available_devices,
                selected_device_index: None,
                draft: ConnectionDraft::Via {
                    json_path: String::new(),
                },
                pending_connect: None,
            },
        }
    }

    fn sync_mouse_passthrough(&mut self, host: &mut dyn OverlayHost) {
        let mouse_passthrough = !self.ui.settings_visible;
        if self.ui.mouse_passthrough == Some(mouse_passthrough) {
            return;
        }

        host.set_passthrough(mouse_passthrough);
        self.ui.mouse_passthrough = Some(mouse_passthrough);
    }

    /// Draw a centered modal with `message` and an OK button that clears `slot`.
    fn message_window(ctx: &egui::Context, title: &str, slot: &mut Option<String>) {
        let Some(message) = slot.clone() else {
            return;
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(message);
                ui.add_space(10.0);
                if ui.button("OK").clicked() {
                    *slot = None;
                }
            });
    }

    /// Wakes the UI up when the overlay is due to appear or disappear on its own.
    fn schedule_overlay_repaint(&self, ctx: &egui::Context) {
        if self.ui.settings_visible {
            return;
        }

        let AppConnectionState::Connected { keyboard } = &self.session.connection else {
            return;
        };

        if let Some(delay) = keyboard.overlay_changes_in(Instant::now()) {
            ctx.request_repaint_after(delay);
        }
    }
}

impl OverlayApp {
    /// Backdrop color the host clears to before egui paints: dimmed while settings
    /// are open, otherwise transparent so only the overlay is visible.
    pub fn clear_color(&self) -> egui::Rgba {
        if self.ui.settings_visible {
            egui::Rgba::from_black_alpha(0.65)
        } else {
            egui::Rgba::TRANSPARENT
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        if self.settings_requested.swap(false, Ordering::Relaxed) {
            self.ui.settings_visible = true;
        }

        self.poll_connect_result();
        self.maintain_connection(ctx);
        self.apply_live_visual_settings();
        self.apply_live_layout_settings();
        self.ui.file_dialog.update(ctx);

        if let Some(path) = self.ui.file_dialog.take_picked() {
            if let ConnectionDraft::Via { json_path } = &mut self.connect.draft {
                *json_path = path.to_string_lossy().to_string();
            }
            self.connect_from_ui();
        }

        self.sync_mouse_passthrough(host);

        if let AppConnectionState::Connected { keyboard } = &self.session.connection {
            self.draw_overlay_window(ctx, keyboard, self.overlay_visible());
        }

        if self.ui.settings_visible {
            self.draw_settings_window(ctx, host);
        }

        Self::message_window(ctx, "Error", &mut self.ui.settings_error);
        Self::message_window(ctx, "Notice", &mut self.ui.settings_warning);

        self.schedule_overlay_repaint(ctx);
    }
}
