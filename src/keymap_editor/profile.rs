//! Firmware-native editor profile abstraction and implementations.
//!
//! An [`EditorProfile`] defines firmware-specific editor structure, sidebar
//! section organization, candidate presentation, and vocabulary.

use std::sync::OnceLock;

use crate::keyboard::Keyboard;
use crate::key_spec::{HidKey, KeySpec, LayerInfo};
use crate::keymap_editor::catalog::common;
use crate::keymap_editor::picker::{CandidateGroup};
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
        section.is_supported(keyboard)
    }

    /// Candidate group for standard keyboard keys.
    fn keyboard_group(&self) -> &'static CandidateGroup;

    /// Candidate group for media/consumer keys.
    #[allow(dead_code)]
    fn media_group(&self) -> &'static CandidateGroup;

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

fn qmk_keyboard_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let usages = crate::protocols::qmk_codec::qmk_all_basic_usages();
        let candidates = common::build_keyboard_candidates(usages, |id| {
            crate::protocols::qmk_codec::qmk_search_tokens_for_hid(0x07, id)
        });
        CandidateGroup {
            name: "Keyboard",
            candidates,
        }
    })
}

fn qmk_media_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let usages = crate::protocols::qmk_codec::qmk_all_media_usages();
        let candidates = common::build_media_candidates(usages, |id| {
            crate::protocols::qmk_codec::qmk_search_tokens_for_hid(0x0C, id)
        });
        CandidateGroup {
            name: "Media",
            candidates,
        }
    })
}

fn qmk_tap_categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(|| vec![qmk_keyboard_group().clone(), qmk_media_group().clone()])
}

fn qmk_bluetooth_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| vec![common::build_bluetooth_group(|_| &[])])
}

fn qmk_output_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| vec![common::build_output_group(|_| &[])])
}

fn qmk_system_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        vec![common::build_system_group(|id| match id {
            0x81 => &["KC_SYSTEM_POWER"],
            0x82 => &["KC_SYSTEM_SLEEP"],
            0x83 => &["KC_SYSTEM_WAKE"],
            _ => &[],
        })]
    })
}

fn qmk_boot_power_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::PowerAction;
        vec![common::build_boot_power_group(|act| match act {
            PowerAction::Reset => &["QK_BOOT"],
            _ => &[],
        })]
    })
}

fn qmk_backlight_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::BacklightAction;
        vec![common::build_backlight_group(|act| match act {
            BacklightAction::Toggle => &["BL_TOGG", "QK_BACKLIGHT_TOGGLE"],
            BacklightAction::On => &["BL_ON", "QK_BACKLIGHT_ON"],
            BacklightAction::Off => &["BL_OFF", "QK_BACKLIGHT_OFF"],
            BacklightAction::Inc => &["BL_UP", "QK_BACKLIGHT_UP"],
            BacklightAction::Dec => &["BL_DOWN", "QK_BACKLIGHT_DOWN"],
            BacklightAction::Cycle => &["BL_STEP", "QK_BACKLIGHT_STEP"],
            BacklightAction::BreathingToggle => &["BL_BRTG", "QK_BACKLIGHT_TOGGLE_BREATHING"],
            _ => &[],
        })]
    })
}

fn qmk_rgb_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::RgbAction;
        vec![common::build_rgb_underglow_group(|act| match act {
            RgbAction::Toggle => &["RGB_TOG", "QK_UNDERGLOW_TOGGLE"],
            RgbAction::On => &["RGB_ON"],
            RgbAction::Off => &["RGB_OFF"],
            RgbAction::EffectInc => &["RGB_MOD", "QK_UNDERGLOW_MODE_NEXT"],
            RgbAction::EffectDec => &["RGB_RMOD", "QK_UNDERGLOW_MODE_PREVIOUS"],
            RgbAction::HueInc => &["RGB_HUI", "QK_UNDERGLOW_HUE_UP"],
            RgbAction::HueDec => &["RGB_HUD", "QK_UNDERGLOW_HUE_DOWN"],
            RgbAction::SatInc => &["RGB_SAI", "QK_UNDERGLOW_SATURATION_UP"],
            RgbAction::SatDec => &["RGB_SAD", "QK_UNDERGLOW_SATURATION_DOWN"],
            RgbAction::BrightInc => &["RGB_VAI", "QK_UNDERGLOW_VALUE_UP"],
            RgbAction::BrightDec => &["RGB_VAD", "QK_UNDERGLOW_VALUE_DOWN"],
            RgbAction::SpeedInc => &["RGB_SPI", "QK_UNDERGLOW_SPEED_UP"],
            RgbAction::SpeedDec => &["RGB_SPD", "QK_UNDERGLOW_SPEED_DOWN"],
            _ => &[],
        })]
    })
}

