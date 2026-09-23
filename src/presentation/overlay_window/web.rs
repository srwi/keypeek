//! Web UI chrome, device pairing, and event orchestration.

use std::sync::mpsc;
use std::sync::Arc;
use web_time::Instant;

use super::state::{SettingsState, UiState};
use super::ui_overlay::OverlayView;
use crate::application::connection_manager::{
    ConnectionEvent, ConnectionState, DeviceConnectionManager,
};
use crate::application::Keyboard;
use crate::device_discovery::DiscoveredDevice;
use crate::platform::web::ConnectedWebDevice;
use crate::platform::web_hid::{
    connect_via_with_layout, is_web_hid_supported, request_and_connect_device,
    trigger_web_file_picker, WebConnectOutcome, WebHidTransport,
};
use crate::presentation::OverlayHost;
use crate::protocols::DeviceError;
use crate::settings::{LegendMode, SettingsStore};
use crate::ui_wake::UiWake;
use crate::ui_widgets::{github_link, version_link};

/// Background color for the browser canvas.
pub fn clear_color() -> egui::Rgba {
    egui::Rgba::from_rgb(0.118, 0.118, 0.180)
}

/// Top-level coordinator for browser execution: WebHID/WebSerial pairing,
/// single-canvas layout, responsive sidebar editor, and web local storage.
pub struct WebOverlayApp {
    pub(crate) ui: UiState,
    pub(crate) settings: SettingsState,
    pub(super) settings_store: Arc<dyn SettingsStore>,
    pub(crate) connection_mgr: DeviceConnectionManager,
    pub(crate) editor: crate::keymap_editor::EditorState,

    pub(crate) pairing_rx: Option<mpsc::Receiver<Result<Option<WebConnectOutcome>, DeviceError>>>,
    pub(crate) pending_via: Option<(DiscoveredDevice, Arc<WebHidTransport>)>,
    pub(crate) web_file_rx: Option<mpsc::Receiver<Result<(String, String), String>>>,
    pub(crate) pending_zmk_telemetry: Option<crate::platform::web_hid::PendingZmkTelemetry>,
    pub(crate) telemetry_hid: Option<WebHidTransport>,
    pub(crate) telemetry_rx: Option<mpsc::Receiver<Option<WebHidTransport>>>,
}

pub type WebApp = WebOverlayApp;
pub type OverlayApp = WebOverlayApp;

impl WebOverlayApp {
    pub fn new(ui_wake: UiWake, settings_store: Arc<dyn SettingsStore>) -> Self {
        Self::with_devices(ui_wake, settings_store, Vec::new())
    }

    pub fn with_devices(
        ui_wake: UiWake,
        settings_store: Arc<dyn SettingsStore>,
        available_devices: Vec<DiscoveredDevice>,
    ) -> Self {
        let base_settings = settings_store.load();

        Self {
            ui: UiState {
                settings_visible: false,
                settings_error: None,
                settings_warning: None,
            },
            settings: SettingsState::new(base_settings),
            settings_store,
            connection_mgr: DeviceConnectionManager::new(available_devices, ui_wake),
            editor: crate::keymap_editor::EditorState::new(),
            pairing_rx: None,
            pending_via: None,
            web_file_rx: None,
            pending_zmk_telemetry: None,
            telemetry_hid: None,
            telemetry_rx: None,
        }
    }

    /// Background color for browser mode.
    pub fn clear_color(&self) -> egui::Rgba {
        clear_color()
    }

    pub(crate) fn is_any_window_open(&self) -> bool {
        self.editor.is_open()
    }

    pub(crate) fn persist_settings(&self) {
        if let Err(e) = self.settings_store.save(&self.settings.active) {
            eprintln!("Failed to save settings: {e}");
        }
    }

    /// Closes the editor window and releases any device write lock.
    pub(crate) fn close_editor(&mut self) {
        self.editor
            .close(self.connection_mgr.connected_keyboard().as_deref());
    }

    /// Requests the editor window to close, initiating a ZMK save first if changes
    /// are pending; otherwise closes immediately.
    #[allow(dead_code)]
    pub(crate) fn request_close_editor(&mut self) -> bool {
        self.editor
            .handle_close_request(self.connection_mgr.connected_keyboard().as_deref())
    }

