//! Firmware-native editor profile abstraction and implementations.
//!
//! An [`EditorProfile`] defines firmware-specific editor structure, sidebar
//! section organization, candidate presentation, and vocabulary.

use crate::keyboard::Keyboard;
use super::draft::EditorSection;

/// A section group for the editor's left sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarSection<T: 'static> {
    pub title: &'static str,
    pub items: &'static [T],
}

/// Defines firmware-native editor structure, candidate presentation, and sidebar layout.
pub trait EditorProfile: Send + Sync {
    /// Name of the profile (e.g. "QMK", "ZMK").
    fn name(&self) -> &'static str;

    /// Sidebar grouping and sections for this profile.
    fn sidebar_sections(&self) -> &[SidebarSection<EditorSection>];

    /// Firmware-native label for a section.
    fn section_label(&self, section: EditorSection) -> &'static str;

    /// Checks if a section is supported on this profile for the given keyboard.
    fn is_section_supported(&self, section: EditorSection, keyboard: &Keyboard) -> bool {
        section.is_supported(keyboard)
    }
}

// ---------------------------------------------------------------------------
// QMK Profile
// ---------------------------------------------------------------------------

const QMK_SIDEBAR_SECTIONS: [SidebarSection<EditorSection>; 7] = [
    SidebarSection {
        title: "Keys",
        items: &[EditorSection::Keyboard, EditorSection::KeyToggle],
    },
    SidebarSection {
        title: "Layers & Mods",
        items: &[
            EditorSection::Layers,
            EditorSection::Combo,
            EditorSection::OneShot,
            EditorSection::ModTap,
            EditorSection::LayerMod,
        ],
    },
    SidebarSection {
        title: "Wireless",
        items: &[EditorSection::Bluetooth, EditorSection::Output],
    },
    SidebarSection {
        title: "Power",
        items: &[EditorSection::BootPower, EditorSection::System],
    },
    SidebarSection {
        title: "Lighting & Audio",
        items: &[
            EditorSection::Backlight,
            EditorSection::Rgb,
            EditorSection::RgbMatrix,
            EditorSection::Audio,
        ],
    },
    SidebarSection {
        title: "Mouse",
        items: &[EditorSection::Mouse],
    },
    SidebarSection {
        title: "Other",
        items: &[
            EditorSection::Special,
            EditorSection::Custom,
            EditorSection::RawHex,
        ],
    },
];

#[derive(Debug, Default, Clone, Copy)]
pub struct QmkEditorProfile;

impl EditorProfile for QmkEditorProfile {
    fn name(&self) -> &'static str {
        "QMK"
    }

    fn sidebar_sections(&self) -> &[SidebarSection<EditorSection>] {
        &QMK_SIDEBAR_SECTIONS
    }

    fn section_label(&self, section: EditorSection) -> &'static str {
        match section {
            EditorSection::OneShot => "One-Shot Mod",
            _ => section.label(),
        }
    }
}

// ---------------------------------------------------------------------------
// ZMK Profile
// ---------------------------------------------------------------------------

const ZMK_SIDEBAR_SECTIONS: [SidebarSection<EditorSection>; 7] = [
    SidebarSection {
        title: "Keys",
        items: &[EditorSection::Keyboard, EditorSection::KeyToggle],
    },
    SidebarSection {
        title: "Layers & Mods",
        items: &[
            EditorSection::Layers,
            EditorSection::Combo,
            EditorSection::OneShot,
            EditorSection::ModTap,
            EditorSection::LayerMod,
        ],
    },
    SidebarSection {
        title: "Wireless",
        items: &[EditorSection::Bluetooth, EditorSection::Output],
    },
    SidebarSection {
        title: "Power",
        items: &[EditorSection::BootPower, EditorSection::System],
    },
    SidebarSection {
        title: "Lighting",
        items: &[EditorSection::Backlight, EditorSection::Rgb],
    },
    SidebarSection {
        title: "Mouse",
        items: &[EditorSection::Mouse],
    },
    SidebarSection {
        title: "Other",
        items: &[
            EditorSection::Special,
            EditorSection::Custom,
        ],
    },
];

#[derive(Debug, Default, Clone, Copy)]
pub struct ZmkEditorProfile;

impl EditorProfile for ZmkEditorProfile {
    fn name(&self) -> &'static str {
        "ZMK"
    }

    fn sidebar_sections(&self) -> &[SidebarSection<EditorSection>] {
        &ZMK_SIDEBAR_SECTIONS
    }

    fn section_label(&self, section: EditorSection) -> &'static str {
        match section {
            EditorSection::OneShot => "Sticky Key",
            _ => section.label(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // ZMK does not generate RawHex or QMK-specific lighting/audio sections
        assert!(!all_items.contains(&EditorSection::RawHex));
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
            Box::new(crate::protocols::mock::MockProtocol::connect().unwrap());
        let layout_name = protocol.get_layout_definition().layouts[0].name.clone();
        let keyboard = Keyboard::new(
            protocol,
            layout_name,
            crate::keyboard::OverlayConfig {
                timeout_ms: 2000,
                activation_delay_ms: 300,
                visible_layers: u32::MAX,
            },
            crate::ui_wake::UiWake::new(std::sync::Arc::new(|| ())),
        )
        .unwrap();

        let qmk = QmkEditorProfile;
        assert!(qmk.is_section_supported(EditorSection::Keyboard, &keyboard));
        assert!(qmk.is_section_supported(EditorSection::Layers, &keyboard));
    }
}
