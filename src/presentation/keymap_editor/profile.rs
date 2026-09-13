//! Firmware-native editor profile abstraction and implementations.
//!
//! An [`EditorProfile`] defines firmware-specific editor structure, sidebar
//! section organization, candidate presentation, and vocabulary.

use crate::application::Keyboard;
use crate::key_presenter::KeyPresenter;
use crate::key_spec::{HidKey, KeySpec, LayerInfo};
use crate::keymap_editor::picker::CandidateGroup;
use super::draft::EditorSection;

/// A section group for the editor's left sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarSection<T: 'static> {
    pub title: &'static str,
    pub items: &'static [T],
}

/// Defines firmware-native editor structure, candidate presentation, and sidebar layout.
pub trait EditorProfile: KeyPresenter + Send + Sync {
    /// Returns the key presenter for this profile.
    fn presenter(&self) -> &dyn KeyPresenter;

    /// Name of the profile (e.g. "QMK", "ZMK").
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Sidebar grouping and sections for this profile.
    fn sidebar_sections(&self) -> &[SidebarSection<EditorSection>];

    /// Firmware-native label for a section.
    fn section_label(&self, section: EditorSection) -> &'static str;

    /// Checks if a section is supported on this profile for the given keyboard.
    fn is_section_supported(&self, section: EditorSection, keyboard: &Keyboard) -> bool {
        if !self.sidebar_sections().iter().any(|s| s.items.contains(&section)) {
            return false;
        }
        is_device_section_supported(section, keyboard)
    }

    /// Candidate groups suitable for tap targets (e.g. Mod-Tap, Layer-Tap).
    fn tap_categories(&self) -> &'static [CandidateGroup];

    /// Candidate groups for a given editor section.
    fn section_groups(&self, section: EditorSection) -> &'static [CandidateGroup];

    /// Candidate groups for layer operations.
    fn layer_groups(
        &self,
        layer_count: usize,
        layer_infos: &[LayerInfo],
        layer_names: &[String],
        tap_key: Option<HidKey>,
    ) -> Vec<CandidateGroup>;

    /// Parses a raw firmware keycode string (e.g. hex input in the Any Keycode section)
    /// into a domain [`KeySpec`].
    fn parse_raw_keycode(&self, _raw: &str) -> Option<KeySpec> {
        None
    }
}