fn qmk_rgb_matrix_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::RgbMatrixAction;
        vec![common::build_rgb_matrix_group(|act| match act {
            RgbMatrixAction::Toggle => &["RGB_MATRIX_TOGGLE"],
            RgbMatrixAction::On => &["RGB_MATRIX_ON"],
            RgbMatrixAction::Off => &["RGB_MATRIX_OFF"],
            RgbMatrixAction::ModeNext => &["RGB_MATRIX_MODE_NEXT"],
            RgbMatrixAction::ModePrev => &["RGB_MATRIX_MODE_PREVIOUS"],
            RgbMatrixAction::HueInc => &["RGB_MATRIX_HUE_UP"],
            RgbMatrixAction::HueDec => &["RGB_MATRIX_HUE_DOWN"],
            RgbMatrixAction::SatInc => &["RGB_MATRIX_SATURATION_UP"],
            RgbMatrixAction::SatDec => &["RGB_MATRIX_SATURATION_DOWN"],
            RgbMatrixAction::BrightInc => &["RGB_MATRIX_VALUE_UP"],
            RgbMatrixAction::BrightDec => &["RGB_MATRIX_VALUE_DOWN"],
            RgbMatrixAction::SpeedInc => &["RGB_MATRIX_SPEED_UP"],
            RgbMatrixAction::SpeedDec => &["RGB_MATRIX_SPEED_DOWN"],
            _ => &[],
        })]
    })
}

fn qmk_audio_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::AudioAction;
        vec![common::build_audio_group(|act| match act {
            AudioAction::On => &["QK_AUDIO_ON"],
            AudioAction::Off => &["QK_AUDIO_OFF"],
            AudioAction::Toggle => &["QK_AUDIO_TOGGLE"],
            AudioAction::ClickyToggle => &["QK_AUDIO_CLICKY_TOGGLE"],
            AudioAction::ClickyOn => &["QK_AUDIO_CLICKY_ON"],
            AudioAction::ClickyOff => &["QK_AUDIO_CLICKY_OFF"],
            AudioAction::ClickyUp => &["QK_AUDIO_CLICKY_UP"],
            AudioAction::ClickyDown => &["QK_AUDIO_CLICKY_DOWN"],
            AudioAction::ClickyReset => &["QK_AUDIO_CLICKY_RESET"],
            AudioAction::MusicOn => &["QK_MUSIC_ON"],
            AudioAction::MusicOff => &["QK_MUSIC_OFF"],
            AudioAction::MusicToggle => &["QK_MUSIC_TOGGLE"],
            AudioAction::MusicModeNext => &["QK_MUSIC_MODE_NEXT"],
            AudioAction::VoiceNext => &["QK_AUDIO_VOICE_NEXT"],
            AudioAction::VoicePrev => &["QK_AUDIO_VOICE_PREVIOUS"],
            _ => &[],
        })]
    })
}

