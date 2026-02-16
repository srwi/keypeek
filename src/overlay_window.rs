use crate::keyboard::Keyboard;
use crate::layout_key::{KeycodeKind, LayoutKey};
use crate::protocols::zmk;
use crate::protocols::zmk_studio;
use crate::protocols::{connect_protocol, format_vid_pid, parse_vid_pid};
use crate::settings::{ProtocolType, Settings, WindowPosition};
use crate::tray::TrayCommand;

use eframe::egui::{self, Align2, Window};
use qmk_via_api::scan::{scan_keyboards, KeyboardDeviceInfo};
use std::sync::mpsc::Receiver;
use std::time::Instant;
use tray_icon::TrayIcon;

const SETTINGS_FILE: &str = "settings.ini";

struct LabelGalleys {
    symbol: Option<std::sync::Arc<egui::Galley>>,
    text: Option<std::sync::Arc<egui::Galley>>,
}

struct DeviceEntry {
    display_name: String,
    vid: u16,
    pid: u16,
    #[allow(dead_code)]
    hid_info: Option<KeyboardDeviceInfo>,
    serial_port: Option<String>,
}

enum AppConnectionState {
    Disconnected,
    Connected { keyboard: Keyboard },
}

pub struct OverlayApp {
    connection_state: AppConnectionState,
    settings_visible: bool,
    settings_error: Option<String>,
    settings_warning: Option<String>,
    ever_connected: bool,
    active_settings: Settings,
    draft_settings: Settings,
    layout_names: Vec<String>,
    available_devices: Vec<DeviceEntry>,
    selected_device_index: Option<usize>,
    protocol_type: ProtocolType,
    json_path: String,
    zmk_serial_port: Option<String>,
    _tray_icon: TrayIcon,
    tray_commands: Receiver<TrayCommand>,
}

impl OverlayApp {
    pub fn new(
        initial_settings: Option<Settings>,
        tray_icon: TrayIcon,
        tray_commands: Receiver<TrayCommand>,
    ) -> Self {
        let base = initial_settings.clone().unwrap_or_default();
        let protocol_type = base.protocol_type;
        let json_path = match base.protocol_type {
            ProtocolType::Via => base.protocol_config.clone(),
            ProtocolType::Vial => base.protocol_config.clone(),
            ProtocolType::Zmk => base
                .protocol_config
                .split('|')
                .next()
                .unwrap_or("")
                .to_string(),
        };
        let zmk_serial_port = if base.protocol_type == ProtocolType::Zmk {
            base.protocol_config
                .split_once('|')
                .map(|(_, p)| p.to_string())
        } else {
            None
        };

        let mut app = Self {
            connection_state: AppConnectionState::Disconnected,
            settings_visible: initial_settings.is_none(),
            settings_error: None,
            settings_warning: None,
            ever_connected: false,
            active_settings: base.clone(),
            draft_settings: base,
            layout_names: Vec::new(),
            available_devices: Vec::new(),
            selected_device_index: None,
            protocol_type,
            json_path,
            zmk_serial_port,
            _tray_icon: tray_icon,
            tray_commands,
        };

        app.refresh_devices();

        if let Some(saved) = initial_settings {
            if let Err(e) = app.connect_with_settings(saved, false) {
                app.settings_visible = true;
                app.settings_error = Some(format!("Failed to connect using saved settings: {e}"));
            }
        }

        app
    }