/// Checks if a device capability filter allows the given editor section on the connected keyboard.
fn is_device_section_supported(section: EditorSection, keyboard: &Keyboard) -> bool {
    match section {
        EditorSection::Keyboard
        | EditorSection::Special
        | EditorSection::Layers
        | EditorSection::Combo => true,
        EditorSection::KeyToggle => keyboard.is_action_supported(&KeySpec::KeyToggle {
            key: HidKey::keyboard(0x04),
            modifiers: crate::hid_labels::Modifiers::default(),
        }),
        EditorSection::ModTap => {
            let sample = KeySpec::ModTap {
                hold: crate::hid_labels::Modifiers {
                    shift: true,
                    ..Default::default()
                },
                tap: HidKey::keyboard(0x04),
                tap_modifiers: crate::hid_labels::Modifiers::default(),
            };
            keyboard.is_action_supported(&sample)
        }
        EditorSection::LayerMod => keyboard.is_action_supported(&KeySpec::Layer {
            layer: 0,
            activation: crate::key_spec::LayerActivation::LayerMod(crate::hid_labels::Modifiers {
                shift: true,
                ..Default::default()
            }),
        }),
        EditorSection::OneShot => {
            let sample = KeySpec::StickyKey {
                key: None,
                modifiers: crate::hid_labels::Modifiers {
                    shift: true,
                    ..Default::default()
                },
            };
            keyboard.is_action_supported(&sample)
        }
        EditorSection::Bluetooth => {
            keyboard.is_action_supported(&KeySpec::Bluetooth(crate::key_spec::BluetoothAction::Clear))
        }
        EditorSection::Output => {
            keyboard.is_action_supported(&KeySpec::Output(crate::key_spec::OutputTarget::Toggle))
        }
        EditorSection::System => keyboard.is_action_supported(&KeySpec::KeyPress {
            key: HidKey::system(0x81),
            modifiers: crate::hid_labels::Modifiers::default(),
        }),
        EditorSection::BootPower => {
            keyboard.is_action_supported(&KeySpec::Power(crate::key_spec::PowerAction::Reset))
        }
        EditorSection::Backlight => keyboard.is_action_supported(&KeySpec::Lighting(
            crate::key_spec::LightingAction::Backlight(crate::key_spec::BacklightAction::Toggle),
        )),
        EditorSection::Rgb => keyboard.is_action_supported(&KeySpec::Lighting(
            crate::key_spec::LightingAction::Rgb(crate::key_spec::RgbAction::Toggle),
        )),
        EditorSection::RgbMatrix => keyboard.is_action_supported(&KeySpec::Lighting(
            crate::key_spec::LightingAction::RgbMatrix(crate::key_spec::RgbMatrixAction::Toggle),
        )),
        EditorSection::Audio => {
            keyboard.is_action_supported(&KeySpec::Audio(crate::key_spec::AudioAction::Toggle))
        }
        EditorSection::Mouse => keyboard.is_action_supported(&KeySpec::Mouse(
            crate::key_spec::MouseAction::Press(crate::key_spec::MouseButton::Left),
        )),
        EditorSection::Custom => keyboard.is_action_supported(&KeySpec::Custom(
            crate::key_spec::CustomBinding {
                kind: crate::key_spec::CustomKind::Macro,
                id: 0,
                name: None,
                param1: None,
                param2: None,
            },
        )),
        EditorSection::RawHex => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firmware::qmk::QmkEditorProfile;
    use crate::firmware::zmk::ZmkEditorProfile;

    #[test]
    fn qmk_profile_exposes_qmk_native_sections_and_names() {
        let profile = QmkEditorProfile;
        assert_eq!(profile.name(), "QMK");
        assert_eq!(profile.section_label(EditorSection::OneShot), "One-Shot Mod");
        assert_eq!(profile.section_label(EditorSection::Keyboard), "Key Press");
        assert_eq!(profile.section_label(EditorSection::RawHex), "Any Keycode");

        let all_items: Vec<EditorSection> = profile
            .sidebar_sections()
            .iter()
            .flat_map(|s| s.items.iter().copied())
            .collect();
        assert!(all_items.contains(&EditorSection::RawHex));
        assert!(all_items.contains(&EditorSection::RgbMatrix));
        assert!(all_items.contains(&EditorSection::Audio));
        assert!(all_items.contains(&EditorSection::LayerMod));
        // QMK does not offer KeyToggle or Wireless sections
        assert!(!all_items.contains(&EditorSection::KeyToggle));
        assert!(!all_items.contains(&EditorSection::Bluetooth));
        assert!(!all_items.contains(&EditorSection::Output));
    }

    #[test]
    fn zmk_profile_exposes_zmk_native_sections_and_names() {
        let profile = ZmkEditorProfile;
        assert_eq!(profile.name(), "ZMK");
        assert_eq!(profile.section_label(EditorSection::OneShot), "Sticky Key");
        assert_eq!(profile.section_label(EditorSection::Keyboard), "Key Press");

        let all_items: Vec<EditorSection> = profile
            .sidebar_sections()
            .iter()
            .flat_map(|s| s.items.iter().copied())
            .collect();
        assert!(all_items.contains(&EditorSection::KeyToggle));
        assert!(all_items.contains(&EditorSection::Bluetooth));
        assert!(all_items.contains(&EditorSection::Output));
        // ZMK does not generate RawHex, LayerMod, or QMK-specific lighting/audio sections
        assert!(!all_items.contains(&EditorSection::RawHex));
        assert!(!all_items.contains(&EditorSection::LayerMod));
        assert!(!all_items.contains(&EditorSection::RgbMatrix));
        assert!(!all_items.contains(&EditorSection::Audio));
    }

    #[test]
    fn profiles_differ_in_hierarchy_and_content() {
        let qmk = QmkEditorProfile;
        let zmk = ZmkEditorProfile;

        let qmk_lighting = qmk
            .sidebar_sections()
            .iter()
            .find(|s| s.title == "Lighting & Audio")
            .expect("QMK has Lighting & Audio");
        assert_eq!(qmk_lighting.items.len(), 4);

        let zmk_lighting = zmk
            .sidebar_sections()
            .iter()
            .find(|s| s.title == "Lighting")
            .expect("ZMK has Lighting");
        assert_eq!(zmk_lighting.items.len(), 2);

        let qmk_keys = qmk
            .sidebar_sections()
            .iter()
            .find(|s| s.title == "Keys")
            .expect("QMK has Keys");
        assert_eq!(qmk_keys.items, &[EditorSection::Keyboard]);

        let zmk_keys = zmk
            .sidebar_sections()
            .iter()
            .find(|s| s.title == "Keys")
            .expect("ZMK has Keys");
        assert_eq!(
            zmk_keys.items,
            &[EditorSection::Keyboard, EditorSection::KeyToggle]
        );

        let qmk_has_wireless = qmk
            .sidebar_sections()
            .iter()
            .any(|s| s.title == "Wireless");
        assert!(!qmk_has_wireless, "QMK should not have Wireless section");

        let zmk_has_wireless = zmk
            .sidebar_sections()
            .iter()
            .any(|s| s.title == "Wireless");
        assert!(zmk_has_wireless, "ZMK should have Wireless section");

        let qmk_other = qmk
            .sidebar_sections()
            .iter()
            .find(|s| s.title == "Other")
            .unwrap();
        assert!(qmk_other.items.contains(&EditorSection::RawHex));

        let zmk_other = zmk
            .sidebar_sections()
            .iter()
            .find(|s| s.title == "Other")
            .unwrap();
        assert!(!zmk_other.items.contains(&EditorSection::RawHex));
    }

    #[test]
    fn profile_delegates_section_support() {
        let protocol: Box<dyn crate::protocols::KeyboardProtocol> =
            Box::new(crate::firmware::mock::MockProtocol::connect().unwrap());
        let layout_name = protocol.get_layout_definition().layouts[0].name.clone();
        let keyboard = Keyboard::new(
            protocol,
            layout_name,
            crate::domain::visibility::OverlayConfig {
                timeout_ms: 2000,
                activation_delay_ms: 300,
                visible_layers: u32::MAX,
            },
            crate::ui_wake::UiWake::new(std::sync::Arc::new(|| ())),
            std::sync::Arc::new(QmkEditorProfile),
        )
        .unwrap();

        let qmk = QmkEditorProfile;
        assert!(qmk.is_section_supported(EditorSection::Keyboard, &keyboard));
        assert!(qmk.is_section_supported(EditorSection::Layers, &keyboard));
        assert!(qmk.is_section_supported(EditorSection::RawHex, &keyboard));
        // QMK rejects KeyToggle and Wireless sections regardless of device
        assert!(!qmk.is_section_supported(EditorSection::KeyToggle, &keyboard));
        assert!(!qmk.is_section_supported(EditorSection::Bluetooth, &keyboard));
        assert!(!qmk.is_section_supported(EditorSection::Output, &keyboard));

        let zmk = ZmkEditorProfile;
        // ZMK does not support RawHex or LayerMod section even if protocol supports it
        assert!(!zmk.is_section_supported(EditorSection::RawHex, &keyboard));
        assert!(!zmk.is_section_supported(EditorSection::LayerMod, &keyboard));
    }

    #[test]
    fn qmk_attaches_qmk_aliases_only() {
        let qmk = QmkEditorProfile;
        let boot = qmk.section_groups(EditorSection::BootPower);
        let qk_boot = boot[0].candidates.iter().find(|c| c.matches_query("QK_BOOT"));
        assert!(qk_boot.is_some(), "QMK profile should contain QK_BOOT token");

        let zmk = ZmkEditorProfile;
        let zmk_boot = zmk.section_groups(EditorSection::BootPower);
        let zmk_has_qk = zmk_boot[0].candidates.iter().any(|c| c.matches_query("QK_BOOT"));
        assert!(!zmk_has_qk, "ZMK profile should NOT contain QK_BOOT token");

        let zmk_has_reset = zmk_boot[0].candidates.iter().any(|c| c.matches_query("&sys_reset"));
        assert!(zmk_has_reset, "ZMK profile should contain &sys_reset token");
    }

    #[test]
    fn zmk_omits_unsupported_groups() {
        let zmk = ZmkEditorProfile;
        assert!(zmk.section_groups(EditorSection::RgbMatrix).is_empty());
        assert!(zmk.section_groups(EditorSection::Audio).is_empty());
        assert!(zmk.section_groups(EditorSection::LayerMod).is_empty());

        let qmk = QmkEditorProfile;
        assert!(!qmk.section_groups(EditorSection::RgbMatrix).is_empty());
        assert!(!qmk.section_groups(EditorSection::Audio).is_empty());
        assert!(qmk.section_groups(EditorSection::Bluetooth).is_empty());
        assert!(qmk.section_groups(EditorSection::Output).is_empty());
        assert!(qmk.section_groups(EditorSection::KeyToggle).is_empty());
    }

    #[test]
    fn capability_handling_separates_family_from_device() {
        // 1. Family constraints: ZMK profile does not offer RawHex section or candidates,
        //    even if the connected protocol were to accept raw hex.
        let protocol: Box<dyn crate::protocols::KeyboardProtocol> =
            Box::new(crate::firmware::mock::MockProtocol::connect().unwrap());
        let layout_name = protocol.get_layout_definition().layouts[0].name.clone();
        let keyboard = Keyboard::new(
            protocol,
            layout_name,
            crate::domain::visibility::OverlayConfig {
                timeout_ms: 2000,
                activation_delay_ms: 300,
                visible_layers: u32::MAX,
            },
            crate::ui_wake::UiWake::new(std::sync::Arc::new(|| ())),
            std::sync::Arc::new(QmkEditorProfile),
        )
        .unwrap();

        let zmk = ZmkEditorProfile;
        assert!(!zmk.is_section_supported(EditorSection::RawHex, &keyboard));
        assert!(!zmk.is_section_supported(EditorSection::Audio, &keyboard));
        assert!(!zmk.is_section_supported(EditorSection::RgbMatrix, &keyboard));
        assert!(!zmk.is_section_supported(EditorSection::LayerMod, &keyboard));

        let qmk = QmkEditorProfile;
        assert!(!qmk.is_section_supported(EditorSection::KeyToggle, &keyboard));
        assert!(!qmk.is_section_supported(EditorSection::Bluetooth, &keyboard));
        assert!(!qmk.is_section_supported(EditorSection::Output, &keyboard));

        // Candidate group family separation:
        // ZMK does not generate Mouse Acceleration
        let zmk_mouse = zmk.section_groups(EditorSection::Mouse);
        assert_eq!(zmk_mouse.len(), 3);
        assert!(!zmk_mouse.iter().any(|g| g.name == "Mouse Acceleration"));

        // QMK generates Mouse Acceleration
        let qmk_mouse = qmk.section_groups(EditorSection::Mouse);
        assert_eq!(qmk_mouse.len(), 4);
        assert!(qmk_mouse.iter().any(|g| g.name == "Mouse Acceleration"));

        // ZMK layer operations omit Default and TapToggle
        let zmk_layers = zmk.layer_groups(4, &[], &[], None);
        assert!(!zmk_layers.iter().any(|g| g.name == "Set Default Layer"));
        assert!(!zmk_layers.iter().any(|g| g.name == "Tap Toggle"));

        // QMK layer operations include Default and TapToggle
        let qmk_layers = qmk.layer_groups(4, &[], &[], None);
        assert!(qmk_layers.iter().any(|g| g.name == "Set Default Layer"));
        assert!(qmk_layers.iter().any(|g| g.name == "Tap Toggle"));

        // Boot & Power family separation:
        // QMK boot & power includes Clear EEPROM, omits Soft Off
        let qmk_boot = qmk.section_groups(EditorSection::BootPower);
        assert!(qmk_boot[0].candidates.iter().any(|c| matches!(
            c.binding,
            KeySpec::Power(crate::key_spec::PowerAction::Other(0xEE))
        )));
        assert!(!qmk_boot[0].candidates.iter().any(|c| matches!(
            c.binding,
            KeySpec::Power(crate::key_spec::PowerAction::SoftOff)
        )));

        // ZMK boot & power includes Soft Off, omits Clear EEPROM
        let zmk_boot = zmk.section_groups(EditorSection::BootPower);
        assert!(zmk_boot[0].candidates.iter().any(|c| matches!(
            c.binding,
            KeySpec::Power(crate::key_spec::PowerAction::SoftOff)
        )));
        assert!(!zmk_boot[0].candidates.iter().any(|c| matches!(
            c.binding,
            KeySpec::Power(crate::key_spec::PowerAction::Other(0xEE))
        )));

        // 2. Device capability variation: QMK feature flags filter lighting/audio actions on protocol level
        let qmk_features = crate::firmware::qmk::common::QmkFeatures {
            has_backlight: true,
            has_rgblight: false,
            has_rgb_matrix: false,
            has_audio: false,
        };
        let filter = crate::firmware::qmk::common::qmk_action_filter(qmk_features).unwrap();
        // Backlight is enabled
        assert!(filter(&KeySpec::Lighting(crate::key_spec::LightingAction::Backlight(
            crate::key_spec::BacklightAction::Toggle
        ))));
        // RGBLight is disabled on this device
        assert!(!filter(&KeySpec::Lighting(crate::key_spec::LightingAction::Rgb(
            crate::key_spec::RgbAction::Toggle
        ))));
        // Audio is disabled on this device
        assert!(!filter(&KeySpec::Audio(crate::key_spec::AudioAction::Toggle)));
    }

    #[test]
    fn raw_keycode_parsing_is_profile_specific() {
        let qmk = QmkEditorProfile;
        let zmk = ZmkEditorProfile;

        assert_eq!(
            qmk.parse_raw_keycode("0004"),
            Some(KeySpec::KeyPress {
                key: HidKey::keyboard(0x04),
                modifiers: Default::default(),
            })
        );
        assert_eq!(qmk.parse_raw_keycode("invalid"), None);
        assert_eq!(zmk.parse_raw_keycode("0004"), None);
    }
}
