use super::state::AppConnectionState;
use super::OverlayApp;
use crate::settings::{ProtocolType, WindowPosition};
use eframe::egui::{self, Align2};
use std::time::Instant;

impl OverlayApp {
    pub(super) fn apply_live_visual_settings(&mut self) {
        let old_timeout = self.settings.active.timeout;
        let changed = self.settings.active.size != self.settings.draft.size
            || self.settings.active.margin != self.settings.draft.margin
            || self.settings.active.position != self.settings.draft.position
            || self.settings.active.timeout != self.settings.draft.timeout
            || self.settings.active.theme != self.settings.draft.theme
            || self.settings.active.save_settings != self.settings.draft.save_settings;

        if !changed {
            return;
        }

        self.settings.active.size = self.settings.draft.size;
        self.settings.active.margin = self.settings.draft.margin;
        self.settings.active.position = self.settings.draft.position;
        self.settings.active.timeout = self.settings.draft.timeout;
        self.settings.active.theme = self.settings.draft.theme.clone();
        self.settings.active.save_settings = self.settings.draft.save_settings;

        if let AppConnectionState::Connected { keyboard } = &self.session.connection {
            if old_timeout != self.settings.active.timeout {
                keyboard.set_timeout(self.settings.active.timeout);
            }
        }

        self.persist_settings();
    }

    pub(super) fn apply_live_layout_settings(&mut self) {
        if self.settings.active.layout_name == self.settings.draft.layout_name {
            return;
        }

        if !matches!(
            self.connect.protocol_type,
            ProtocolType::Via | ProtocolType::Vial
        ) {
            self.settings.draft.layout_name = self.settings.active.layout_name.clone();
            return;
        }

        let Some(definition) = self.session.connected_definition.as_ref() else {
            self.ui.settings_error =
                Some("Missing keyboard definition for live layout switch".to_string());
            self.settings.draft.layout_name = self.settings.active.layout_name.clone();
            return;
        };

        let selected_layout = self.settings.draft.layout_name.clone();
        let next_layout = match definition.get_layout(&selected_layout) {
            Ok(layout) => layout,
            Err(e) => {
                self.ui.settings_error = Some(format!("Failed to switch layout: {e}"));
                self.settings.draft.layout_name = self.settings.active.layout_name.clone();
                return;
            }
        };

        let AppConnectionState::Connected { keyboard } = &mut self.session.connection else {
            return;
        };

        keyboard.set_layout(next_layout);
        self.settings.active.layout_name = selected_layout;
        self.persist_settings();
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
            AppConnectionState::Disconnected => false,
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
}