    /// Switches the active keyboard layout and saves the preferred layout name.
    pub(crate) fn switch_layout(&mut self, name: &str) {
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            if let Err(e) = keyboard.switch_layout(name) {
                self.ui.set_error(e);
            } else {
                self.connection_mgr
                    .set_preferred_layout_name(Some(name.to_string()));
            }
        }
    }

    /// Commits modified draft settings to active settings and updates connected keyboard config.
    pub(super) fn sync_visual_settings(&mut self) {
        if self.settings.commit_draft() {
            if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
                keyboard.set_config(self.settings.overlay_config());
            }
            self.persist_settings();
        }
    }

    /// Schedules a repaint when the overlay visibility timer expires.
    fn schedule_overlay_repaint(&self, ctx: &egui::Context) {
        if self.is_any_window_open() {
            return;
        }

        let Some(keyboard) = self.connection_mgr.connected_keyboard() else {
            return;
        };

        if let Some(delay) = keyboard.overlay_changes_in(Instant::now()) {
            ctx.request_repaint_after(delay);
        }
    }

    /// Web update step: polls pairing channels, telemetry channels, and file picker channels.
    pub fn update(&mut self, ctx: &egui::Context, _host: &mut dyn OverlayHost) {
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            keyboard.poll();
        }

        if let Some(event) = self
            .connection_mgr
            .update(self.settings.overlay_config(), |d| {
                ctx.request_repaint_after(d)
            })
        {
            match event {
                ConnectionEvent::Connected => {
                    self.ui.clear_alerts();
                    self.sync_visual_settings();
                }
                ConnectionEvent::ConnectionFailed(e) => {
                    self.ui.set_error(e);
                }
                ConnectionEvent::Disconnected => {
                    self.close_editor();
                }
                ConnectionEvent::DeviceLocked(dev) => {
                    self.ui.set_error(dev.lock_message());
                }
                ConnectionEvent::RequiresLayoutFile(_) => {
                    self.trigger_web_layout_file_picker();
                }
            }
        }

        if let Some(rx) = &self.pairing_rx {
            match rx.try_recv() {
                Ok(Ok(Some(WebConnectOutcome::Connected(connected)))) => {
                    self.handle_connected_web_device(connected);
                    self.pairing_rx = None;
                }
                Ok(Ok(Some(WebConnectOutcome::ZmkConnected {
                    connected,
                    pending_telemetry,
                }))) => {
                    self.handle_connected_web_device(connected);
                    self.pending_zmk_telemetry = pending_telemetry;
                    self.pairing_rx = None;
                }
                Ok(Ok(Some(WebConnectOutcome::RequiresLayoutFile { device, transport }))) => {
                    self.ui.settings_error = None;
                    self.ui.settings_warning = None;
                    self.connection_mgr.set_requires_layout_file(device.clone());
                    self.pending_via = Some((device, transport));
                    self.pairing_rx = None;
                }
                Ok(Ok(None)) => {
                    self.pairing_rx = None;
                    if self.connection_mgr.is_connecting() {
                        self.connection_mgr.disconnect();
                    }
                }
                Ok(Err(err)) => {
                    self.connection_mgr.handle_device_error(&err);
                    self.pairing_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pairing_rx = None;
                    if self.connection_mgr.is_connecting() {
                        self.connection_mgr.disconnect();
                    }
                }
            }
        }

        if let Some(rx) = &self.telemetry_rx {
            match rx.try_recv() {
                Ok(Some(hid)) => {
                    self.telemetry_hid = Some(hid);
                    self.telemetry_rx = None;
                }
                Ok(None) => {
                    self.telemetry_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.telemetry_rx = None;
                }
            }
        }

        if let Some(rx) = &self.web_file_rx {
            match rx.try_recv() {
                Ok(Ok((_filename, content))) => {
                    self.web_file_rx = None;
                    self.connect_pending_via_with_json(content);
                }
                Ok(Err(e)) => {
                    self.ui.settings_error = Some(e);
                    self.web_file_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.web_file_rx = None;
                }
            }
        }
    }

    /// Completes connection when a web device connects.
    pub(super) fn handle_connected_web_device(&mut self, connected: ConnectedWebDevice) {
        let idx = self.connection_mgr.add_device(connected.device.clone());
        self.connection_mgr.select_device(idx);
        self.connection_mgr
            .set_connected(connected.keyboard, connected.profile, None);
        self.ui.settings_error = None;
        self.ui.settings_warning = None;
        self.pending_via = None;
    }

    /// Prompts user to pair a WebHID device.
    pub fn request_web_hid_pairing(&mut self) {
        self.connection_mgr.set_connecting();
        let (tx, rx) = mpsc::channel();
        self.pairing_rx = Some(rx);
        let ui_wake = self.connection_mgr.ui_wake().clone();
        let overlay_config = self.settings.overlay_config();
        wasm_bindgen_futures::spawn_local(async move {
            let result = request_and_connect_device(overlay_config, ui_wake.clone()).await;
            let _ = tx.send(result);
            ui_wake.request_repaint();
        });
    }

    /// Prompts user to connect a ZMK Studio keyboard over Web Serial.
    pub fn request_web_serial_pairing(&mut self) {
        self.connection_mgr.set_connecting();
        let (tx, rx) = mpsc::channel();
        self.pairing_rx = Some(rx);
        let ui_wake = self.connection_mgr.ui_wake().clone();
        let overlay_config = self.settings.overlay_config();
        wasm_bindgen_futures::spawn_local(async move {
            let result = crate::platform::web_serial::request_and_connect_zmk(
                overlay_config,
                ui_wake.clone(),
            )
            .await;
            let _ = tx.send(result);
            ui_wake.request_repaint();
        });
    }

    /// Opens the browser file chooser for VIA layout JSON files.
    pub fn trigger_web_layout_file_picker(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.web_file_rx = Some(rx);
        let ui_wake = self.connection_mgr.ui_wake().clone();
        trigger_web_file_picker(tx, ui_wake);
    }

    /// Connects a pending VIA device with the selected layout JSON.
    pub fn connect_pending_via_with_json(&mut self, json_content: String) {
        let Some((device, transport)) = self.pending_via.take() else {
            return;
        };
        self.connection_mgr.set_connecting();
        let (tx, rx) = mpsc::channel();
        self.pairing_rx = Some(rx);
        let ui_wake = self.connection_mgr.ui_wake().clone();
        let overlay_config = self.settings.overlay_config();
        wasm_bindgen_futures::spawn_local(async move {
            let outcome = connect_via_with_layout(
                device,
                transport,
                json_content,
                overlay_config,
                ui_wake.clone(),
            )
            .await
            .map(|conn| Some(WebConnectOutcome::Connected(conn)));
            let _ = tx.send(outcome);
            ui_wake.request_repaint();
        });
    }

    /// Draws the centered responsive keyboard layout.
    pub fn draw_overlay_canvas(&mut self, ui: &mut egui::Ui, keyboard: &Keyboard) {
        let (raw_w, raw_h) = keyboard.layout().get_dimensions();
        let layout_w = raw_w.max(1.0);
        let layout_h = raw_h.max(1.0);
        const PADDING: f32 = 32.0;
        const MAX_KEY_SIZE: f32 = 75.0;
        const MIN_KEY_SIZE: f32 = 18.0;

        let avail_size = ui.available_size();
        let avail_w = (avail_size.x - PADDING).max(MIN_KEY_SIZE);
        let avail_h = (avail_size.y - PADDING).max(MIN_KEY_SIZE);

        let key_size = (avail_w / layout_w)
            .min(avail_h / layout_h)
            .clamp(MIN_KEY_SIZE, MAX_KEY_SIZE);

        let overlay_h = layout_h * key_size;
        let v_space = ((avail_size.y - overlay_h) * 0.5).max(0.0);

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if v_space > 0.0 {
                    ui.add_space(v_space);
                }

                ui.vertical_centered(|ui| {
                    OverlayView::new(
                        keyboard,
                        &mut self.editor,
                        &self.settings.active,
                        key_size,
                        true,
                    )
                    .show(ui);
                });
            });
    }

    /// Renders the web interface: top bar, editor sidebar, overlay canvas, and modals.
    pub fn render_web(&mut self, ui: &mut egui::Ui) {
        let connected = self.connection_mgr.connected_pair();
        let keyboard_ref = connected.as_ref().map(|(k, _)| k.as_ref());

        // 1. Fixed top panel for connection status and quick settings
        self.render_web_top_bar(ui, keyboard_ref);

        // 2. Resizable left sidebar for keymap editor (if open and connected)
        if let Some((keyboard, profile)) = &connected {
            if self.editor.is_open() {
                let style = self.settings.paint_style(crate::keymap_editor::KEY_UNIT);
                self.editor
                    .draw_sidebar(ui, keyboard, profile.as_ref(), &style);
            }
        }

        // 3. Central panel for the keyboard overlay canvas or connection buttons
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some((keyboard, _)) = &connected {
                self.draw_overlay_canvas(ui, keyboard);
            } else {
                let v_pad = ((ui.available_height() - 80.0) * 0.45).max(20.0);
                ui.add_space(v_pad);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Connect a keyboard to view and edit layers")
                            .size(15.0)
                            .weak(),
                    );
                    ui.add_space(16.0);

                    let id = ui.id().with("connect_buttons_row");
                    let row_width: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(270.0);
                    let left_space = ((ui.available_width() - row_width) * 0.5).max(0.0);

                    ui.horizontal(|ui| {
                        if left_space > 0.0 {
                            ui.add_space(left_space);
                        }

                        let hid_supported = is_web_hid_supported();
                        let serial_supported =
                            crate::platform::web_serial::is_web_serial_supported();

                        let mut hid_btn = ui.add_enabled(
                            hid_supported,
                            egui::Button::new("Connect QMK / Vial"),
                        );
                        if !hid_supported {
                            hid_btn = hid_btn.on_hover_text(
                                "WebHID is not supported in this browser. Please use Chrome, Edge, or Opera.",
                            );
                        } else if hid_btn.clicked() {
                            self.request_web_hid_pairing();
                        }

                        let mut serial_btn = ui.add_enabled(
                            serial_supported,
                            egui::Button::new("Connect ZMK"),
                        );
                        if !serial_supported {
                            serial_btn = serial_btn.on_hover_text(
                                "Web Serial is not supported in this browser. Please use Chrome, Edge, or Opera.",
                            );
                        } else if serial_btn.clicked() {
                            self.request_web_serial_pairing();
                        }

                        let actual_width = serial_btn.rect.max.x - hid_btn.rect.min.x;
                        if (actual_width - row_width).abs() > 1.0 {
                            ui.data_mut(|d| d.insert_temp(id, actual_width));
                        }
                    });
                });
            }
        });

        // 4. Modal prompts
        let ctx = ui.ctx().clone();
        self.render_web_layout_modal(&ctx);
        self.render_zmk_telemetry_modal(&ctx);
    }

    /// Top bar with title, layout switcher, legend mode, and status.
    fn render_web_top_bar(&mut self, ui: &mut egui::Ui, keyboard: Option<&Keyboard>) {
        egui::Panel::top("web_top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("KeyPeek").strong().size(15.0));
                version_link(ui);
                ui.separator();
                github_link(ui);

                // Layout switching (shown when multiple layouts exist).
                if let Some(kbd) = keyboard {
                    if kbd.supports_multiple_layouts() {
                        ui.separator();
                        let current_layout = kbd.active_layout_name();
                        ui.label("Layout:");
                        egui::ComboBox::from_id_salt("web_layout_combo")
                            .selected_text(&current_layout)
                            .show_ui(ui, |ui| {
                                for name in kbd.layout_names() {
                                    let is_selected = name == current_layout;
                                    if ui.selectable_label(is_selected, &name).clicked()
                                        && !is_selected
                                    {
                                        self.switch_layout(&name);
                                    }
                                }
                            });
                    }
                }

                // Controls, connection status, and alerts on the right.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("web_legend_mode")
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
                        });
                    ui.label("Legends:");

                    if let Some(_kbd) = keyboard {
                        ui.separator();
                        let dev_name = self
                            .connection_mgr
                            .selected_device()
                            .map(|d| d.display_name())
                            .unwrap_or_else(|| "Connected".to_string());
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {}",
                                egui_phosphor::regular::KEYBOARD,
                                dev_name
                            ))
                            .color(egui::Color32::LIGHT_GREEN),
                        );
                    }

                    if let ConnectionState::RequiresLayoutFile { device } =
                        self.connection_mgr.state()
                    {
                        if ui
                            .button(egui::RichText::new(format!(
                                "{} Load Layout for {}",
                                egui_phosphor::regular::FOLDER_OPEN,
                                device.base_name
                            )))
                            .clicked()
                        {
                            self.trigger_web_layout_file_picker();
                        }
                    }

                    if let ConnectionState::Locked { device } = self.connection_mgr.state() {
                        let msg = format!(
                            "{} is locked. Unlock it on the keyboard.",
                            device.base_name
                        );
                        if render_alert_chip(ui, &msg, "🔒", egui::Color32::KHAKI) {
                            self.connection_mgr.disconnect();
                        }
                    } else if let ConnectionState::Error(err) = self.connection_mgr.state() {
                        if render_alert_chip(ui, err, "!", egui::Color32::LIGHT_RED) {
                            self.connection_mgr.clear_error();
                        }
                    } else if let Some(err) = &self.ui.settings_error {
                        if render_alert_chip(ui, err, "!", egui::Color32::LIGHT_RED) {
                            self.ui.settings_error = None;
                        }
                    } else if let Some(notice) = &self.ui.settings_warning {
                        if render_alert_chip(ui, notice, "i", egui::Color32::KHAKI) {
                            self.ui.settings_warning = None;
                        }
                    }
                });
            });
        });

        self.sync_visual_settings();
    }

    /// Modal prompt asking for VIA layout JSON.
    fn render_web_layout_modal(&mut self, ctx: &egui::Context) {
        let ConnectionState::RequiresLayoutFile { device } =
            self.connection_mgr.state().clone()
        else {
            return;
        };

        let mut trigger_picker = false;
        let mut dismiss = false;

        egui::Modal::new(egui::Id::new("web_layout_modal")).show(ctx, |ui| {
            ui.set_max_width(400.0);
            ui.heading(format!("Layout Required for {}", device.base_name));
            ui.add_space(8.0);
            ui.label(
                "This keyboard uses the VIA protocol. Please select its layout definition JSON file to complete connection.",
            );

            if let Some(err) = &self.ui.settings_error {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::LIGHT_RED, format!("Error: {err}"));
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new(format!(
                        "{} Select Layout JSON",
                        egui_phosphor::regular::FOLDER_OPEN
                    )))
                    .clicked()
                {
                    trigger_picker = true;
                }

                if ui.button("Cancel").clicked() {
                    dismiss = true;
                }
            });
        });

        if trigger_picker {
            self.trigger_web_layout_file_picker();
        }
        if dismiss {
            self.pending_via = None;
            self.connection_mgr.disconnect();
            self.ui.settings_error = None;
        }
    }

    /// Modal prompt asking to authorize WebHID companion telemetry for ZMK.
    fn render_zmk_telemetry_modal(&mut self, ctx: &egui::Context) {
        if self.pending_zmk_telemetry.is_none() {
            return;
        }

        let mut should_authorize = false;
        let mut should_skip = false;

        egui::Modal::new(egui::Id::new("zmk_telemetry_modal")).show(ctx, |ui| {
            ui.set_max_width(400.0);
            ui.heading("ZMK Connected");
            ui.add_space(8.0);
            ui.label(
                "To enable real-time layer switching and live key highlights, authorize the keyboard's HID telemetry interface.",
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new(format!(
                        "{} Authorize Telemetry",
                        egui_phosphor::regular::BROADCAST
                    )))
                    .clicked()
                {
                    should_authorize = true;
                }
                if ui.button("Skip").clicked() {
                    should_skip = true;
                }
            });
        });

        if should_skip {
            self.pending_zmk_telemetry = None;
        } else if should_authorize {
            let pending = self.pending_zmk_telemetry.take().unwrap();
            let ui_wake = self.connection_mgr.ui_wake().clone();
            let (tx, rx) = mpsc::channel();
            self.telemetry_rx = Some(rx);

            wasm_bindgen_futures::spawn_local(async move {
                let hid = pending.request_and_open(ui_wake.clone()).await;
                let _ = tx.send(hid);
                ui_wake.request_repaint();
            });
        }
    }

    /// Primary entry point called by the web platform runner.
    pub fn ui(&mut self, ui: &mut egui::Ui, host: &mut dyn OverlayHost) {
        let ctx = ui.ctx().clone();
        self.update(&ctx, host);
        if !self.connection_mgr.is_connected() && self.editor.is_open() {
            self.close_editor();
        }
        self.render_web(ui);
        self.schedule_overlay_repaint(&ctx);
    }
}

/// Renders a clickable chip in the top bar to dismiss notifications or errors.
fn render_alert_chip(ui: &mut egui::Ui, text: &str, icon: &str, color: egui::Color32) -> bool {
    ui.separator();
    ui.button(egui::RichText::new(format!("{icon} {text} x")).color(color))
        .on_hover_text("Click to dismiss")
        .clicked()
}
