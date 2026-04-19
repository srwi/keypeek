use crate::device_discovery::DiscoveredDevice;
use crate::settings::Settings;
use crate::ui_wake::UiWake;

use eframe::egui;
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
    _tray_icon: tray_icon::TrayIcon,
    ui_wake: UiWake,
    ui: UiState,
    settings: SettingsState,
    session: SessionState,
    connect: ConnectDraftState,
}

impl OverlayApp {
    pub fn new(
        tray_icon: tray_icon::TrayIcon,
        ui_wake: UiWake,
        base_settings: Settings,
        available_devices: Vec<DiscoveredDevice>,
    ) -> Self {
        Self {
            _tray_icon: tray_icon,
            ui_wake,
            ui: UiState {
                settings_visible: true,
                settings_error: None,
                settings_warning: None,
                mouse_passthrough: None,
                #[cfg(target_os = "macos")]
                macos_maximized: false,
                file_dialog: egui_file_dialog::FileDialog::new(),
            },
            settings: SettingsState {
                active: base_settings.clone(),
                draft: base_settings,
            },
            session: SessionState {
                connection: AppConnectionState::Disconnected,
                ever_connected: false,
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

    fn sync_mouse_passthrough(&mut self, ctx: &egui::Context) {
        let mouse_passthrough = !self.ui.settings_visible;
        if self.ui.mouse_passthrough == Some(mouse_passthrough) {
            return;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(mouse_passthrough));
        self.ui.mouse_passthrough = Some(mouse_passthrough);
    }

    fn schedule_overlay_hide_repaint(&self, ctx: &egui::Context) {
        if self.ui.settings_visible {
            return;
        }

        let AppConnectionState::Connected { keyboard } = &self.session.connection else {
            return;
        };

        let Some(time_to_hide) = keyboard
            .time_to_hide_overlay
            .lock()
            .unwrap()
            .as_ref()
            .copied()
        else {
            return;
        };

        if let Some(delay) = time_to_hide.checked_duration_since(Instant::now()) {
            ctx.request_repaint_after(delay);
        }
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if self.ui.settings_visible {
            egui::Rgba::from_black_alpha(0.65).to_array()
        } else {
            egui::Rgba::TRANSPARENT.to_array()
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // On macOS, with_maximized(true) doesn't work for undecorated transparent
        // windows. Explicitly size the window to fill the monitor on the first frame.
        #[cfg(target_os = "macos")]
        if !self.ui.macos_maximized {
            if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(0.0, 0.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(monitor_size));
                self.ui.macos_maximized = true;
            }
        }

        self.poll_connect_result();
        self.apply_live_visual_settings();
        self.apply_live_layout_settings();
        self.ui.file_dialog.update(ctx);

        if let Some(path) = self.ui.file_dialog.take_picked() {
            if let ConnectionDraft::Via { json_path } = &mut self.connect.draft {
                *json_path = path.to_string_lossy().to_string();
            }
            self.connect_from_ui();
        }

        self.sync_mouse_passthrough(ctx);

        if let AppConnectionState::Connected { keyboard } = &self.session.connection {
            if self.overlay_visible() {
                self.draw_overlay_window(ctx, keyboard);
            }
        }

        if self.ui.settings_visible {
            self.draw_settings_window(ctx);
        }

        if let Some(error_message) = self.ui.settings_error.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(error_message);
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.ui.settings_error = None;
                    }
                });
        }

        if let Some(warning_message) = self.ui.settings_warning.clone() {
            egui::Window::new("Notice")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(warning_message);
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.ui.settings_warning = None;
                    }
                });
        }

        self.schedule_overlay_hide_repaint(ctx);
    }
}
