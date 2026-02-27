use super::state::AppConnectionState;
use super::OverlayApp;
use crate::settings::WindowPosition;
use eframe::egui::{self, Window};

impl OverlayApp {
    pub(super) fn draw_settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.ui.settings_visible;
        let connection_locked = matches!(
            self.session.connection,
            AppConnectionState::Connected { .. }
        );
        let selected_device = self
            .connect
            .selected_device_index
            .and_then(|i| self.connect.available_devices.get(i))
            .cloned();
        let selected_device_text = selected_device
            .as_ref()
            .map(|d| d.display_name())
            .unwrap_or_else(|| "Select device...".to_string());

        let settings_window_size = egui::vec2(450.0, 700.0);
        let settings_window_pos = ctx.viewport_rect().center() - settings_window_size * 0.5;

        Window::new("QMK Layout Helper Settings")
            .open(&mut open)
            .default_size(settings_window_size)
            .default_pos(settings_window_pos)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.heading("Connection");
                    ui.add_space(8.0);
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
                                            for idx in 0..self.connect.available_devices.len() {
                                                let device = &self.connect.available_devices[idx];
                                                let selected =
                                                    self.connect.selected_device_index == Some(idx);
                                                if ui
                                                    .selectable_label(
                                                        selected,
                                                        device.display_name(),
                                                    )
                                                    .clicked()
                                                {
                                                    self.select_device(idx);
                                                }
                                            }
                                            if self.connect.available_devices.is_empty() {
                                                ui.weak("No devices found");
                                            }
                                        });

                                    ui.allocate_ui_with_layout(
                                        egui::vec2(RIGHT_COLUMN_WIDTH, 20.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            let connect_in_progress =
                                                self.connect.pending_connect.is_some();
                                            let can_connect = !connection_locked
                                                && !connect_in_progress
                                                && self.connect.selected_device_index.is_some();
                                            let button_label = if connect_in_progress {
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
                                let layout_enabled = !self.session.layout_names.is_empty();
                                let layout_width =
                                    (ui.available_width() - RIGHT_COLUMN_WIDTH - control_spacing)
                                        .max(120.0);
                                ui.add_enabled_ui(layout_enabled, |ui| {
                                    let selected_text = if self.session.layout_names.is_empty() {
                                        "Connect to device first".to_string()
                                    } else {
                                        self.settings.draft.layout_name.clone()
                                    };
                                    egui::ComboBox::from_id_salt("layout_combo")
                                        .width(layout_width)
                                        .selected_text(selected_text)
                                        .show_ui(ui, |ui| {
                                            for name in &self.session.layout_names {
                                                ui.selectable_value(
                                                    &mut self.settings.draft.layout_name,
                                                    name.clone(),
                                                    name,
                                                );
                                            }
                                        });
                                });
                                ui.allocate_ui_with_layout(
                                    egui::vec2(RIGHT_COLUMN_WIDTH, 20.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.add(egui::Checkbox::new(
                                            &mut self.settings.draft.save_settings,
                                            "Auto-connect",
                                        ));
                                    },
                                );
                            });
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Overlay Appearance");
                    ui.add_space(8.0);

                    egui::Grid::new("appearance_grid")
                        .num_columns(2)
                        .striped(true)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Alignment");
                            egui::ComboBox::from_id_salt("position_combo")
                                .width(ui.available_width())
                                .selected_text(self.settings.draft.position.to_string())
                                .show_ui(ui, |ui| {
                                    for pos in [
                                        WindowPosition::TopLeft,
                                        WindowPosition::TopRight,
                                        WindowPosition::BottomLeft,
                                        WindowPosition::BottomRight,
                                        WindowPosition::Top,
                                        WindowPosition::Bottom,
                                    ] {
                                        ui.selectable_value(
                                            &mut self.settings.draft.position,
                                            pos,
                                            pos.to_string(),
                                        );
                                    }
                                });
                            ui.end_row();

                            ui.label("Distance from screen edge");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.settings.draft.margin)
                                    .speed(1)
                                    .suffix(" px"),
                            );
                            ui.end_row();

                            ui.label("Key unit size");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.settings.draft.size)
                                    .speed(1)
                                    .range(20..=1000)
                                    .suffix(" px"),
                            );
                            ui.end_row();

                            ui.label("Display duration");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.settings.draft.timeout)
                                    .speed(50)
                                    .range(0..=60_000)
                                    .suffix(" ms"),
                            );
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Theme");
                    ui.add_space(8.0);

                    ui.columns(2, |columns| {
                        columns[0].vertical(|ui| {
                            Self::theme_color_entry(
                                ui,
                                "Font color",
                                &mut self.settings.draft.theme.font_color,
                            );
                            Self::theme_color_entry(
                                ui,
                                "Layer 0 color",
                                &mut self.settings.draft.theme.layer_colors[0],
                            );
                            Self::theme_color_entry(
                                ui,
                                "Layer 1 color",
                                &mut self.settings.draft.theme.layer_colors[1],
                            );
                            Self::theme_color_entry(
                                ui,
                                "Layer 2 color",
                                &mut self.settings.draft.theme.layer_colors[2],
                            );
                        });

                        columns[1].vertical(|ui| {
                            Self::theme_color_entry(
                                ui,
                                "Layer 3 color",
                                &mut self.settings.draft.theme.layer_colors[3],
                            );
                            Self::theme_color_entry(
                                ui,
                                "Layer 4 color",
                                &mut self.settings.draft.theme.layer_colors[4],
                            );
                            Self::theme_color_entry(
                                ui,
                                "Layer 5 color",
                                &mut self.settings.draft.theme.layer_colors[5],
                            );
                            Self::theme_color_entry(
                                ui,
                                "Other layers color",
                                &mut self.settings.draft.theme.layer_colors[6],
                            );
                        });
                    });
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add(egui::Hyperlink::from_label_and_url(
                        egui::RichText::new("github.com/srwi/qmk-layout-helper").weak(),
                        "https://github.com/srwi/qmk-layout-helper",
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Hyperlink::from_label_and_url(
                            egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                                .weak(),
                            "https://github.com/srwi/qmk-layout-helper/releases",
                        ));
                    });
                });
            });

        if self.ui.settings_visible && !open {
            self.ui.settings_visible = false;
            if !self.session.ever_connected {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