    fn refresh_devices(&mut self) {
        self.available_devices.clear();
        self.selected_device_index = None;

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

        for sp in zmk_studio::scan_serial_ports() {
            let already_listed = self
                .available_devices
                .iter()
                .any(|d| d.vid == sp.vid && d.pid == sp.pid);

            let display_name = sp
                .product
                .unwrap_or_else(|| format!("{:04X}:{:04X}", sp.vid, sp.pid));

            if already_listed {
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
            self.layout_names.clear();

            let vid_pid = format_vid_pid(device.vid, device.pid);
            if device.serial_port.is_some() {
                self.protocol_type = ProtocolType::Zmk;
                self.json_path = vid_pid;
                self.zmk_serial_port = device.serial_port.clone();
            } else if self.protocol_type != ProtocolType::Zmk {
                let vial_result = connect_protocol(ProtocolType::Vial, &vid_pid);
                if vial_result.is_ok() {
                    self.protocol_type = ProtocolType::Vial;
                    self.json_path = vid_pid;
                } else {
                    self.protocol_type = ProtocolType::Via;
                    self.json_path.clear();
                }
            } else {
                self.json_path = vid_pid;
            }
            self.settings_error = None;
        }
    }

    fn build_protocol_config(&self) -> Result<String, String> {
        match self.protocol_type {
            ProtocolType::Vial => Ok(self.json_path.trim().to_string()),
            ProtocolType::Via => {
                let path = self.json_path.trim();
                if path.is_empty() {
                    Err("Please provide a JSON config file path".to_string())
                } else {
                    Ok(path.to_string())
                }
            }
            ProtocolType::Zmk => {
                let serial_port = self
                    .zmk_serial_port
                    .as_ref()
                    .ok_or_else(|| "No serial port detected for this ZMK device".to_string())?;
                Ok(format!("{}|{}", self.json_path.trim(), serial_port))
            }
        }
    }

    fn active_connection_signature(&self) -> (ProtocolType, String, String) {
        (
            self.active_settings.protocol_type,
            self.active_settings.protocol_config.clone(),
            self.active_settings.layout_name.clone(),
        )
    }

    fn draft_connection_signature(&self) -> Option<(ProtocolType, String, String)> {
        let config = self.build_protocol_config().ok()?;
        Some((
            self.protocol_type,
            config,
            self.draft_settings.layout_name.clone(),
        ))
    }

    fn connection_change_requested(&self) -> bool {
        match self.draft_connection_signature() {
            Some(sig) => sig != self.active_connection_signature(),
            None => false,
        }
    }

    fn connect_with_settings(
        &mut self,
        mut settings: Settings,
        opened_from_ui: bool,
    ) -> Result<(), String> {
        let protocol_config = settings.protocol_config.clone();

        let protocol = if settings.protocol_type == ProtocolType::Zmk {
            let (vid, pid) = parse_vid_pid(
                protocol_config
                    .split('|')
                    .next()
                    .ok_or_else(|| "Invalid ZMK config".to_string())?,
            )
            .map_err(|e| format!("Invalid VID:PID: {e}"))?;

            let serial_port = protocol_config
                .split_once('|')
                .map(|(_, p)| p.to_string())
                .ok_or_else(|| "Missing serial port in ZMK config".to_string())?;

            let studio_data = zmk_studio::fetch_studio_data(&serial_port).map_err(|e| {
                if e.to_string() == "DEVICE_LOCKED" {
                    "Device is locked. Please press the Studio unlock key combination on your keyboard, then click Connect again.".to_string()
                } else {
                    format!("ZMK Studio error: {e}")
                }
            })?;

            self.layout_names = zmk::save_and_get_layout_names(vid, pid, &studio_data)
                .map_err(|e| format!("Failed to process ZMK data: {e}"))?;

            if let Some(first) = self.layout_names.first() {
                if !self.layout_names.contains(&settings.layout_name) {
                    settings.layout_name = first.clone();
                }
            }

            connect_protocol(settings.protocol_type, &settings.protocol_config)
                .map_err(|e| format!("Failed to connect to device: {e}"))?
        } else {
            let protocol = connect_protocol(settings.protocol_type, &settings.protocol_config)
                .map_err(|e| format!("Failed to connect to device: {e}"))?;

            self.layout_names = protocol.get_layout_definition().get_layout_names();
            if let Some(first) = self.layout_names.first() {
                if !self.layout_names.contains(&settings.layout_name) {
                    settings.layout_name = first.clone();
                }
            }
            protocol
        };

        let keyboard = Keyboard::new(protocol, settings.layout_name.clone(), settings.timeout)
            .map_err(|e| format!("Failed to create keyboard: {e}"))?;

        self.active_settings = settings.clone();
        self.draft_settings = settings;
        self.protocol_type = self.active_settings.protocol_type;
        self.connection_state = AppConnectionState::Connected { keyboard };
        self.ever_connected = true;
        self.settings_error = None;
        self.settings_warning = None;

        if opened_from_ui {
            self.settings_visible = true;
        }

        self.persist_settings();
        Ok(())
    }

    fn persist_settings(&self) {
        if self.active_settings.save_settings {
            if let Err(e) = self.active_settings.save_to_file(SETTINGS_FILE) {
                eprintln!("Failed to save settings: {e}");
            }
        }
    }

    fn connect_from_ui(&mut self) {
        if matches!(self.connection_state, AppConnectionState::Connected { .. }) {
            self.settings_warning =
                Some("Switching device/protocol/layout requires app restart in this version."
                    .to_string());
            return;
        }

        if self.selected_device_index.is_none() {
            self.settings_error = Some("No device selected".to_string());
            return;
        }

        let protocol_config = match self.build_protocol_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.settings_error = Some(e);
                return;
            }
        };

