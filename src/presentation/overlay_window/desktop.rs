//! Desktop-specific window orchestration, native file picker, and platform setup.

use super::{OverlayApp, OverlayHost};

/// Background clear color for desktop: transparent overlay or dimmed modal backdrop.
pub fn clear_color(is_any_window_open: bool) -> egui::Rgba {
    if is_any_window_open {
        egui::Rgba::from_black_alpha(0.65)
    } else {
        egui::Rgba::TRANSPARENT
    }
}

/// Desktop-specific state holding native file picker and window passthrough state.
pub struct DesktopPlatform {
    pub(crate) file_dialog: egui_file_dialog::FileDialog,
    pub(crate) mouse_passthrough: Option<bool>,
}

impl Default for DesktopPlatform {
    fn default() -> Self {
        Self {
            file_dialog: egui_file_dialog::FileDialog::new(),
            mouse_passthrough: None,
        }
    }
}

impl DesktopPlatform {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OverlayApp {
    /// Background clear color for desktop: transparent overlay or dimmed modal backdrop.
    pub(super) fn platform_clear_color(&self) -> egui::Rgba {
        clear_color(self.is_any_window_open())
    }

    /// Update phase for desktop: updates file dialog, handles picked layout files, and syncs passthrough.
    pub(super) fn update_platform(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        self.platform.file_dialog.update(ctx);

        if let Some(path) = self.platform.file_dialog.take_picked() {
            self.connection_mgr
                .set_layout_file_path(path.to_string_lossy().to_string());
            self.connect_from_ui();
        }

        self.sync_mouse_passthrough(host);
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

    /// Renders the complete desktop interface: overlay window, key editor window,
    /// settings window, and notification dialogs.
    pub(super) fn render_desktop(&mut self, ctx: &egui::Context, host: &mut dyn OverlayHost) {
        let connected = self.connection_mgr.connected_pair();
        if let Some((keyboard, profile)) = &connected {
            self.draw_overlay_window(ctx, keyboard, self.overlay_visible());
            if self.editor.is_open() {
                let style = self.paint_style(crate::keymap_editor::KEY_UNIT);
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

    /// Triggers the native file dialog to pick a layout file.
    pub(super) fn pick_layout_file(&mut self) {
        self.platform.file_dialog.pick_file();
    }

    /// Synchronizes native OS mouse passthrough mode with the current window state.
    pub(super) fn sync_mouse_passthrough(&mut self, host: &mut dyn OverlayHost) {
        let mouse_passthrough = !self.is_any_window_open();
        if self.platform.mouse_passthrough == Some(mouse_passthrough) {
            return;
        }

        host.set_passthrough(mouse_passthrough);
        self.platform.mouse_passthrough = Some(mouse_passthrough);
    }
}
