use crate::key_paint::KeyPaintStyle;
use crate::settings::Settings;

pub struct UiState {
    pub settings_visible: bool,
    pub settings_error: Option<String>,
    pub settings_warning: Option<String>,
}

impl UiState {
    pub fn clear_alerts(&mut self) {
        self.settings_error = None;
        self.settings_warning = None;
    }

    pub fn set_error(&mut self, err: impl Into<String>) {
        self.settings_error = Some(err.into());
    }

    pub fn set_warning(&mut self, warning: impl Into<String>) {
        self.settings_warning = Some(warning.into());
    }
}

pub struct SettingsState {
    pub active: Settings,
    pub draft: Settings,
}

impl SettingsState {
    pub fn new(settings: Settings) -> Self {
        Self {
            active: settings.clone(),
            draft: settings,
        }
    }

    /// Commits draft settings into active settings if they differ.
    /// Returns `true` if changes were committed.
    pub fn commit_draft(&mut self) -> bool {
        if self.draft == self.active {
            false
        } else {
            self.active = self.draft.clone();
            true
        }
    }

    /// Active overlay timing and layer configuration.
    pub fn overlay_config(&self) -> crate::domain::visibility::OverlayConfig {
        crate::domain::visibility::OverlayConfig {
            timeout_ms: self.active.timeout,
            activation_delay_ms: self.active.activation_delay,
            visible_layers: self.active.visible_layers.bits(),
        }
    }

    /// Creates a key paint style for the specified key size using active settings.
    pub fn paint_style(&self, unit: f32) -> KeyPaintStyle {
        KeyPaintStyle::from_settings(&self.active).with_unit(unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_draft_no_changes() {
        let mut state = SettingsState::new(Settings::default());
        assert!(!state.commit_draft());
    }

    #[test]
    fn test_commit_draft_with_changes() {
        let mut state = SettingsState::new(Settings::default());
        state.draft.timeout = 9999;
        assert_ne!(state.active.timeout, 9999);

        assert!(state.commit_draft());
        assert_eq!(state.active.timeout, 9999);

        // Subsequent commit with no further edits returns false
        assert!(!state.commit_draft());
    }
}
