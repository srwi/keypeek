use crate::settings::Settings;

pub struct UiState {
    pub settings_visible: bool,
    pub settings_error: Option<String>,
    pub settings_warning: Option<String>,
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
