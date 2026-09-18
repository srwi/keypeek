use crate::device_discovery::DiscoveredDevice;
use crate::platform::OverlayHost;
use crate::settings::Settings;
use crate::ui_wake::UiWake;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub mod connection_manager;
mod settings_sync;
mod state;
mod ui_overlay;
mod ui_settings;

use connection_manager::{ConnectOutcome, ConnectionEvent, DeviceConnectionManager};
use state::{SettingsState, UiState};

pub struct OverlayApp {
    _tray: crate::tray::Tray,
    settings_requested: Arc<AtomicBool>,
    pub(crate) ui: UiState,
    settings: SettingsState,
    pub(crate) connection_mgr: DeviceConnectionManager,
    pub(crate) editor: crate::keymap_editor::EditorState,
}

impl OverlayApp {
    pub fn new(
        tray: crate::tray::Tray,
        settings_requested: Arc<AtomicBool>,
        ui_wake: UiWake,
        base_settings: Settings,
        available_devices: Vec<DiscoveredDevice>,
    ) -> Self {
        Self {
            _tray: tray,
            settings_requested,
            ui: UiState {
                settings_visible: true,
                settings_error: None,
                settings_warning: None,
                mouse_passthrough: None,
                file_dialog: egui_file_dialog::FileDialog::new(),
            },
            settings: SettingsState::new(base_settings),
            connection_mgr: DeviceConnectionManager::new(available_devices, ui_wake),
            editor: crate::keymap_editor::EditorState::new(),
        }
    }

    pub(crate) fn is_any_window_open(&self) -> bool {
        self.ui.settings_visible || self.editor.is_open()
    }

    fn sync_mouse_passthrough(&mut self, host: &mut dyn OverlayHost) {
        let mouse_passthrough = !self.is_any_window_open();
        if self.ui.mouse_passthrough == Some(mouse_passthrough) {
            return;
        }

        host.set_passthrough(mouse_passthrough);
        self.ui.mouse_passthrough = Some(mouse_passthrough);
    }

    pub(crate) fn persist_settings(&self) {
        if let Err(e) = self.settings.active.save() {
            eprintln!("Failed to save settings: {e}");
        }
    }

    pub(super) fn connect_from_ui(&mut self) {
        match self.connection_mgr.connect(self.overlay_config()) {
            ConnectOutcome::Started => {
                self.ui.settings_error = None;
            }
            ConnectOutcome::RequiresLayoutFile => {
                self.ui.file_dialog.pick_file();
            }
            ConnectOutcome::AlreadyConnected => {
                self.ui.settings_warning = Some(
                    "Switching device/protocol/layout requires app restart in this version."
                        .to_string(),
                );
            }
            ConnectOutcome::Failed(e) => {
                self.ui.settings_error = Some(e);
            }
        }
    }

    /// Draw a centered modal with `message` and an OK button that clears `slot`.
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

    /// Requests the editor window to close, initiating a ZMK save first if changes
    /// are pending; otherwise closes immediately.
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
    /// the overlay is visible.
    pub fn clear_color(&self) -> egui::Rgba {
        if self.is_any_window_open() {
            egui::Rgba::from_black_alpha(0.65)
        } else {
            egui::Rgba::TRANSPARENT
        }
    }

    /// A [`KeyPaintStyle`] tuned for the given unit size (pixels per key-unit):
    /// `active.size`-scaled keys on the overlay, miniature ones in pickers.
    pub(crate) fn paint_style(&self, unit: f32) -> crate::key_paint::KeyPaintStyle {
        crate::key_paint::KeyPaintStyle::from_settings(&self.settings.active).with_unit(unit)
    }

    /// Update phase: processes requests, background task completions, dialog updates,
    /// and window passthrough state before any UI rendering occurs.
    fn update(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        if self.settings_requested.swap(false, Ordering::Relaxed) {
            self.ui.settings_visible = true;
        }

        if let Some(event) = self.connection_mgr.update(self.overlay_config(), ctx) {
            match event {
                ConnectionEvent::Connected => {
                    self.ui.settings_error = None;
                    self.ui.settings_warning = None;
                    self.persist_settings();
                }
                ConnectionEvent::ConnectionFailed(e) => {
                    self.ui.settings_error = Some(e);
                }
                ConnectionEvent::Disconnected => {
                    self.close_editor();
                }
            }
        }

        self.ui.file_dialog.update(ctx);

        if let Some(path) = self.ui.file_dialog.take_picked() {
            self.connection_mgr
                .set_layout_file_path(path.to_string_lossy().to_string());
            self.connect_from_ui();
        }

        self.sync_mouse_passthrough(host);
    }

    /// Render phase: paints visible overlay, editor, settings, and modal dialogs.
    fn render(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        if let Some((keyboard, profile)) = self.connection_mgr.connected_pair() {
            // Clone the shared keyboard so drawing can mutate app state (the
            // editor) without holding a borrow on `self.connection_mgr`.
            self.draw_overlay_window(ctx, &keyboard, self.overlay_visible());
            if self.editor.is_open() {
                let style = self.paint_style(crate::keymap_editor::KEY_UNIT);
                self.editor
                    .draw_window(ctx, &keyboard, profile.as_ref(), &style);
            }
        } else if self.editor.is_open() {
            // The connection dropped; close the editor. Unsaved ZMK changes
            // died with the connection, so the dirty flag goes too.
            self.close_editor();
        }

        if self.ui.settings_visible {
            self.draw_settings_window(ctx, host);
        }

        Self::message_window(ctx, "Error", &mut self.ui.settings_error);
        Self::message_window(ctx, "Notice", &mut self.ui.settings_warning);
    }

    pub fn ui(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        self.update(ctx, host);
        self.render(ctx, host);
        self.schedule_overlay_repaint(ctx);
    }
}
