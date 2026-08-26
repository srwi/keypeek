use super::state::AppConnectionState;
use super::OverlayApp;
use crate::keyboard::OverlayConfig;
use crate::settings::{ProtocolType, WindowPosition};
use egui::Align2;
use std::sync::Arc;
use std::time::Instant;

impl OverlayApp {
    /// The active settings as the overlay timing values `Keyboard` runs on.
    pub(super) fn overlay_config(&self) -> OverlayConfig {
        OverlayConfig {
            timeout_ms: self.settings.active.timeout,
            activation_delay_ms: self.settings.active.activation_delay,
            visible_layers: self.settings.active.visible_layers.bits(),
        }
    }

    pub(super) fn apply_live_visual_settings(&mut self) {
        if self.settings.active == self.settings.draft {
            return;
        }

        self.settings.active = self.settings.draft.clone();

        if let AppConnectionState::Connected { keyboard } = &self.session.connection {
            keyboard.set_config(self.overlay_config());
        }
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

        if let Some(keyboard) = Arc::get_mut(keyboard) {
            keyboard.set_layout(next_layout);
        }
        self.session.active_layout_name = selected_layout;
    }

    pub(super) fn get_anchor_params(&self) -> (Align2, egui::Vec2) {
        use WindowPosition::*;
        let m = self.settings.active.margin as f32;
        let (align, dx, dy) = match self.settings.active.position {
            TopLeft => (Align2::LEFT_TOP, m, m),
            TopRight => (Align2::RIGHT_TOP, -m, m),
            BottomLeft => (Align2::LEFT_BOTTOM, m, -m),
            BottomRight => (Align2::RIGHT_BOTTOM, -m, -m),
            Bottom => (Align2::CENTER_BOTTOM, 0.0, -m),
            Top => (Align2::CENTER_TOP, 0.0, m),
        };
        (align, egui::vec2(dx, dy))
    }

    pub(super) fn overlay_visible(&self) -> bool {
        match &self.session.connection {
            AppConnectionState::Disconnected | AppConnectionState::Reconnecting { .. } => false,
            AppConnectionState::Connected { keyboard } => {
                self.ui.settings_visible || keyboard.overlay_is_visible(Instant::now())
            }
        }
    }
}