        let mut settings = self.draft_settings.clone();
        settings.protocol_type = self.protocol_type;
        settings.protocol_config = protocol_config;

        if let Err(e) = self.connect_with_settings(settings, true) {
            self.settings_error = Some(e);
        }
    }

    fn apply_live_visual_settings(&mut self) {
        if let AppConnectionState::Connected { keyboard } = &self.connection_state {
            let old_timeout = self.active_settings.timeout;

            self.active_settings.size = self.draft_settings.size;
            self.active_settings.margin = self.draft_settings.margin;
            self.active_settings.position = self.draft_settings.position;
            self.active_settings.timeout = self.draft_settings.timeout;
            self.active_settings.save_settings = self.draft_settings.save_settings;

            if old_timeout != self.active_settings.timeout {
                keyboard.set_timeout(self.active_settings.timeout);
            }

            self.persist_settings();
        }
    }

    fn get_anchor_params(&self) -> (Align2, egui::Vec2) {
        match self.active_settings.position {
            WindowPosition::TopLeft => (
                Align2::LEFT_TOP,
                egui::vec2(self.active_settings.margin as f32, self.active_settings.margin as f32),
            ),
            WindowPosition::TopRight => (
                Align2::RIGHT_TOP,
                egui::vec2(
                    -(self.active_settings.margin as f32),
                    self.active_settings.margin as f32,
                ),
            ),
            WindowPosition::BottomLeft => (
                Align2::LEFT_BOTTOM,
                egui::vec2(
                    self.active_settings.margin as f32,
                    -(self.active_settings.margin as f32),
                ),
            ),
            WindowPosition::BottomRight => (
                Align2::RIGHT_BOTTOM,
                egui::vec2(
                    -(self.active_settings.margin as f32),
                    -(self.active_settings.margin as f32),
                ),
            ),
            WindowPosition::Bottom => (
                Align2::CENTER_BOTTOM,
                egui::vec2(0.0, -(self.active_settings.margin as f32)),
            ),
            WindowPosition::Top => (
                Align2::CENTER_TOP,
                egui::vec2(0.0, self.active_settings.margin as f32),
            ),
        }
    }

    fn overlay_visible(&self) -> bool {
        match &self.connection_state {
            AppConnectionState::Disconnected => false,
            AppConnectionState::Connected { keyboard } => {
                if self.settings_visible {
                    true
                } else {
                    match keyboard.time_to_hide_overlay.lock().unwrap().as_ref() {
                        Some(time_to_hide) => Instant::now() < *time_to_hide,
                        None => true,
                    }
                }
            }
        }
    }

    fn draw_settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.settings_visible;
        Window::new("QMK Layout Helper Settings")
            .open(&mut open)
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_size([540.0, 460.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.heading("QMK Layout Helper");
                    ui.hyperlink_to(
                        format!("Version {}", env!("CARGO_PKG_VERSION")),
                        "https://github.com/srwi/qmk-layout-helper",
                    );

                    ui.add_space(16.0);

                    egui::Grid::new("settings_grid")
                        .num_columns(2)
                        .striped(true)
                        .spacing([25.0, 14.0])
                        .show(ui, |ui| {
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
                                        ui.selectable_value(
                                            &mut self.protocol_type,
                                            ProtocolType::Vial,
                                            "VIAL (auto-detect)",
                                        );
                                        ui.selectable_value(
                                            &mut self.protocol_type,
                                            ProtocolType::Via,
                                            "VIA",
                                        );
                                        ui.selectable_value(
                                            &mut self.protocol_type,
                                            ProtocolType::Zmk,
                                            "ZMK",
                                        );
                                    });
                            });
                            ui.end_row();

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
                                        for idx in 0..self.available_devices.len() {
                                            let device = &self.available_devices[idx];
                                            let selected = self.selected_device_index == Some(idx);
                                            if ui
                                                .selectable_label(selected, &device.display_name)
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

                            let config_label = match self.protocol_type {
                                ProtocolType::Vial => "Device ID",
                                ProtocolType::Via => "JSON Config",
                                ProtocolType::Zmk => "Device ID",
                            };
                            ui.label(config_label);
                            ui.horizontal(|ui| {
                                let input_width = ui.available_width() - 110.0;
                                let input_interactive = self.protocol_type == ProtocolType::Via
                                    && !matches!(
                                        self.connection_state,
                                        AppConnectionState::Connected { .. }
                                    );
                                ui.add_sized(
                                    [input_width, 20.0],
                                    egui::TextEdit::singleline(&mut self.json_path)
                                        .hint_text(match self.protocol_type {
                                            ProtocolType::Via => "Path to keyboard info JSON",
                                            _ => "Auto-filled from device",
                                        })
                                        .interactive(input_interactive),
                                );

                                let connected =
                                    matches!(self.connection_state, AppConnectionState::Connected { .. });
                                let can_connect = if connected {
                                    self.connection_change_requested()
                                } else {
                                    self.selected_device_index.is_some()
                                };
                                let button_text = if connected {
                                    if self.connection_change_requested() {
                                        "Apply"
                                    } else {
                                        "Connected"
                                    }
                                } else {
                                    "Connect"
                                };

                                ui.add_enabled_ui(can_connect, |ui| {
                                    if ui
                                        .add_sized([100.0, 20.0], egui::Button::new(button_text))
                                        .clicked()
                                    {
                                        self.connect_from_ui();
                                    }
                                });
                            });
                            ui.end_row();

                            ui.label("Layout");
                            let layout_enabled = !self.layout_names.is_empty();
                            ui.add_enabled_ui(layout_enabled, |ui| {
                                let selected_text = if self.layout_names.is_empty() {
                                    "Connect to device first".to_string()
                                } else {
                                    self.draft_settings.layout_name.clone()
                                };
                                egui::ComboBox::from_id_salt("layout_combo")
                                    .width(ui.available_width())
                                    .selected_text(selected_text)
                                    .show_ui(ui, |ui| {
                                        for name in &self.layout_names {
                                            ui.selectable_value(
                                                &mut self.draft_settings.layout_name,
                                                name.clone(),
                                                name,
                                            );
                                        }
                                    });
                            });
                            ui.end_row();

                            ui.label("Alignment");
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("position_combo")
                                    .width(ui.available_width())
                                    .selected_text(self.draft_settings.position.to_string())
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
                                                &mut self.draft_settings.position,
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
                                egui::DragValue::new(&mut self.draft_settings.margin)
                                    .speed(1)
                                    .suffix(" px"),
                            );
                            ui.end_row();

                            ui.label("Key unit size");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.draft_settings.size)
                                    .speed(1)
                                    .range(20..=1000)
                                    .suffix(" px"),
                            );
                            ui.end_row();

                            ui.label("Display duration");
                            ui.add_sized(
                                ui.available_size(),
                                egui::DragValue::new(&mut self.draft_settings.timeout)
                                    .speed(50)
                                    .range(0..=60_000)
                                    .suffix(" ms"),
                            );
                            ui.end_row();
                        });

                    ui.add_space(18.0);
                    ui.checkbox(&mut self.draft_settings.save_settings, "Remember settings");
                });
            });

        if self.settings_visible && !open {
            self.settings_visible = false;
            if !self.ever_connected {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn generate_key_label_galleys(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        font: egui::FontId,
        color: egui::Color32,
    ) -> LabelGalleys {
        let size = self.active_settings.size as f32;
        let create_galley =
            |text: String, fid: egui::FontId| ui.painter().layout_no_wrap(text, fid, color);
        let fits_width =
            |galley: &std::sync::Arc<egui::Galley>, max: f32| galley.rect.width() <= max;
        let max_width = rect.width() * 0.85;

        if let Some(symbol) = &key.symbol {
            let symbol_font = egui::FontId::proportional(0.33 * size);
            let symbol_galley = create_galley(symbol.clone(), symbol_font);

            if !key.tap.is_empty() {
                let text_galley = create_galley(key.tap.full.clone(), font.clone());
                let gap = 0.06 * size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return LabelGalleys {
                        symbol: Some(symbol_galley),
                        text: Some(text_galley),
                    };
                }
            }

            if let Some(short) = &key.tap.short {
                let text_galley = create_galley(short.clone(), font.clone());
                let gap = 0.06 * size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return LabelGalleys {
                        symbol: Some(symbol_galley),
                        text: Some(text_galley),
                    };
                }
            }

            return LabelGalleys {
                symbol: Some(symbol_galley),
                text: None,
            };
        }

        let full_galley = create_galley(key.tap.full.clone(), font.clone());
        if fits_width(&full_galley, max_width) {
            return LabelGalleys {
                symbol: None,
                text: Some(full_galley),
            };
        }

        let mut truncated = if let Some(short) = &key.tap.short {
            let short_galley = create_galley(short.clone(), font.clone());
            if fits_width(&short_galley, max_width) {
                return LabelGalleys {
                    symbol: None,
                    text: Some(short_galley),
                };
            }
            short.clone()
        } else {
            key.tap.full.clone()
        };

        while truncated.len() > 1 {
            truncated.pop();
            let truncated_with_ellipsis = format!("{}...", truncated);
            let truncated_galley = create_galley(truncated_with_ellipsis, font.clone());
            if fits_width(&truncated_galley, max_width) {
                return LabelGalleys {
                    symbol: None,
                    text: Some(truncated_galley),
                };
            }
        }

        LabelGalleys {
            symbol: None,
            text: None,
        }
    }

    fn get_keycode_color(
        &self,
        layer: u8,
        kind: KeycodeKind,
        desaturate: bool,
        pressed: bool,
    ) -> (egui::Color32, egui::Color32, f32, egui::Color32) {
        const ALPHA: u8 = 239;
        const DESATURATE_FACTOR: f32 = 0.7;

        const BLACK: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, ALPHA);
        const LAYER_0: egui::Color32 = egui::Color32::from_rgba_premultiplied(83, 83, 83, ALPHA);
        const LAYER_1: egui::Color32 = egui::Color32::from_rgba_premultiplied(80, 140, 115, ALPHA);
        const LAYER_2: egui::Color32 = egui::Color32::from_rgba_premultiplied(100, 115, 150, ALPHA);
        const LAYER_3: egui::Color32 = egui::Color32::from_rgba_premultiplied(140, 110, 150, ALPHA);
        const LAYER_4: egui::Color32 = egui::Color32::from_rgba_premultiplied(95, 121, 127, ALPHA);
        const LAYER_5: egui::Color32 = egui::Color32::from_rgba_premultiplied(147, 137, 110, ALPHA);
        const LAYER_N: egui::Color32 = egui::Color32::from_rgba_premultiplied(127, 127, 127, ALPHA);

        let size = self.active_settings.size as f32;
        let mut background_color = match layer {
            0 => LAYER_0,
            1 => LAYER_1,
            2 => LAYER_2,
            3 => LAYER_3,
            4 => LAYER_4,
            5 => LAYER_5,
            _ => LAYER_N,
        };

        if pressed {
            return (
                background_color.lerp_to_gamma(egui::Color32::WHITE, 0.2),
                background_color.lerp_to_gamma(egui::Color32::WHITE, 0.7),
                0.03 * size,
                egui::Color32::WHITE,
            );
        }

        if kind == KeycodeKind::Special {
            background_color = background_color.lerp_to_gamma(BLACK, 0.6);
        } else if kind == KeycodeKind::Modifier {
            background_color = background_color.lerp_to_gamma(BLACK, 0.3);
        }

        let mut border_color = background_color.lerp_to_gamma(BLACK, 0.2);
        if desaturate && layer != 0 {
            background_color = background_color.lerp_to_gamma(LAYER_0, DESATURATE_FACTOR);
            border_color = border_color.lerp_to_gamma(LAYER_0, DESATURATE_FACTOR);
        }

        let font_color = if desaturate {
            egui::Color32::WHITE.gamma_multiply(1.0 - DESATURATE_FACTOR)
        } else {
            egui::Color32::WHITE
        };

        (background_color, border_color, 1.0, font_color)
    }

    fn draw_overlay_window(&self, ctx: &egui::Context, keyboard: &Keyboard) {
        let anchor_params = self.get_anchor_params();
        let mut window_open = true;
        let size = self.active_settings.size as f32;

        Window::new("QMK Layout Helper")
            .open(&mut window_open)
            .auto_sized()
            .anchor(anchor_params.0, anchor_params.1)
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .fade_out(true)
            .title_bar(false)
            .show(ctx, |ui| {
                let layout_size = keyboard.layout.get_dimensions();
                ui.allocate_space(egui::vec2(layout_size.0 * size, layout_size.1 * size));
                let window_pos = ui.min_rect().min;

                for key in &keyboard.layout.keys {
                    let (effective_layer, is_background_key) =
                        keyboard.get_effective_key_layer(key.row, key.col);

                    let layout_key = keyboard
                        .get_key(effective_layer as usize, key.row, key.col)
                        .unwrap_or_default();

                    let first_layer_key_kind = keyboard
                        .get_key(0, key.row, key.col)
                        .map(|k| k.kind)
                        .unwrap_or(KeycodeKind::Basic);

                    let (fill_color, stroke_color, border_thickness, font_color) =
                        self.get_keycode_color(
                            layout_key.layer_ref.unwrap_or(effective_layer),
                            first_layer_key_kind,
                            is_background_key,
                            keyboard.is_key_pressed(key.row, key.col),
                        );

                    let rect = egui::Rect::from_min_size(
                        egui::pos2(key.x * size, key.y * size) + window_pos.to_vec2(),
                        egui::vec2(key.w * size, key.h * size),
                    )
                    .shrink(0.06 * size);

                    ui.painter().rect(
                        rect,
                        0.1 * size,
                        fill_color,
                        egui::Stroke::new(border_thickness, stroke_color),
                        egui::StrokeKind::Outside,
                    );

                    let font = egui::FontId::proportional(0.25 * size);
                    match self.generate_key_label_galleys(ui, &layout_key, rect, font, font_color) {
                        LabelGalleys {
                            symbol: Some(symbol_galley),
                            text: Some(text_galley),
                        } => {
                            let gap = 0.06 * size;
                            let total_width =
                                symbol_galley.rect.width() + gap + text_galley.rect.width();
                            let start_x = rect.center().x - total_width * 0.5;

                            let text_pos_x = start_x + gap + symbol_galley.rect.width();
                            let text_pos = egui::pos2(
                                text_pos_x,
                                rect.center().y - text_galley.rect.center().y,
                            );
                            let sym_pos = egui::pos2(
                                start_x,
                                rect.center().y - symbol_galley.rect.center().y,
                            );
                            ui.painter().galley(sym_pos, symbol_galley, font_color);
                            ui.painter().galley(text_pos, text_galley, font_color);
                        }
                        LabelGalleys {
                            symbol: Some(symbol_galley),
                            text: None,
                        } => {
                            let sym_pos = rect.center() - symbol_galley.rect.center().to_vec2();
                            ui.painter().galley(sym_pos, symbol_galley, font_color);
                        }
                        LabelGalleys {
                            symbol: None,
                            text: Some(text_galley),
                        } => {
                            let label_pos = rect.center() - text_galley.rect.center().to_vec2();
                            ui.painter().galley(label_pos, text_galley, font_color);
                        }
                        _ => {}
                    }
                }
            });
    }

    fn drain_tray_commands(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.tray_commands.try_recv() {
            match cmd {
                TrayCommand::ShowSettings => {
                    self.settings_visible = true;
                }
                TrayCommand::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if self.settings_visible {
            egui::Rgba::from_black_alpha(0.35).to_array()
        } else {
            egui::Rgba::TRANSPARENT.to_array()
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_tray_commands(ctx);
        self.apply_live_visual_settings();

        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(
            !self.settings_visible,
        ));

        if let AppConnectionState::Connected { keyboard } = &self.connection_state {
            if self.overlay_visible() {
                self.draw_overlay_window(ctx, keyboard);
            }
        }

        if self.settings_visible {
            self.draw_settings_window(ctx);
        }

        if let Some(error_message) = self.settings_error.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(error_message);
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.settings_error = None;
                    }
                });
        }

        if let Some(warning_message) = self.settings_warning.clone() {
            egui::Window::new("Notice")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(warning_message);
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.settings_warning = None;
                    }
                });
        }

        ctx.request_repaint();
    }
}
