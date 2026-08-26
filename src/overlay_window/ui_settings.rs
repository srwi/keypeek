use super::state::AppConnectionState;
use super::OverlayApp;
use crate::settings::{LayerMask, LegendMode, Settings, ThemeColor, ThemeSettings, WindowPosition};
use egui::Window;

impl OverlayApp {
    /// The color button that ends every theme row, right-aligned in the row.
    fn theme_color_button(ui: &mut egui::Ui, color: &mut ThemeColor) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut display_color = Self::to_egui_color(*color);
            if ui.color_edit_button_srgba(&mut display_color).changed() {
                *color = Self::from_egui_color(display_color);
            }
        });
    }

    /// A theme row: a label and the color it stands for.
    fn theme_color_entry(ui: &mut egui::Ui, label: &str, color: &mut ThemeColor) {
        ui.horizontal(|ui| {
            ui.label(label);
            Self::theme_color_button(ui, color);
        });
        ui.add_space(4.0);
    }

    /// A theme row for the layers in the `layers` bitmask: their visibility checkbox,
    /// the label, and the color they are painted in.
    fn theme_layer_entry(
        ui: &mut egui::Ui,
        label: &str,
        color: &mut ThemeColor,
        visible_layers: &mut LayerMask,
        layers: u32,
    ) {
        ui.horizontal(|ui| {
            let mut shown = visible_layers.contains_any(layers);
            let toggle_size = ui.spacing().interact_size.y;
            if ui
                .add_sized(
                    [toggle_size, toggle_size],
                    egui::Checkbox::without_text(&mut shown),
                )
                .on_hover_text("Show the overlay while these layers are active")
                .changed()
            {
                visible_layers.set(layers, shown);
            }
            ui.label(label);
            Self::theme_color_button(ui, color);
        });
        ui.add_space(4.0);
    }

    fn timeout_to_ui_value(timeout: i64) -> u32 {
        if timeout < 0 {
            15_000
        } else {
            (timeout as u32).min(14_999)
        }
    }

    fn ui_value_to_timeout(value: u32) -> i64 {
        if value >= 15_000 {
            -1
        } else {
            value as i64
        }
    }

    pub(super) fn draw_settings_window(
        &mut self,
        ctx: &egui::Context,
        host: &mut dyn crate::platform::OverlayHost,
    ) {
        let mut open = self.ui.settings_visible;
        let reconnecting = matches!(
            self.session.connection,
            AppConnectionState::Reconnecting { .. }
        );
        // Keep the device/protocol pickers locked while connected or reconnecting.
        let connection_locked =
            !matches!(self.session.connection, AppConnectionState::Disconnected);
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

        Window::new("KeyPeek Settings")
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
                                let layout_enabled = !self.session.layout_names.is_empty();
                                let layout_width =
                                    (ui.available_width() - RIGHT_COLUMN_WIDTH - control_spacing)
                                        .max(120.0);
                                ui.add_enabled_ui(layout_enabled, |ui| {
                                    let selected_text = if self.session.layout_names.is_empty() {
                                        "Connect to device first".to_string()
                                    } else {
                                        self.session.draft_layout_name.clone()
                                    };
                                    egui::ComboBox::from_id_salt("layout_combo")
                                        .width(layout_width)
                                        .selected_text(selected_text)
                                        .show_ui(ui, |ui| {
                                            for name in &self.session.layout_names {
                                                ui.selectable_value(
                                                    &mut self.session.draft_layout_name,
                                                    name.clone(),
                                                    name,
                                                );
                                            }
                                        });
                                });
                                ui.allocate_space(egui::vec2(RIGHT_COLUMN_WIDTH, 20.0));
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

                            ui.label("Display duration");
                            let mut timeout_ui =
                                Self::timeout_to_ui_value(self.settings.draft.timeout);
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut timeout_ui)
                                    .speed(50)
                                    .range(0..=15_000)
                                    .custom_formatter(|value, _range| {
                                        if value >= 15_000.0 {
                                            "∞".to_string()
                                        } else {
                                            format!("{} ms", value as i64)
                                        }
                                    }),
                            );
                            self.settings.draft.timeout = Self::ui_value_to_timeout(timeout_ui);
                            ui.end_row();

                            ui.label("Activation delay");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.settings.draft.activation_delay)
                                    .speed(10)
                                    .range(0..=Settings::MAX_ACTIVATION_DELAY_MS)
                                    .suffix(" ms"),
                            )
                            .on_hover_text(
                                "How long a layer has to be held before the overlay appears",
                            );
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

                            ui.label("Key label font scale");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.settings.draft.font_size_multiplier)
                                    .speed(0.01)
                                    .range(0.5..=1.5)
                                    .suffix(" x"),
                            );
                            ui.end_row();

                            ui.label("Auto-fit long labels");
                            ui.checkbox(
                                &mut self.settings.draft.auto_fit_before_ellipsis,
                                "Fit long labels to available space",
                            );
                            ui.end_row();

                            ui.label("Legend mode");
                            egui::ComboBox::from_id_salt("legend_mode_combo")
                                .width(ui.available_width())
                                .selected_text(self.settings.draft.legend_mode.to_string())
                                .show_ui(ui, |ui| {
                                    for mode in [
                                        LegendMode::Stacked,
                                        LegendMode::Single,
                                        LegendMode::SingleLive,
                                    ] {
                                        ui.selectable_value(
                                            &mut self.settings.draft.legend_mode,
                                            mode,
                                            mode.to_string(),
                                        );
                                    }
                                })
                                .response
                                .on_hover_text(
                                    "Stacked: Base+Shifted shown together (default). Single: \
                                     only what a plain tap produces. Single + live preview: \
                                     also swaps to the Shift/RAlt result while that modifier \
                                     is physically held.",
                                );
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Theme");
                    ui.add_space(8.0);

                    const LAYER_LABELS: [&str; ThemeSettings::OTHER_LAYERS as usize] = [
                        "Layer 0", "Layer 1", "Layer 2", "Layer 3", "Layer 4", "Layer 5",
                    ];
                    const HALF: usize = LAYER_LABELS.len() / 2;

                    let draft = &mut self.settings.draft;

                    let mut layer_row = |ui: &mut egui::Ui, layer: usize| {
                        let (label, mask) = if layer == ThemeSettings::OTHER_LAYERS as usize {
                            ("Other layers", !0 << layer)
                        } else {
                            (LAYER_LABELS[layer], 1 << layer)
                        };
                        Self::theme_layer_entry(
                            ui,
                            label,
                            &mut draft.theme.layer_colors[layer],
                            &mut draft.visible_layers,
                            mask,
                        );
                    };
                    ui.columns(2, |columns| {
                        columns[0].vertical(|ui| {
                            Self::theme_color_entry(ui, "Font color", &mut draft.theme.font_color);
                            for layer in 0..HALF {
                                layer_row(ui, layer);
                            }
                        });
                        columns[1].vertical(|ui| {
                            for layer in HALF..LAYER_LABELS.len() {
                                layer_row(ui, layer);
                            }
                            layer_row(ui, ThemeSettings::OTHER_LAYERS as usize);
                        });
                    });
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add(egui::Hyperlink::from_label_and_url(
                        egui::RichText::new("github.com/srwi/keypeek").weak(),
                        "https://github.com/srwi/keypeek",
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Hyperlink::from_label_and_url(
                            egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                                .weak(),
                            "https://github.com/srwi/keypeek/releases",
                        ));
                    });
                });
            });

        if self.ui.settings_visible && !open {
            self.ui.settings_visible = false;
            self.persist_settings();
            if !self.session.ever_connected {
                host.request_close();
            }
        }
    }
}
