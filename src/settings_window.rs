use crate::protocols::qmk_json_parser;
use crate::protocols::via::ViaProtocol;
use crate::protocols::vial::VialProtocol;
use crate::protocols::KeyboardProtocol;
use crate::settings::ProtocolType;
use crate::settings::Settings;
use crate::settings::WindowPosition;

use eframe::egui::{self};
use qmk_via_api::scan::{scan_keyboards, KeyboardDeviceInfo};
use std::sync::{Arc, Mutex};

pub struct SettingsApp {
    current: Settings,
    shared: Arc<Mutex<Settings>>,
    error: Option<String>,
    layout_names: Vec<String>,
    available_devices: Vec<KeyboardDeviceInfo>,
    selected_device_index: Option<usize>,
    connected: bool,
    is_vial_device: bool,
    json_path: String,
}

impl SettingsApp {
    pub fn new(shared: Arc<Mutex<Settings>>) -> Self {
        let current = shared.lock().map(|s| s.clone()).unwrap_or_default();
        let mut app = Self {
            json_path: current.device_identifier.clone(),
            current,
            shared,
            error: None,
            layout_names: Vec::new(),
            available_devices: Vec::new(),
            selected_device_index: None,
            connected: false,
            is_vial_device: false,
        };
        app.refresh_devices();
        app
    }

    fn refresh_devices(&mut self) {
        self.available_devices = scan_keyboards();
        self.selected_device_index = None;
        self.connected = false;
        self.is_vial_device = false;
        self.layout_names.clear();
    }

    fn select_device(&mut self, index: usize) {
        if let Some(device) = self.available_devices.get(index) {
            self.selected_device_index = Some(index);
            self.connected = false;
            self.layout_names.clear();

            let vial_result = VialProtocol::connect(device.vendor_id, device.product_id);
            self.is_vial_device = vial_result.is_ok();
            drop(vial_result); // Explicitly drop to release HID handle

            if self.is_vial_device {
                self.json_path = format!("{:04x}:{:04x}", device.vendor_id, device.product_id);
            } else {
                self.json_path = String::new();
            }
            self.error = None;
        }
    }

    fn connect(&mut self) {
        let Some(device) = self
            .selected_device_index
            .and_then(|i| self.available_devices.get(i))
        else {
            self.error = Some("No device selected".to_string());
            return;
        };

        if self.is_vial_device {
            match VialProtocol::connect(device.vendor_id, device.product_id) {
                Ok(vial) => {
                    self.current.protocol_type = ProtocolType::Vial;
                    self.current.device_identifier =
                        format!("{:04x}:{:04x}", device.vendor_id, device.product_id);
                    self.layout_names = vial.get_layout_definition().get_layout_names();
                    self.connected = true;
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(format!("Failed to connect via VIAL: {e}"));
                }
            }
        } else {
            let path = self.json_path.trim();
            if path.is_empty() {
                self.error = Some("Please provide a JSON config file path".to_string());
                return;
            }

            match qmk_json_parser::parse_qmk_json(path) {
                Ok(definition) => {
                    if let Err(e) = ViaProtocol::connect(path) {
                        self.error = Some(format!("Failed to connect via VIA: {e}"));
                        return;
                    }
                    self.current.protocol_type = ProtocolType::Via;
                    self.current.device_identifier = path.to_string();
                    self.layout_names = definition.get_layout_names();
                    self.connected = true;
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(format!("Failed to parse JSON config: {e}"));
                }
            }
        }

        // Set default layout if needed
        if let Some(first) = self.layout_names.first() {
            if !self.layout_names.contains(&self.current.layout_name) {
                self.current.layout_name = first.clone();
            }
        }
    }

