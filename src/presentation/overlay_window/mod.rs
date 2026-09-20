use super::OverlayHost;
use crate::device_discovery::DiscoveredDevice;
use crate::settings::SettingsStore;
use crate::ui_wake::UiWake;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;
mod settings_sync;
mod state;
mod ui_overlay;
#[cfg(not(target_arch = "wasm32"))]
mod ui_settings;
#[cfg(target_arch = "wasm32")]
mod web;

use crate::application::connection_manager::{ConnectionEvent, DeviceConnectionManager};
use state::{SettingsState, UiState};

pub struct OverlayApp {
    settings_requested: Arc<AtomicBool>,
    pub(crate) ui: UiState,
    settings: SettingsState,
    settings_store: Arc<dyn SettingsStore>,
    pub(crate) connection_mgr: DeviceConnectionManager,
    pub(crate) editor: crate::keymap_editor::EditorState,
    #[cfg(target_arch = "wasm32")]
    pub(crate) platform: web::WebPlatform,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) platform: desktop::DesktopPlatform,
}

impl OverlayApp {
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
                settings_visible: !cfg!(target_arch = "wasm32"),
                settings_error: None,
                settings_warning: None,
            },
            settings: SettingsState::new(base_settings),
            settings_store,
            connection_mgr: DeviceConnectionManager::new(available_devices, ui_wake),
            editor: crate::keymap_editor::EditorState::new(),
            #[cfg(target_arch = "wasm32")]
            platform: web::WebPlatform::new(),
            #[cfg(not(target_arch = "wasm32"))]
            platform: desktop::DesktopPlatform::new(),
        }
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
        match self.connection_mgr.connect(self.overlay_config()) {
            crate::application::connection_manager::ConnectOutcome::Started => {
                self.ui.settings_error = None;
            }
            crate::application::connection_manager::ConnectOutcome::RequiresLayoutFile => {
                self.pick_layout_file();
            }
            crate::application::connection_manager::ConnectOutcome::AlreadyConnected => {
                self.ui.settings_warning = Some(
                    "Switching device/protocol/layout requires app restart in this version."
                        .to_string(),
                );
            }
            crate::application::connection_manager::ConnectOutcome::Failed(e) => {
                self.ui.settings_error = Some(e);
            }
        }
    }



    /// Requests the editor window to close, initiating a ZMK save first if changes
    /// are pending; otherwise closes immediately.
    #[allow(dead_code)]
    pub(crate) fn request_close_editor(&mut self) {
        if self.editor.request_close() {
            if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
                keyboard.release_edit_lock();
            }
        }
    }

    /// Closes the editor window immediately and releases any open write lock on the
    /// connected keyboard, if one is present.
    pub(crate) fn close_editor(&mut self) {
        self.editor.reset();
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            keyboard.release_edit_lock();
        }
    }

    /// Switches the active keyboard layout, updating preferred layout state and capturing errors.
    pub(crate) fn switch_layout(&mut self, name: &str) {
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            if let Err(e) = keyboard.switch_layout(name) {
                self.ui.settings_error = Some(e);
            } else {
                self.connection_mgr
                    .set_preferred_layout_name(Some(name.to_string()));
            }
        }
    }

    /// Wakes the UI up when the overlay is due to appear or disappear on its own.
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
}

impl OverlayApp {
    /// Backdrop color the host clears to before egui paints: dimmed while either
    /// the settings or keymap editor window is open, otherwise transparent so only
    /// the overlay is visible (or canvas color on web).
    pub fn clear_color(&self) -> egui::Rgba {
        self.platform_clear_color()
    }

    /// A [`KeyPaintStyle`] tuned for the given unit size (pixels per key-unit):
    /// `active.size`-scaled keys on the overlay, miniature ones in pickers.
    pub(crate) fn paint_style(&self, unit: f32) -> crate::key_paint::KeyPaintStyle {
        crate::key_paint::KeyPaintStyle::from_settings(&self.settings.active).with_unit(unit)
    }

    /// Update phase: processes requests, background task completions, dialog updates,
    /// and window passthrough state before any UI rendering occurs.
    fn update(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            keyboard.poll();
        }

        if self.settings_requested.swap(false, Ordering::Relaxed) {
            self.ui.settings_visible = true;
        }

        if let Some(event) = self
            .connection_mgr
            .update(self.overlay_config(), |d| ctx.request_repaint_after(d))
        {
            match event {
                ConnectionEvent::Connected => {
                    self.ui.settings_error = None;
                    self.ui.settings_warning = None;
                    self.sync_visual_settings();
                }
                ConnectionEvent::ConnectionFailed(e) => {
                    self.ui.settings_error = Some(e);
                }
                ConnectionEvent::Disconnected => {
                    self.ui.settings_warning = Some("Device disconnected".into());
                    self.request_close_editor();
                }
            }
        }

        self.update_platform(ctx, host);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn ui(&mut self, ui: &mut egui::Ui, host: &mut dyn OverlayHost) {
        let ctx = ui.ctx().clone();
        self.update(&ctx, host);
        if !self.connection_mgr.is_connected() && self.editor.is_open() {
            self.close_editor();
        }
        self.render_web(ui);
        self.schedule_overlay_repaint(&ctx);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        if !self.connection_mgr.is_connected() && self.editor.is_open() {
            self.close_editor();
        }

        self.render_desktop(ctx, host);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn ui(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        self.update(ctx, host);
        self.render(ctx, host);
        self.schedule_overlay_repaint(ctx);
    }
}