fn qmk_mouse_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::{MouseAction, MouseButton};
        common::build_mouse_groups(|spec| match spec {
            KeySpec::Mouse(MouseAction::Press(MouseButton::Left)) => &["MS_BTN1"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Right)) => &["MS_BTN2"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Middle)) => &["MS_BTN3"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Button4)) => &["MS_BTN4"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Button5)) => &["MS_BTN5"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Other(6))) => &["MS_BTN6"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Other(7))) => &["MS_BTN7"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Other(8))) => &["MS_BTN8"],
            KeySpec::Mouse(MouseAction::Move { x: 0, y: -1 }) => &["MS_UP"],
            KeySpec::Mouse(MouseAction::Move { x: 0, y: 1 }) => &["MS_DOWN"],
            KeySpec::Mouse(MouseAction::Move { x: -1, y: 0 }) => &["MS_LEFT"],
            KeySpec::Mouse(MouseAction::Move { x: 1, y: 0 }) => &["MS_RIGHT"],
            KeySpec::Mouse(MouseAction::Scroll { x: 0, y: 1 }) => &["MS_WH_UP"],
            KeySpec::Mouse(MouseAction::Scroll { x: 0, y: -1 }) => &["MS_WH_DOWN"],
            KeySpec::Mouse(MouseAction::Acceleration(0)) => &["MS_ACL0"],
            KeySpec::Mouse(MouseAction::Acceleration(1)) => &["MS_ACL1"],
            KeySpec::Mouse(MouseAction::Acceleration(2)) => &["MS_ACL2"],
            _ => &[],
        })
    })
}

fn qmk_special_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        vec![common::build_special_group(|spec| match spec {
            KeySpec::Transparent => &["KC_TRNS"],
            KeySpec::None => &["KC_NO"],
            KeySpec::CapsWord => &["QK_CAPS_WORD_TOGGLE"],
            KeySpec::KeyRepeat => &["QK_KEY_REPEAT"],
            KeySpec::GraveEscape => &["QK_GRAVE_ESCAPE"],
            _ => &[],
        })]
    })
}

fn common_custom_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| common::build_custom_groups())
}

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

    fn keyboard_group(&self) -> &'static CandidateGroup {
        qmk_keyboard_group()
    }

    fn media_group(&self) -> &'static CandidateGroup {
        qmk_media_group()
    }

    fn tap_categories(&self) -> &'static [CandidateGroup] {
        qmk_tap_categories()
    }

    fn section_groups(&self, section: EditorSection) -> &'static [CandidateGroup] {
        match section {
            EditorSection::Keyboard | EditorSection::KeyToggle | EditorSection::Combo | EditorSection::ModTap | EditorSection::OneShot => {
                qmk_tap_categories()
            }
            EditorSection::Bluetooth => qmk_bluetooth_groups(),
            EditorSection::Output => qmk_output_groups(),
            EditorSection::System => qmk_system_groups(),
            EditorSection::BootPower => qmk_boot_power_groups(),
            EditorSection::Backlight => qmk_backlight_groups(),
            EditorSection::Rgb => qmk_rgb_groups(),
            EditorSection::RgbMatrix => qmk_rgb_matrix_groups(),
            EditorSection::Audio => qmk_audio_groups(),
            EditorSection::Mouse => qmk_mouse_groups(),
            EditorSection::Special => qmk_special_groups(),
            EditorSection::Custom => common_custom_groups(),
            EditorSection::Layers | EditorSection::LayerMod | EditorSection::RawHex => &[],
        }
    }

    fn layer_groups(
        &self,
        layer_count: usize,
        layer_infos: &[LayerInfo],
        layer_names: &[String],
        tap_key: Option<HidKey>,
    ) -> Vec<CandidateGroup> {
        common::build_layer_groups(layer_count, layer_infos, layer_names, tap_key, |_| &[])
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

fn zmk_keyboard_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let usages = crate::protocols::zmk_codec::zmk_all_keyboard_usages();
        let candidates = common::build_keyboard_candidates(usages, |id| {
            crate::protocols::zmk_codec::zmk_search_tokens_for_hid(0x07, id)
        });
        CandidateGroup {
            name: "Keyboard",
            candidates,
        }
    })
}

