use super::state::AppConnectionState;
use super::OverlayApp;
use crate::platform::OverlayHost;
use crate::settings::{ProtocolType, WindowPosition};
use egui::Align2;
use std::time::Instant;

impl OverlayApp {
    pub(super) fn apply_live_visual_settings(&mut self) {
        if self.settings.active == self.settings.draft {
            return;
        }

        let old_timeout = self.settings.active.timeout;
        self.settings.active = self.settings.draft.clone();

        if let AppConnectionState::Connected { keyboard } = &self.session.connection {
            if old_timeout != self.settings.active.timeout {
                keyboard.set_timeout(self.settings.active.timeout);
            }
        }

        self.persist_settings();
    }

    pub(super) fn apply_live_layout_settings(&mut self) {
        if self.session.active_layout_name == self.session.draft_layout_name {
            return;
        }

        if !matches!(
            self.connect.draft.protocol_type(),
            ProtocolType::Via | ProtocolType::Vial
        ) {
            self.session.draft_layout_name = self.session.active_layout_name.clone();
            return;
        }

        let Some(definition) = self.session.connected_definition.as_ref() else {
            self.ui.settings_error =
                Some("Missing keyboard definition for live layout switch".to_string());
            self.session.draft_layout_name = self.session.active_layout_name.clone();
            return;
        };

        let selected_layout = self.session.draft_layout_name.clone();
        let next_layout = match definition.get_layout(&selected_layout) {
            Ok(layout) => layout,
            Err(e) => {
                self.ui.settings_error = Some(format!("Failed to switch layout: {e}"));
                self.session.draft_layout_name = self.session.active_layout_name.clone();
                return;
            }
        };

        let AppConnectionState::Connected { keyboard } = &mut self.session.connection else {
            return;
        };

        keyboard.set_layout(next_layout);
        self.session.active_layout_name = selected_layout;
    }

    pub(super) fn get_anchor_params(&self) -> (Align2, egui::Vec2) {
        match self.settings.active.position {
            WindowPosition::TopLeft => (
                Align2::LEFT_TOP,
                egui::vec2(
                    self.settings.active.margin as f32,
                    self.settings.active.margin as f32,
                ),
            ),
            WindowPosition::TopRight => (
                Align2::RIGHT_TOP,
                egui::vec2(
                    -(self.settings.active.margin as f32),
                    self.settings.active.margin as f32,
                ),
            ),
            WindowPosition::BottomLeft => (
                Align2::LEFT_BOTTOM,
                egui::vec2(
                    self.settings.active.margin as f32,
                    -(self.settings.active.margin as f32),
                ),
            ),
            WindowPosition::BottomRight => (
                Align2::RIGHT_BOTTOM,
                egui::vec2(
                    -(self.settings.active.margin as f32),
                    -(self.settings.active.margin as f32),
                ),
            ),
            WindowPosition::Bottom => (
                Align2::CENTER_BOTTOM,
                egui::vec2(0.0, -(self.settings.active.margin as f32)),
            ),
            WindowPosition::Top => (
                Align2::CENTER_TOP,
                egui::vec2(0.0, self.settings.active.margin as f32),
            ),
        }
    }

    pub(super) fn overlay_visible(&self) -> bool {
        match &self.session.connection {
            AppConnectionState::Disconnected | AppConnectionState::Reconnecting { .. } => false,
            AppConnectionState::Connected { keyboard } => {
                if self.ui.settings_visible {
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

    /// Ensure the draft screen points to an available screen, falling back to the
    /// current screen (or the first available one) when the saved value is stale.
    pub(super) fn ensure_valid_draft_screen(&mut self, host: &dyn OverlayHost) {
        let screens = host.available_screens();
        if screens.is_empty() {
            return;
        }
        if screens
            .iter()
            .any(|screen| screen.id == self.settings.draft.screen)
        {
            return;
        }
        self.settings.draft.screen = host
            .current_screen()
            .filter(|id| screens.iter().any(|screen| screen.id == *id))
            .unwrap_or_else(|| screens[0].id.clone());
    }

    pub(super) fn apply_live_screen_setting(
        &mut self,
        ctx: &egui::Context,
        host: &mut dyn OverlayHost,
    ) {
        self.ensure_valid_draft_screen(host);

        let screens = host.available_screens();
        if screens.is_empty() {
            // The windowing system hasn't reported screens yet; retry next frame.
            return;
        }

        let screen_changed = self.settings.active.screen != self.settings.draft.screen;
        if screen_changed || !self.ui.screen_applied {
            host.move_to_screen(&self.settings.draft.screen);
            // The overlay egui Window is positioned absolutely in screen space;
            // after moving the native window we must reset its cached area so it
            // re-anchors to the new viewport instead of staying on the old monitor.
            ctx.memory_mut(|mem| mem.reset_areas());
            self.settings.active.screen = self.settings.draft.screen.clone();
            self.ui.screen_applied = true;
            self.persist_settings();
        }
    }
}
