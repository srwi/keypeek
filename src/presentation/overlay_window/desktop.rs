//! Desktop-specific window orchestration, native file picker, and connection settings UI.

use super::{OverlayApp, OverlayHost};
use crate::ui_widgets::titled_group;

/// Background clear color for desktop: transparent overlay or dimmed modal backdrop.
pub fn clear_color(is_any_window_open: bool) -> egui::Rgba {
    if is_any_window_open {
        egui::Rgba::from_black_alpha(0.65)
    } else {
        egui::Rgba::TRANSPARENT
    }
}

/// Desktop-specific state holding native file picker and window passthrough state.
pub struct DesktopPlatform {
    pub(crate) file_dialog: egui_file_dialog::FileDialog,
    pub(crate) mouse_passthrough: Option<bool>,
}

impl Default for DesktopPlatform {
    fn default() -> Self {
        Self {
            file_dialog: egui_file_dialog::FileDialog::new(),
            mouse_passthrough: None,
        }
    }
}

impl DesktopPlatform {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OverlayApp {
    /// Background clear color for desktop: transparent overlay or dimmed modal backdrop.
    pub(super) fn platform_clear_color(&self) -> egui::Rgba {
        clear_color(self.is_any_window_open())
    }

    /// Update phase for desktop: updates file dialog, handles picked layout files, and syncs passthrough.
    pub(super) fn update_platform(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        self.platform.file_dialog.update(ctx);

        if let Some(path) = self.platform.file_dialog.take_picked() {
            self.connection_mgr
                .set_layout_file_path(path.to_string_lossy().to_string());
            self.connect_from_ui();
        }

        self.sync_mouse_passthrough(host);
    }

    /// No-op on desktop (connection UI is embedded inside Settings window).
    pub(super) fn render_platform(
        &mut self,
        _ctx: &egui::Context,
        _keyboard: Option<&crate::application::Keyboard>,
    ) {
    }

    /// Triggers the native file dialog to pick a layout file.
    pub(super) fn pick_layout_file(&mut self) {
        self.platform.file_dialog.pick_file();
    }

    /// Synchronizes native OS mouse passthrough mode with the current window state.
    pub(super) fn sync_mouse_passthrough(&mut self, host: &mut dyn OverlayHost) {
        let mouse_passthrough = !self.is_any_window_open();
        if self.platform.mouse_passthrough == Some(mouse_passthrough) {
            return;
        }

        host.set_passthrough(mouse_passthrough);
        self.platform.mouse_passthrough = Some(mouse_passthrough);
    }

    /// Renders the Connection section inside KeyPeek Settings window on desktop.
    pub(super) fn draw_connection_settings(&mut self, ui: &mut egui::Ui) {
        titled_group(ui, "Connection", |ui| {
            let reconnecting = self.connection_mgr.is_reconnecting();
            // Keep the device/protocol pickers locked while connected or reconnecting.
            let connection_locked = self.connection_mgr.is_locked();
            let selected_device = self.connection_mgr.selected_device().cloned();
            let selected_device_text = selected_device
                .as_ref()
                .map(|d| d.display_name())
                .unwrap_or_else(|| "Select device...".to_string());

            let control_spacing = ui.spacing().item_spacing.x;
            const RIGHT_COLUMN_WIDTH: f32 = 100.0;

            egui::Grid::new("connection_grid")
                .num_columns(2)
                .striped(true)
                .spacing([20.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Device");
                    ui.add_enabled_ui(!connection_locked, |ui| {
                        ui.horizontal(|ui| {
                            let combo_width = (ui.available_width()
                                - RIGHT_COLUMN_WIDTH
                                - control_spacing)
                                .max(120.0);
                            egui::ComboBox::from_id_salt("device_combo")
                                .width(combo_width)
                                .selected_text(selected_device_text.clone())
                                .show_ui(ui, |ui| {
                                    for idx in 0..self.connection_mgr.available_devices().len() {
                                        let device =
                                            &self.connection_mgr.available_devices()[idx];
                                        let selected =
                                            self.connection_mgr.selected_device_index() == Some(idx);
                                        if ui.selectable_label(selected, device.display_name()).clicked() {
                                            self.connection_mgr.select_device(idx);
                                            self.ui.settings_error = None;
                                        }
                                    }
                                    if self.connection_mgr.available_devices().is_empty() {
                                        ui.weak("No devices found");
                                    }
                                });

                            ui.allocate_ui_with_layout(
                                egui::vec2(RIGHT_COLUMN_WIDTH, 20.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    let connect_in_progress = self.connection_mgr.is_connecting();
                                    let can_connect = !connection_locked
                                        && !connect_in_progress
                                        && self.connection_mgr.selected_device_index().is_some();
                                    let button_label = if reconnecting {
                                        "Reconnecting..."
                                    } else if connect_in_progress {
                                        "Connecting..."
                                    } else {
                                        "Connect"
                                    };
                                    ui.add_enabled_ui(can_connect, |ui| {
                                        if ui
                                            .add_sized(
                                                [RIGHT_COLUMN_WIDTH, 20.0],
                                                egui::Button::new(button_label),
                                            )
                                            .clicked()
                                        {
                                            self.connect_from_ui();
                                        }
                                    });
                                },
                            );
                        });
                    });
                    ui.end_row();

                    ui.label("Layout");
                    ui.horizontal(|ui| {
                        let (layout_enabled, current_layout, layout_names) =
                            if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
                                (
                                    keyboard.supports_live_layout_switching(),
                                    keyboard.active_layout_name(),
                                    keyboard.layout_names(),
                                )
                            } else {
                                (false, "Connect to device first".to_string(), Vec::new())
                            };
                        let layout_width =
                            (ui.available_width() - RIGHT_COLUMN_WIDTH - control_spacing).max(120.0);
                        ui.add_enabled_ui(layout_enabled, |ui| {
                            egui::ComboBox::from_id_salt("layout_combo")
                                .width(layout_width)
                                .selected_text(&current_layout)
                                .show_ui(ui, |ui| {
                                    for name in &layout_names {
                                        let is_selected = name == &current_layout;
                                        if ui.selectable_label(is_selected, name).clicked()
                                            && !is_selected
                                        {
                                            if let Some(keyboard) =
                                                self.connection_mgr.connected_keyboard()
                                            {
                                                if let Err(e) = keyboard.switch_layout(name) {
                                                    self.ui.settings_error = Some(e);
                                                } else {
                                                    self.connection_mgr
                                                        .set_preferred_layout_name(Some(name.clone()));
                                                }
                                            }
                                        }
                                    }
                                });
                        });
                        ui.allocate_space(egui::vec2(RIGHT_COLUMN_WIDTH, 20.0));
                    });
                    ui.end_row();
                });
        });
    }
}