fn zmk_media_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let usages = crate::protocols::zmk_codec::zmk_all_consumer_usages();
        let candidates = common::build_media_candidates(usages, |id| {
            crate::protocols::zmk_codec::zmk_search_tokens_for_hid(0x0C, id)
        });
        CandidateGroup {
            name: "Media",
            candidates,
        }
    })
}

fn zmk_tap_categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(|| vec![zmk_keyboard_group().clone(), zmk_media_group().clone()])
}

fn zmk_bluetooth_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::BluetoothAction;
        vec![common::build_bluetooth_group(|act| match act {
            BluetoothAction::Clear => &["&bt BT_CLR", "bt_clr"],
            BluetoothAction::Next => &["&bt BT_NXT", "bt_nxt"],
            BluetoothAction::Prev => &["&bt BT_PRV", "bt_prv"],
            BluetoothAction::ClearAll => &["&bt BT_CLR_ALL", "bt_clr_all"],
            BluetoothAction::Select(_) => &["&bt BT_SEL", "bt_sel"],
            BluetoothAction::Disconnect(_) => &["&bt BT_DISC", "bt_disc"],
            _ => &[],
        })]
    })
}

fn zmk_output_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::OutputTarget;
        vec![common::build_output_group(|target| match target {
            OutputTarget::Toggle => &["&out OUT_TOG", "out_tog"],
            OutputTarget::Usb => &["&out OUT_USB", "out_usb"],
            OutputTarget::Ble => &["&out OUT_BLE", "out_ble"],
            OutputTarget::None => &["&out OUT_NONE", "out_none"],
            _ => &[],
        })]
    })
}

fn zmk_system_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| vec![common::build_system_group(|_| &[])])
}

fn zmk_boot_power_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::PowerAction;
        vec![common::build_boot_power_group(|act| match act {
            PowerAction::Reset => &["&sys_reset"],
            PowerAction::Bootloader => &["&bootloader"],
            PowerAction::SoftOff => &["&soft_off"],
            PowerAction::UnlockKeymap => &["&studio_unlock"],
            PowerAction::Toggle => &["&ext_power EP_TOG"],
            PowerAction::On => &["&ext_power EP_ON"],
            PowerAction::Off => &["&ext_power EP_OFF"],
            _ => &[],
        })]
    })
}

fn zmk_backlight_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::BacklightAction;
        vec![common::build_backlight_group(|act| match act {
            BacklightAction::Toggle => &["&bl BL_TOG"],
            BacklightAction::On => &["&bl BL_ON"],
            BacklightAction::Off => &["&bl BL_OFF"],
            BacklightAction::Inc => &["&bl BL_INC"],
            BacklightAction::Dec => &["&bl BL_DEC"],
            BacklightAction::Cycle => &["&bl BL_CYCLE"],
            _ => &[],
        })]
    })
}

fn zmk_rgb_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::RgbAction;
        vec![common::build_rgb_underglow_group(|act| match act {
            RgbAction::Toggle => &["&rgb_ug RGB_TOG"],
            RgbAction::On => &["&rgb_ug RGB_ON"],
            RgbAction::Off => &["&rgb_ug RGB_OFF"],
            RgbAction::EffectInc => &["&rgb_ug RGB_EFF"],
            RgbAction::EffectDec => &["&rgb_ug RGB_EFR"],
            RgbAction::HueInc => &["&rgb_ug RGB_HUI"],
            RgbAction::HueDec => &["&rgb_ug RGB_HUD"],
            RgbAction::SatInc => &["&rgb_ug RGB_SAI"],
            RgbAction::SatDec => &["&rgb_ug RGB_SAD"],
            RgbAction::BrightInc => &["&rgb_ug RGB_BRI"],
            RgbAction::BrightDec => &["&rgb_ug RGB_BRD"],
            RgbAction::SpeedInc => &["&rgb_ug RGB_SPI"],
            RgbAction::SpeedDec => &["&rgb_ug RGB_SPD"],
            _ => &[],
        })]
    })
}

