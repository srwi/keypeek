use super::OverlayApp;
use crate::domain::visibility::OverlayConfig;
use egui::Align2;

impl OverlayApp {
    /// The active settings as the overlay timing values `Keyboard` runs on.
    pub(super) fn overlay_config(&self) -> OverlayConfig {
        OverlayConfig {
            timeout_ms: self.settings.active.timeout,
            activation_delay_ms: self.settings.active.activation_delay,
            visible_layers: self.settings.active.visible_layers.bits(),
        }
    }

    /// Commits modified draft settings to active settings and updates connected keyboard config.
    pub(super) fn sync_visual_settings(&mut self) {
        if self.settings.commit_draft() {
            if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
                keyboard.set_config(self.overlay_config());
            }
        }
    }

    pub(super) fn get_anchor_params(&self) -> (Align2, egui::Vec2) {
        self.settings
            .active
            .position
            .anchor_and_offset(self.settings.active.margin as f32)
    }

    pub(super) fn overlay_visible(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            self.connection_mgr.connected_keyboard().is_some()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use web_time::Instant;
            if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
                self.is_any_window_open() || keyboard.overlay_is_visible(Instant::now())
            } else {
                false
            }
        }
    }
}
