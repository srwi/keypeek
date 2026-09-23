//! Firmware editor profile definitions.
//!
//! An [`EditorProfile`] defines editor structure, sidebar sections, and candidates.

use super::draft::EditorSection;
use crate::key_presenter::KeyPresenter;
use crate::key_spec::{HidKey, KeySpec, Modifiers};
use crate::keymap_editor::candidate::CandidateGroup;

/// A section group for the editor left sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarSection<T: 'static> {
    pub title: &'static str,
    pub items: &'static [T],
}

/// Target tap key and modifier configuration for Layer-Tap candidates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayerTapTarget {
    pub key: Option<HidKey>,
    pub modifiers: Modifiers,
}

/// Defines firmware editor structure, candidate presentation, and sidebar layout.
pub trait EditorProfile: KeyPresenter + Send + Sync {
    /// Returns the key presenter for this profile.
    fn presenter(&self) -> &dyn KeyPresenter;

    /// Name of the profile (for example "QMK" or "ZMK").
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Sidebar grouping and sections for this profile.
    fn sidebar_sections(&self) -> &[SidebarSection<EditorSection>];

    /// Firmware label for a section.
    fn section_label(&self, section: EditorSection) -> &'static str {
        section.label()
    }

    /// Checks if a section is supported on this profile given a device capability filter.
    fn is_section_supported(
        &self,
        section: EditorSection,
        is_action_supported: &dyn Fn(&KeySpec) -> bool,
    ) -> bool {
        if !self
            .sidebar_sections()
            .iter()
            .any(|s| s.items.contains(&section))
        {
            return false;
        }
        is_device_section_supported(section, is_action_supported)
    }

    /// Candidate groups for tap targets (such as Mod-Tap or Layer-Tap).
    fn tap_categories(&self) -> &'static [CandidateGroup];

    /// Candidate groups for an editor section.
    fn section_groups(&self, section: EditorSection) -> &'static [CandidateGroup];

    /// Candidate groups for layer operations.
    fn layer_groups(
        &self,
        layer_count: usize,
        layer_names: &[String],
        tap: LayerTapTarget,
    ) -> Vec<CandidateGroup>;

    /// Returns whether this profile supports modifier combinations on tap keys.
    fn supports_tap_modifiers(&self) -> bool {
        false
    }

    /// Returns whether this profile supports one-shot keys with a base key.
    fn supports_oneshot_keys(&self) -> bool {
        false
    }

    /// Parses a raw keycode string into a [`KeySpec`].
    fn parse_raw_keycode(&self, _raw: &str) -> Option<KeySpec> {
        None
    }
}

/// Checks if a device capability filter allows the given editor section.
fn is_device_section_supported(
    section: EditorSection,
    is_action_supported: &dyn Fn(&KeySpec) -> bool,
) -> bool {
    match section {
        EditorSection::Keyboard
        | EditorSection::Special
        | EditorSection::Layers
        | EditorSection::Combo
        | EditorSection::KeyToggle
        | EditorSection::ModTap
        | EditorSection::LayerMod
        | EditorSection::OneShot
        | EditorSection::Bluetooth
        | EditorSection::Output
        | EditorSection::System
        | EditorSection::BootPower
        | EditorSection::RawHex => true,

        EditorSection::Backlight => is_action_supported(&KeySpec::Lighting(
            crate::key_spec::LightingAction::Backlight(crate::key_spec::BacklightAction::Toggle),
        )),
        EditorSection::Rgb => is_action_supported(&KeySpec::Lighting(
            crate::key_spec::LightingAction::Rgb(crate::key_spec::RgbAction::Toggle),
        )),
        EditorSection::RgbMatrix => is_action_supported(&KeySpec::Lighting(
            crate::key_spec::LightingAction::RgbMatrix(crate::key_spec::RgbMatrixAction::Toggle),
        )),
        EditorSection::Audio => {
            is_action_supported(&KeySpec::Audio(crate::key_spec::AudioAction::Toggle))
        }
        EditorSection::Mouse => is_action_supported(&KeySpec::Mouse(
            crate::key_spec::MouseAction::Press(crate::key_spec::MouseButton::Left),
        )),
        EditorSection::Custom => {
            is_action_supported(&KeySpec::Custom(crate::key_spec::CustomBinding {
                kind: crate::key_spec::CustomKind::Macro,
                id: 0,
                name: None,
                param1: None,
                param2: None,
            }))
        }
    }
}