fn zmk_mouse_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::{MouseAction, MouseButton};
        common::build_mouse_groups(|spec| match spec {
            KeySpec::Mouse(MouseAction::Press(MouseButton::Left)) => &["&mkp LCLK"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Right)) => &["&mkp RCLK"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Middle)) => &["&mkp MCLK"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Button4)) => &["&mkp MB4"],
            KeySpec::Mouse(MouseAction::Press(MouseButton::Button5)) => &["&mkp MB5"],
            KeySpec::Mouse(MouseAction::Move { x: 0, y: -1 }) => &["&mmv MOVE_UP"],
            KeySpec::Mouse(MouseAction::Move { x: 0, y: 1 }) => &["&mmv MOVE_DOWN"],
            KeySpec::Mouse(MouseAction::Move { x: -1, y: 0 }) => &["&mmv MOVE_LEFT"],
            KeySpec::Mouse(MouseAction::Move { x: 1, y: 0 }) => &["&mmv MOVE_RIGHT"],
            KeySpec::Mouse(MouseAction::Scroll { x: 0, y: 1 }) => &["&msc SCRL_UP"],
            KeySpec::Mouse(MouseAction::Scroll { x: 0, y: -1 }) => &["&msc SCRL_DOWN"],
            _ => &[],
        })
    })
}

fn zmk_special_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        vec![common::build_special_group(|spec| match spec {
            KeySpec::Transparent => &["&trans"],
            KeySpec::None => &["&none"],
            KeySpec::CapsWord => &["&caps_word"],
            KeySpec::KeyRepeat => &["&key_repeat"],
            _ => &[],
        })]
    })
}

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

    fn keyboard_group(&self) -> &'static CandidateGroup {
        zmk_keyboard_group()
    }

    fn media_group(&self) -> &'static CandidateGroup {
        zmk_media_group()
    }

    fn tap_categories(&self) -> &'static [CandidateGroup] {
        zmk_tap_categories()
    }

    fn section_groups(&self, section: EditorSection) -> &'static [CandidateGroup] {
        match section {
            EditorSection::Keyboard | EditorSection::KeyToggle | EditorSection::Combo | EditorSection::ModTap | EditorSection::OneShot => {
                zmk_tap_categories()
            }
            EditorSection::Bluetooth => zmk_bluetooth_groups(),
            EditorSection::Output => zmk_output_groups(),
            EditorSection::System => zmk_system_groups(),
            EditorSection::BootPower => zmk_boot_power_groups(),
            EditorSection::Backlight => zmk_backlight_groups(),
            EditorSection::Rgb => zmk_rgb_groups(),
            EditorSection::RgbMatrix => &[],
            EditorSection::Audio => &[],
            EditorSection::Mouse => zmk_mouse_groups(),
            EditorSection::Special => zmk_special_groups(),
            EditorSection::Custom => common_custom_groups(),
            EditorSection::Layers | EditorSection::LayerMod | EditorSection::RawHex => &[],
        }
    }

    fn layer_groups(
        &self,
        layer_count: usize,
        layer_infos: &[LayerInfo],
        layer_names: &[String],
        tap_key: Option<HidKey>,
    ) -> Vec<CandidateGroup> {
        common::build_layer_groups(layer_count, layer_infos, layer_names, tap_key, |act| {
            use crate::key_spec::LayerActivation;
            match act {
                LayerActivation::Momentary => &["&mo"],
                LayerActivation::Toggle => &["&tog"],
                LayerActivation::To => &["&to"],
                LayerActivation::Sticky => &["&sl"],
                _ => &[],
            }
        })
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
        assert!(qmk.is_section_supported(EditorSection::RawHex, &keyboard));

        let zmk = ZmkEditorProfile;
        // ZMK does not support RawHex section even if protocol supports it
        assert!(!zmk.is_section_supported(EditorSection::RawHex, &keyboard));
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

        let qmk = QmkEditorProfile;
        assert!(!qmk.section_groups(EditorSection::RgbMatrix).is_empty());
        assert!(!qmk.section_groups(EditorSection::Audio).is_empty());
    }
}