    fn device_display_name(device: &KeyboardDeviceInfo) -> String {
        device
            .product
            .clone()
            .unwrap_or_else(|| format!("{:04X}:{:04X}", device.vendor_id, device.product_id))
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame {
                inner_margin: egui::Margin::symmetric(30, 20),
                fill: ctx.style().visuals.window_fill,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.heading("QMK Layout Helper");
                    ui.hyperlink_to(
                        format!("Version {}", env!("CARGO_PKG_VERSION")),
                        "https://github.com/srwi/qmk-layout-helper",
                    );

                    ui.add_space(20.0);

                    egui::Grid::new("settings_grid")
                        .num_columns(2)
                        .striped(true)
                        .spacing([25.0, 14.0])
                        .show(ui, |ui| {
                            // Device selector
                            ui.label("Device");
                            ui.horizontal(|ui| {
                                let combo_width = ui.available_width() - 80.0;

                                let selected_text = self
                                    .selected_device_index
                                    .and_then(|i| self.available_devices.get(i))
                                    .map(|d| Self::device_display_name(d))
                                    .unwrap_or_else(|| "Select device...".to_string());

                                egui::ComboBox::from_id_salt("device_combo")
                                    .width(combo_width)
                                    .selected_text(selected_text)
                                    .show_ui(ui, |ui| {
                                        let device_count = self.available_devices.len();
                                        for idx in 0..device_count {
                                            let device = &self.available_devices[idx];
                                            let is_selected =
                                                self.selected_device_index == Some(idx);
                                            if ui
                                                .selectable_label(
                                                    is_selected,
                                                    Self::device_display_name(device),
                                                )
                                                .clicked()
                                            {
                                                self.select_device(idx);
                                            }
                                        }
                                        if self.available_devices.is_empty() {
                                            ui.weak("No devices found");
                                        }
                                    });

                                if ui
                                    .add_sized([70.0, 20.0], egui::Button::new("Refresh"))
                                    .clicked()
                                {
                                    self.refresh_devices();
                                }
                            });
                            ui.end_row();

                            // Config / Connect row
                            ui.label(if self.is_vial_device {
                                "Device ID"
                            } else {
                                "JSON Config"
                            });
                            ui.horizontal(|ui| {
                                let input_width = ui.available_width() - 90.0;

                                let input_interactive = !self.is_vial_device && !self.connected;
                                ui.add_sized(
                                    [input_width, 20.0],
                                    egui::TextEdit::singleline(&mut self.json_path)
                                        .hint_text("Path to keyboard info JSON")
                                        .interactive(input_interactive),
                                );

                                let connect_enabled =
                                    self.selected_device_index.is_some() && !self.connected;
                                let button_text = if self.connected {
                                    "Connected"
                                } else {
                                    "Connect"
                                };

                                ui.add_enabled_ui(connect_enabled, |ui| {
                                    if ui
                                        .add_sized([80.0, 20.0], egui::Button::new(button_text))
                                        .clicked()
                                    {
                                        self.connect();
                                    }
                                });
                            });
                            ui.end_row();

                            // Layout selection
                            ui.label("Layout");
                            let layout_enabled = !self.layout_names.is_empty();
                            ui.add_enabled_ui(layout_enabled, |ui| {
                                let selected_text = if self.layout_names.is_empty() {
                                    "Connect to device first".to_string()
                                } else {
                                    self.current.layout_name.clone()
                                };
                                egui::ComboBox::from_id_salt("layout_combo")
                                    .width(ui.available_width())
                                    .selected_text(selected_text)
                                    .show_ui(ui, |ui| {
                                        for name in &self.layout_names {
                                            ui.selectable_value(
                                                &mut self.current.layout_name,
                                                name.clone(),
                                                name,
                                            );
                                        }
                                    });
                            });
                            ui.end_row();

                            let position_label = self.current.position.to_string();
                            ui.label("Alignment");
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("position_combo")
                                    .width(ui.available_width())
                                    .selected_text(position_label)
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
                                                &mut self.current.position,
                                                pos,
                                                pos.to_string(),
                                            );
                                        }
                                    });
                            });
                            ui.end_row();

                            ui.label("Distance from screen edge");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.current.margin)
                                    .speed(1)
                                    .suffix(" px"),
                            );
                            ui.end_row();

                            ui.label("Key unit size");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.current.size)
                                    .speed(1)
                                    .range(20..=1000)
                                    .suffix(" px"),
                            );
                            ui.end_row();

                            ui.label("Display duration");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.current.timeout)
                                    .speed(50)
                                    .range(0..=60_000)
                                    .suffix(" ms"),
                            );
                            ui.end_row();
                        });

                    ui.add_space(20.0);
                    ui.checkbox(&mut self.current.save_settings, "Remember settings");
                    ui.add_space(5.0);
                    ui.add_enabled_ui(self.connected && !self.layout_names.is_empty(), |ui| {
                        if ui
                            .add_sized([90.0, 28.0], egui::Button::new("Start"))
                            .clicked()
                        {
                            if let Ok(mut settings) = self.shared.lock() {
                                settings.protocol_type = self.current.protocol_type;
                                settings.device_identifier = self.current.device_identifier.clone();
                                settings.layout_name = self.current.layout_name.clone();
                                settings.size = self.current.size;
                                settings.position = self.current.position;
                                settings.timeout = self.current.timeout;
                                settings.margin = self.current.margin;
                                settings.confirmed = true;
                                settings.save_settings = self.current.save_settings;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    });
                });
            });

        if let Some(error_message) = self.error.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(error_message);
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.error = None;
                    }
                });
        }
    }
}
