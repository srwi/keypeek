use crate::protocols::zmk_studio;
use crate::protocols::{connect_protocol, connect_zmk_studio, format_vid_pid, parse_vid_pid};
use crate::settings::ProtocolType;
use crate::settings::Settings;
use crate::settings::WindowPosition;

use eframe::egui::{self};
use qmk_via_api::scan::{scan_keyboards, KeyboardDeviceInfo};
use std::sync::{Arc, Mutex};

/// Unified device entry shown in the device dropdown.
struct DeviceEntry {
    display_name: String,
    vid: u16,
    pid: u16,
    /// For HID devices (VIA/VIAL)
    #[allow(dead_code)]
    hid_info: Option<KeyboardDeviceInfo>,
    /// For ZMK Studio serial devices
    serial_port: Option<String>,
}

pub struct SettingsApp {
    current: Settings,
    shared: Arc<Mutex<Settings>>,
    error: Option<String>,
    layout_names: Vec<String>,
    available_devices: Vec<DeviceEntry>,
    selected_device_index: Option<usize>,
    connected: bool,
    protocol_type: ProtocolType,
    json_path: String,
    /// Tracks whether we're showing the unlock prompt
    zmk_unlock_pending: bool,
    /// Serial port name for ZMK Studio connection
    zmk_serial_port: Option<String>,
}

impl SettingsApp {
    pub fn new(shared: Arc<Mutex<Settings>>) -> Self {
        let current = shared.lock().map(|s| s.clone()).unwrap_or_default();
        let json_path = match current.protocol_type {
            ProtocolType::Via => current.protocol_config.clone(),
            ProtocolType::Vial => current.protocol_config.clone(),
            ProtocolType::Zmk => {
                // ZMK config format: "vid:pid|serial_port"
                current
                    .protocol_config
                    .split('|')
                    .next()
                    .unwrap_or("")
                    .to_string()
            }
        };
        let zmk_serial_port = if current.protocol_type == ProtocolType::Zmk {
            current
                .protocol_config
                .split_once('|')
                .map(|(_, p)| p.to_string())
        } else {
            None
        };
        let mut app = Self {
            json_path,
            protocol_type: current.protocol_type,
            current,
            shared,
            error: None,
            layout_names: Vec::new(),
            available_devices: Vec::new(),
            selected_device_index: None,
            connected: false,
            zmk_unlock_pending: false,
            zmk_serial_port,
        };
        app.refresh_devices();
        app
    }

    fn refresh_devices(&mut self) {
        self.available_devices.clear();
        self.selected_device_index = None;
        self.connected = false;
        self.layout_names.clear();

        // Add HID devices (for VIA/VIAL)
        for dev in scan_keyboards() {
            self.available_devices.push(DeviceEntry {
                display_name: dev
                    .product
                    .clone()
                    .unwrap_or_else(|| format!("{:04X}:{:04X}", dev.vendor_id, dev.product_id)),
                vid: dev.vendor_id,
                pid: dev.product_id,
                hid_info: Some(dev),
                serial_port: None,
            });
        }

        // Add ZMK Studio serial devices
        for sp in zmk_studio::scan_serial_ports() {
            // Skip if we already have an HID entry with the same VID/PID
            let already_listed = self
                .available_devices
                .iter()
                .any(|d| d.vid == sp.vid && d.pid == sp.pid);

            let display_name = sp
                .product
                .unwrap_or_else(|| format!("{:04X}:{:04X}", sp.vid, sp.pid));

            if already_listed {
                // Add the serial port info to the existing entry
                if let Some(entry) = self
                    .available_devices
                    .iter_mut()
                    .find(|d| d.vid == sp.vid && d.pid == sp.pid)
                {
                    entry.serial_port = Some(sp.port_name);
                    entry.display_name = format!("{} (Studio)", entry.display_name);
                }
            } else {
                self.available_devices.push(DeviceEntry {
                    display_name: format!("{} [{}]", display_name, sp.port_name),
                    vid: sp.vid,
                    pid: sp.pid,
                    hid_info: None,
                    serial_port: Some(sp.port_name),
                });
            }
        }
    }

