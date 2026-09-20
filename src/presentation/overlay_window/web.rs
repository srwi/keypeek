//! WebHID-specific UI chrome, pairing flows, and event orchestration.

use std::sync::mpsc;
use std::sync::Arc;

use super::{OverlayApp, OverlayHost};
use crate::device_discovery::DiscoveredDevice;
use crate::platform::web_hid::{
    connect_via_with_layout, is_web_hid_supported, request_and_connect_device,
    trigger_web_file_picker, ConnectedWebDevice, WebConnectOutcome, WebHidTransport,
};
use crate::protocols::DeviceError;

/// Background color for the browser canvas.
pub fn clear_color() -> egui::Rgba {
    egui::Rgba::from_rgb(0.118, 0.118, 0.180)
}

/// Web-specific state holding active async channels and pending VIA connection data.
#[derive(Default)]
pub struct WebPlatform {
    pub(crate) pairing_rx: Option<mpsc::Receiver<Result<Option<WebConnectOutcome>, DeviceError>>>,
    pub(crate) pending_via: Option<(DiscoveredDevice, Arc<WebHidTransport>)>,
    pub(crate) web_file_rx: Option<mpsc::Receiver<Result<(String, String), String>>>,
}

impl WebPlatform {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OverlayApp {
    /// Background color for browser mode.
    pub(super) fn platform_clear_color(&self) -> egui::Rgba {
        clear_color()
    }

    /// Update phase for WebHID: polls pairing and file picker channels.
    pub(super) fn update_platform(&mut self, _ctx: &egui::Context, _host: &mut dyn OverlayHost) {
        if let Some(rx) = &self.platform.pairing_rx {
            match rx.try_recv() {
                Ok(Ok(Some(WebConnectOutcome::Connected(connected)))) => {
                    self.handle_connected_web_device(connected);
                    self.platform.pairing_rx = None;
                }
                Ok(Ok(Some(WebConnectOutcome::RequiresLayoutFile {
                    device,
                    transport,
                }))) => {
                    self.ui.settings_error = None;
                    self.ui.settings_warning = None;
                    self.platform.pending_via = Some((device, transport));
                    self.platform.pairing_rx = None;
                }
                Ok(Ok(None)) => {
                    self.platform.pairing_rx = None;
                }
                Ok(Err(e)) => {
                    self.ui.settings_error = Some(e.to_string());
                    self.platform.pairing_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.platform.pairing_rx = None;
                }
            }
        }

        if let Some(rx) = &self.platform.web_file_rx {
            match rx.try_recv() {
                Ok(Ok((_filename, content))) => {
                    self.platform.web_file_rx = None;
                    self.connect_pending_via_with_json(content);
                }
                Ok(Err(e)) => {
                    self.ui.settings_error = Some(e);
                    self.platform.web_file_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.platform.web_file_rx = None;
                }
            }
        }
    }

    /// WebHID does not use blocking file dialogs; pending layouts use the modal.
    pub(super) fn pick_layout_file(&mut self) {}

    /// Finalizes connection when a WebHID device successfully connects.
    pub(super) fn handle_connected_web_device(&mut self, connected: ConnectedWebDevice) {
        let idx = self.connection_mgr.add_device(connected.device.clone());
        self.connection_mgr.select_device(idx);
        self.connection_mgr.set_connected(
            connected.keyboard,
            connected.profile,
            None,
        );
        self.ui.settings_error = None;
        self.ui.settings_warning = None;
        self.platform.pending_via = None;
    }

    /// Dispatches an async browser HID permission request to pair a device.
    pub fn request_web_hid_pairing(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.platform.pairing_rx = Some(rx);
        let ui_wake = self.connection_mgr.ui_wake().clone();
        let overlay_config = self.overlay_config();
        wasm_bindgen_futures::spawn_local(async move {
            let result = request_and_connect_device(overlay_config, ui_wake.clone()).await;
            let _ = tx.send(result);
            ui_wake.request_repaint();
        });
    }

    /// Triggers browser native file chooser for VIA layout JSON files.
    pub fn trigger_web_layout_file_picker(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.platform.web_file_rx = Some(rx);
        let ui_wake = self.connection_mgr.ui_wake().clone();
        trigger_web_file_picker(tx, ui_wake);
    }

    /// Connects a pending VIA device once its layout JSON content has been selected.
    pub fn connect_pending_via_with_json(&mut self, json_content: String) {
        if let Some((device, transport)) = self.platform.pending_via.clone() {
            let (tx, rx) = mpsc::channel();
            self.platform.pairing_rx = Some(rx);
            let ui_wake = self.connection_mgr.ui_wake().clone();
            let overlay_config = self.overlay_config();
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
    }

    /// Renders the web-specific chrome toolbar and modal dialogs.
    pub(super) fn render_platform(
        &mut self,
        ctx: &egui::Context,
        keyboard: Option<&crate::application::Keyboard>,
    ) {
        self.render_web_chrome(ctx, keyboard);
        self.render_web_layout_modal(ctx);
    }

    /// Floating top-right toolbar for browser mode: Connect button and Settings toggle.
    fn render_web_chrome(
        &mut self,
        ctx: &egui::Context,
        _keyboard: Option<&crate::application::Keyboard>,
    ) {
        egui::Area::new(egui::Id::new("web_toolbar"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 16.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let supported = is_web_hid_supported();
                    let is_connected = self.connection_mgr.is_connected();
                    let btn_text = if is_connected {
                        format!("{} Switch Keyboard", egui_phosphor::regular::PLUG)
                    } else {
                        format!("{} Connect Keyboard", egui_phosphor::regular::PLUG)
                    };
                    let btn = ui.add_enabled(
                        supported,
                        egui::Button::new(egui::RichText::new(btn_text)),
                    );
                    if !supported {
                        btn.on_hover_text(
                            "WebHID is not supported in this browser. Please use Chrome or Edge.",
                        );
                    } else if btn.clicked() {
                        self.request_web_hid_pairing();
                    }

                    if let Some((device, _)) = &self.platform.pending_via {
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

                    if ui
                        .button(egui::RichText::new(format!(
                            "{} Settings",
                            egui_phosphor::regular::GEAR
                        )))
                        .clicked()
                    {
                        self.ui.settings_visible = !self.ui.settings_visible;
                    }
                });
            });
    }

    /// Centered modal asking for layout JSON when a VIA keyboard is connected.
    fn render_web_layout_modal(&mut self, ctx: &egui::Context) {
        if let Some((device, _)) = &self.platform.pending_via {
            let mut trigger_picker = false;
            let mut dismiss = false;

            egui::Window::new(format!("Layout Required — {}", device.base_name))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_max_width(440.0);
                    ui.vertical(|ui| {
                        ui.label(
                            "This keyboard uses the VIA protocol and does not store its layout \
                             on the device.",
                        );
                        ui.label(
                            "Please select its layout definition JSON file (VIA JSON or QMK info.json) \
                             to complete the connection.",
                        );

                        if let Some(err) = &self.ui.settings_error {
                            ui.add_space(4.0);
                            ui.colored_label(egui::Color32::LIGHT_RED, format!("Error: {err}"));
                        }

                        ui.add_space(10.0);
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

                        ui.add_space(6.0);
                        ui.weak("Tip: Once loaded, this layout will be remembered for this keyboard.");
                    });
                });

            if trigger_picker {
                self.trigger_web_layout_file_picker();
            }
            if dismiss {
                self.platform.pending_via = None;
                self.ui.settings_error = None;
            }
        }
    }
}
