//! Desktop application coordinator, window orchestration, and native integration.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use web_time::Instant;

use super::state::{SettingsState, UiState};
use super::ui_overlay::OverlayView;
use crate::overlay_host::OverlayHost;
use crate::settings::SettingsStore;
use keypeek_protocol::connection_manager::{
    ConnectOutcome, ConnectionEvent, DeviceConnectionManager,
};
use keypeek_protocol::{DiscoveredDevice, Keyboard, UiWake};

/// Background clear color for desktop: transparent overlay or dimmed backdrop.
pub fn clear_color(is_any_window_open: bool) -> egui::Rgba {
    if is_any_window_open {
        egui::Rgba::from_black_alpha(0.65)
    } else {
        egui::Rgba::TRANSPARENT
    }
}

/// Top-level coordinator for desktop execution: multi-windowing, system tray,
/// mouse passthrough, and native file dialogs.
pub struct DesktopOverlayApp {
    pub(super) settings_requested: Arc<AtomicBool>,
    pub(crate) ui: UiState,
    pub(super) settings: SettingsState,
    pub(super) settings_store: Arc<dyn SettingsStore>,
    pub(crate) connection_mgr: DeviceConnectionManager,
    pub(crate) editor: crate::keymap_editor::EditorState,
    pub(super) file_dialog: egui_file_dialog::FileDialog,
    pub(super) mouse_passthrough: Option<bool>,
}

pub type DesktopApp = DesktopOverlayApp;
pub type OverlayApp = DesktopOverlayApp;

impl DesktopOverlayApp {
    pub fn new(
        settings_requested: Arc<AtomicBool>,
        ui_wake: UiWake,
        settings_store: Arc<dyn SettingsStore>,
        available_devices: Vec<DiscoveredDevice>,
    ) -> Self {
        let base_settings = settings_store.load();

        Self {
            settings_requested,
            ui: UiState {
                settings_visible: true,
                settings_error: None,
                settings_warning: None,
            },
            settings: SettingsState::new(base_settings),
            settings_store,
            connection_mgr: DeviceConnectionManager::new(available_devices, ui_wake),
            editor: crate::keymap_editor::EditorState::new(),
            file_dialog: egui_file_dialog::FileDialog::new(),
            mouse_passthrough: None,
        }
    }

    /// Background color before egui paints.
    pub fn clear_color(&self) -> egui::Rgba {
        clear_color(self.is_any_window_open())
    }

    pub(crate) fn is_any_window_open(&self) -> bool {
        self.ui.settings_visible || self.editor.is_open()
    }

    pub(crate) fn persist_settings(&self) {
        if let Err(e) = self.settings_store.save(&self.settings.active) {
            eprintln!("Failed to save settings: {e}");
        }
    }

    /// Initiates connection for the currently selected device in connection_mgr.
    pub fn connect_from_ui(&mut self) {
        match self.connection_mgr.connect(self.settings.overlay_config()) {
            ConnectOutcome::Started => {
                self.ui.settings_error = None;
            }
            ConnectOutcome::RequiresLayoutFile => {
                self.pick_layout_file();
            }
            ConnectOutcome::AlreadyConnected => {
                self.ui.set_warning(
                    "Switching device/protocol/layout requires app restart in this version.",
                );
            }
            ConnectOutcome::Failed(e) => {
                self.ui.set_error(e);
            }
        }
    }

    /// Requests the editor window to close, initiating a ZMK save first if changes
    /// are pending; otherwise closes immediately.
    #[allow(dead_code)]
    pub(crate) fn request_close_editor(&mut self) -> bool {
        self.editor
            .handle_close_request(self.connection_mgr.connected_keyboard().as_deref())
    }

    /// Closes the editor window and releases any device write lock.
    pub(crate) fn close_editor(&mut self) {
        self.editor
            .close(self.connection_mgr.connected_keyboard().as_deref());
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
        }
    }

    pub(super) fn get_anchor_params(&self) -> (egui::Align2, egui::Vec2) {
        self.settings
            .active
            .position
            .anchor_and_offset(self.settings.active.margin as f32)
    }

    pub(super) fn overlay_visible(&self) -> bool {
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            self.is_any_window_open() || keyboard.overlay_is_visible(Instant::now())
        } else {
            false
        }
    }

    /// Opens the native file dialog to pick a layout file.
    pub(super) fn pick_layout_file(&mut self) {
        self.file_dialog.pick_file();
    }

    /// Sets OS mouse passthrough mode based on current window state.
    pub(super) fn sync_mouse_passthrough(&mut self, host: &mut dyn OverlayHost) {
        let mouse_passthrough = !self.is_any_window_open();
        if self.mouse_passthrough == Some(mouse_passthrough) {
            return;
        }

        host.set_passthrough(mouse_passthrough);
        self.mouse_passthrough = Some(mouse_passthrough);
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

    /// Updates state, handles background tasks, and processes input before drawing.
    pub fn update(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            keyboard.poll();
        }

        if self.settings_requested.swap(false, Ordering::Relaxed) {
            self.ui.settings_visible = true;
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
                    self.pick_layout_file();
                }
            }
        }

        self.file_dialog.update(ctx);

        if let Some(path) = self.file_dialog.take_picked() {
            self.connection_mgr
                .set_layout_file_path(path.to_string_lossy().to_string());
            self.connect_from_ui();
        }

        self.sync_mouse_passthrough(host);
    }

    /// Shows a modal dialog with `message` and an OK button that clears `slot`.
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

    /// Renders the overlay in an egui Window.
    pub fn draw_overlay_window(&mut self, ctx: &egui::Context, keyboard: &Keyboard, visible: bool) {
        let anchor_params = self.get_anchor_params();
        let mut window_open = visible;
        let hit_test_enabled = self.is_any_window_open();
        let size = self.settings.active.size as f32;

        egui::Window::new("KeyPeek")
            .open(&mut window_open)
            .auto_sized()
            .interactable(hit_test_enabled)
            .anchor(anchor_params.0, anchor_params.1)
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .fade_out(true)
            .title_bar(false)
            .show(ctx, |ui| {
                OverlayView::new(
                    keyboard,
                    &mut self.editor,
                    &self.settings.active,
                    size,
                    hit_test_enabled,
                )
                .show(ui);
            });
    }

    /// Renders desktop windows: overlay, key editor, settings, and notices.
    pub fn render(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        if !self.connection_mgr.is_connected() && self.editor.is_open() {
            self.close_editor();
        }

        let connected = self.connection_mgr.connected_pair();
        if let Some((keyboard, profile)) = &connected {
            self.draw_overlay_window(ctx, keyboard, self.overlay_visible());
            if self.editor.is_open() {
                let style = self.settings.paint_style(crate::keymap_editor::KEY_UNIT);
                self.editor
                    .draw_window(ctx, keyboard, profile.as_ref(), &style);
            }
        }

        if self.ui.settings_visible {
            self.draw_settings_window(ctx, host);
        }

        Self::message_window(ctx, "Error", &mut self.ui.settings_error);
        Self::message_window(ctx, "Notice", &mut self.ui.settings_warning);
    }

    /// Primary entry point called by the desktop platform runner.
    pub fn ui(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        self.update(ctx, host);
        self.render(ctx, host);
        self.schedule_overlay_repaint(ctx);
    }
}