    fn select_device(&mut self, index: usize) {
        if let Some(device) = self.available_devices.get(index) {
            self.selected_device_index = Some(index);
            self.connected = false;
            self.layout_names.clear();
            self.zmk_unlock_pending = false;

            let vid_pid = format_vid_pid(device.vid, device.pid);

            if device.serial_port.is_some() {
                // ZMK Studio device
                self.protocol_type = ProtocolType::Zmk;
                self.json_path = vid_pid;
                self.zmk_serial_port = device.serial_port.clone();
            } else if self.protocol_type != ProtocolType::Zmk {
                // Auto-detect VIAL for HID devices
                let vial_result = connect_protocol(ProtocolType::Vial, &vid_pid);
                if vial_result.is_ok() {
                    self.protocol_type = ProtocolType::Vial;
                    self.json_path = vid_pid;
                } else {
                    self.protocol_type = ProtocolType::Via;
                    self.json_path = String::new();
                }
            } else {
                self.json_path = vid_pid;
            }
            self.error = None;
        }
    }

    fn connect(&mut self) {
        if self.selected_device_index.is_none() {
            self.error = Some("No device selected".to_string());
            return;
        }

        // Build protocol_config from UI fields
        let protocol_config = match self.protocol_type {
            ProtocolType::Vial => self.json_path.trim().to_string(),
            ProtocolType::Via => {
                let path = self.json_path.trim();
                if path.is_empty() {
                    self.error = Some("Please provide a JSON config file path".to_string());
                    return;
                }
                path.to_string()
            }
            ProtocolType::Zmk => {
                let serial_port = match &self.zmk_serial_port {
                    Some(p) => p.clone(),
                    None => {
                        self.error =
                            Some("No serial port detected for this ZMK device".to_string());
                        return;
                    }
                };
                let vid_pid = self.json_path.trim();
                format!("{}|{}", vid_pid, serial_port)
            }
        };

        if self.protocol_type == ProtocolType::Zmk {
            // ZMK Studio flow: try to fetch data via Studio protocol
            let serial_port = self.zmk_serial_port.clone().unwrap();
            match zmk_studio::fetch_studio_data(&serial_port) {
                Ok(studio_data) => {
                    // Unlocked and data fetched — build protocol
                    let (vid, pid) = match parse_vid_pid(self.json_path.trim()) {
                        Ok(vp) => vp,
                        Err(e) => {
                            self.error = Some(format!("Invalid VID:PID: {e}"));
                            return;
                        }
                    };
                    match connect_zmk_studio(vid, pid, studio_data) {
                        Ok(protocol) => {
                            self.current.protocol_type = self.protocol_type;
                            self.current.protocol_config = protocol_config;
                            self.layout_names = protocol.get_layout_definition().get_layout_names();
                            self.connected = true;
                            self.error = None;
                            self.zmk_unlock_pending = false;
                        }
                        Err(e) => {
                            self.error = Some(format!("Failed to connect: {e}"));
                        }
                    }
                }
                Err(e) if e.to_string() == "DEVICE_LOCKED" => {
                    // Device is locked — show unlock prompt
                    self.zmk_unlock_pending = true;
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(format!("ZMK Studio error: {e}"));
                }
            }
        } else {
            // VIA/VIAL: use existing connect flow
            match connect_protocol(self.protocol_type, &protocol_config) {
                Ok(protocol) => {
                    self.current.protocol_type = self.protocol_type;
                    self.current.protocol_config = protocol_config;
                    self.layout_names = protocol.get_layout_definition().get_layout_names();
                    self.connected = true;
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(format!("Failed to connect: {e}"));
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

    fn try_zmk_unlock_and_connect(&mut self) {
        let serial_port = match &self.zmk_serial_port {
            Some(p) => p.clone(),
            None => return,
        };

        // Check if unlocked now
        match zmk_studio::fetch_studio_data(&serial_port) {
            Ok(studio_data) => {
                let (vid, pid) = match parse_vid_pid(self.json_path.trim()) {
                    Ok(vp) => vp,
                    Err(e) => {
                        self.error = Some(format!("Invalid VID:PID: {e}"));
                        self.zmk_unlock_pending = false;
                        return;
                    }
                };
                let protocol_config = format!("{}|{}", self.json_path.trim(), serial_port);
                match connect_zmk_studio(vid, pid, studio_data) {
                    Ok(protocol) => {
                        self.current.protocol_type = self.protocol_type;
                        self.current.protocol_config = protocol_config;
                        self.layout_names = protocol.get_layout_definition().get_layout_names();
                        self.connected = true;
                        self.zmk_unlock_pending = false;
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to connect: {e}"));
                        self.zmk_unlock_pending = false;
                    }
                }
            }
            Err(e) if e.to_string() == "DEVICE_LOCKED" => {
                // Still locked — keep showing prompt
            }
            Err(e) => {
                self.error = Some(format!("ZMK Studio error: {e}"));
                self.zmk_unlock_pending = false;
            }
        }
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
                            // Protocol type selector
                            ui.label("Protocol");
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("protocol_combo")
                                    .width(ui.available_width())
                                    .selected_text(match self.protocol_type {
                                        ProtocolType::Via => "VIA",
                                        ProtocolType::Vial => "VIAL",
                                        ProtocolType::Zmk => "ZMK",
                                    })
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_value(
                                                &mut self.protocol_type,
                                                ProtocolType::Vial,
                                                "VIAL (auto-detect)",
                                            )
                                            .clicked()
                                        {
                                            self.connected = false;
                                            self.layout_names.clear();
                                        }
                                        if ui
                                            .selectable_value(
                                                &mut self.protocol_type,
                                                ProtocolType::Via,
                                                "VIA",
                                            )
                                            .clicked()
                                        {
                                            self.connected = false;
                                            self.layout_names.clear();
                                        }
                                        if ui
                                            .selectable_value(
                                                &mut self.protocol_type,
                                                ProtocolType::Zmk,
                                                "ZMK",
                                            )
                                            .clicked()
                                        {
                                            self.connected = false;
                                            self.layout_names.clear();
                                        }
                                    });
                            });
                            ui.end_row();

                            // Device selector
                            ui.label("Device");
                            ui.horizontal(|ui| {
                                let combo_width = ui.available_width() - 80.0;

                                let selected_text = self
                                    .selected_device_index
                                    .and_then(|i| self.available_devices.get(i))
                                    .map(|d| d.display_name.clone())
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
                                                    device.display_name.clone(),
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
                            let config_label = match self.protocol_type {
                                ProtocolType::Vial => "Device ID",
                                ProtocolType::Via => "JSON Config",
                                ProtocolType::Zmk => "Device ID",
                            };
                            ui.label(config_label);
                            ui.horizontal(|ui| {
                                let input_width = ui.available_width() - 90.0;

                                let input_interactive =
                                    self.protocol_type == ProtocolType::Via && !self.connected;
                                ui.add_sized(
                                    [input_width, 20.0],
                                    egui::TextEdit::singleline(&mut self.json_path)
                                        .hint_text(match self.protocol_type {
                                            ProtocolType::Via => "Path to keyboard info JSON",
                                            _ => "Auto-filled from device",
                                        })
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
                                settings.protocol_config = self.current.protocol_config.clone();
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

        // ZMK unlock popup
        if self.zmk_unlock_pending {
            egui::Window::new("Unlock ZMK Device")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Your ZMK keyboard is locked.");
                    ui.add_space(5.0);
                    ui.label("Press the Studio unlock key combination on your keyboard.");
                    ui.add_space(10.0);
                    ui.spinner();
                    ui.add_space(10.0);
                    if ui.button("Cancel").clicked() {
                        self.zmk_unlock_pending = false;
                    }
                });

            // Poll for unlock every frame (repaint is requested automatically)
            self.try_zmk_unlock_and_connect();
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

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
