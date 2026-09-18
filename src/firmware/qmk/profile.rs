use std::sync::OnceLock;

use crate::firmware::qmk::QmkKeyPresenter;
use crate::key_presenter::KeyPresenter;
use crate::key_spec::KeySpec;
use crate::keymap_editor::catalog::common;
use crate::keymap_editor::draft::EditorSection;
use crate::keymap_editor::picker::CandidateGroup;
use crate::keymap_editor::profile::{EditorProfile, SidebarSection};
use crate::layout_key::LayoutKey;

const QMK_SIDEBAR_SECTIONS: [SidebarSection<EditorSection>; 6] = [
    SidebarSection {
        title: "Keys",
        items: &[EditorSection::Keyboard],
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
        let usages = super::codec::qmk_all_basic_usages();
        let candidates = common::build_keyboard_candidates(&QmkKeyPresenter, usages, |id| {
            super::codec::qmk_search_tokens_for_hid(0x07, id)
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
        let usages = super::codec::qmk_all_media_usages();
        let candidates = common::build_media_candidates(&QmkKeyPresenter, usages, |id| {
            super::codec::qmk_search_tokens_for_hid(0x0C, id)
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

fn qmk_system_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        vec![common::build_system_group(
            &QmkKeyPresenter,
            |id| match id {
                0x81 => &["KC_SYSTEM_POWER"][..],
                0x82 => &["KC_SYSTEM_SLEEP"][..],
                0x83 => &["KC_SYSTEM_WAKE"][..],
                _ => &[][..],
            },
        )]
    })
}

const QMK_BOOT_POWER_ACTIONS: [(crate::key_spec::PowerAction, &[&str]); 3] = [
    (crate::key_spec::PowerAction::Reset, &["reset", "reboot"]),
    (
        crate::key_spec::PowerAction::Bootloader,
        &["bootloader", "dfu", "flash", "boot"],
    ),
    (
        crate::key_spec::PowerAction::Other(0xEE),
        &["clear eeprom", "eeprom reset"],
    ),
];

fn qmk_boot_power_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::PowerAction;
        vec![common::build_boot_power_group(
            &QmkKeyPresenter,
            &QMK_BOOT_POWER_ACTIONS,
            |act| -> &'static [&'static str] {
                match act {
                    PowerAction::Reset => &["QK_BOOT", "QK_REBOOT"],
                    PowerAction::Bootloader => &["QK_BOOTLOADER"],
                    PowerAction::Other(0xEE) => &["QK_CLEAR_EEPROM"],
                    _ => &[],
                }
            },
        )]
    })
}

fn qmk_backlight_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::BacklightAction;
        vec![common::build_backlight_group(
            &QmkKeyPresenter,
            |act| -> &'static [&'static str] {
                match act {
                    BacklightAction::Toggle => &["BL_TOGG", "QK_BACKLIGHT_TOGGLE"],
                    BacklightAction::On => &["BL_ON", "QK_BACKLIGHT_ON"],
                    BacklightAction::Off => &["BL_OFF", "QK_BACKLIGHT_OFF"],
                    BacklightAction::Inc => &["BL_UP", "QK_BACKLIGHT_UP"],
                    BacklightAction::Dec => &["BL_DOWN", "QK_BACKLIGHT_DOWN"],
                    BacklightAction::Cycle => &["BL_STEP", "QK_BACKLIGHT_STEP"],
                    BacklightAction::BreathingToggle => {
                        &["BL_BRTG", "QK_BACKLIGHT_TOGGLE_BREATHING"]
                    }
                    _ => &[],
                }
            },
        )]
    })
}

fn qmk_rgb_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::RgbAction;
        vec![common::build_rgb_underglow_group(
            &QmkKeyPresenter,
            |act| -> &'static [&'static str] {
                match act {
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
                }
            },
        )]
    })
}

fn qmk_rgb_matrix_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::RgbMatrixAction;
        vec![common::build_rgb_matrix_group(
            &QmkKeyPresenter,
            |act| -> &'static [&'static str] {
                match act {
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
                }
            },
        )]
    })
}

fn qmk_audio_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::AudioAction;
        vec![common::build_audio_group(
            &QmkKeyPresenter,
            |act| -> &'static [&'static str] {
                match act {
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
                }
            },
        )]
    })
}

fn qmk_mouse_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        use crate::key_spec::{MouseAction, MouseButton};
        common::build_mouse_groups(&QmkKeyPresenter, true, |spec| -> &'static [&'static str] {
            match spec {
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
            }
        })
    })
}

fn qmk_special_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        vec![common::build_special_group(
            &QmkKeyPresenter,
            |spec| -> &'static [&'static str] {
                match spec {
                    KeySpec::Transparent => &["KC_TRNS"],
                    KeySpec::None => &["KC_NO"],
                    KeySpec::CapsWord => &["QK_CAPS_WORD_TOGGLE"],
                    KeySpec::KeyRepeat => &["QK_KEY_REPEAT"],
                    KeySpec::GraveEscape => &["QK_GRAVE_ESCAPE"],
                    _ => &[],
                }
            },
        )]
    })
}

fn qmk_custom_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| common::build_custom_groups(&QmkKeyPresenter))
}

const QMK_LAYER_OPS: [(&str, crate::key_spec::LayerActivation, &[&str]); 6] = [
    (
        "Momentary",
        crate::key_spec::LayerActivation::Momentary,
        &["mo", "momentary"],
    ),
    (
        "Toggle",
        crate::key_spec::LayerActivation::Toggle,
        &["tg", "toggle"],
    ),
    (
        "Switch To Layer",
        crate::key_spec::LayerActivation::To,
        &["to", "switch"],
    ),
    (
        "Sticky Layer",
        crate::key_spec::LayerActivation::Sticky,
        &["sl", "sticky", "oneshot"],
    ),
    (
        "Set Default Layer",
        crate::key_spec::LayerActivation::Default,
        &["df", "default"],
    ),
    (
        "Tap Toggle",
        crate::key_spec::LayerActivation::TapToggle,
        &["tt", "tap toggle"],
    ),
];

impl KeyPresenter for QmkEditorProfile {
    fn present_key(&self, spec: &KeySpec, layer_names: &[String]) -> Option<LayoutKey> {
        QmkKeyPresenter.present_key(spec, layer_names)
    }
}

impl EditorProfile for QmkEditorProfile {
    fn presenter(&self) -> &dyn KeyPresenter {
        self
    }

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

    fn tap_categories(&self) -> &'static [CandidateGroup] {
        qmk_tap_categories()
    }

    fn section_groups(&self, section: EditorSection) -> &'static [CandidateGroup] {
        match section {
            EditorSection::Keyboard
            | EditorSection::Combo
            | EditorSection::ModTap
            | EditorSection::OneShot => qmk_tap_categories(),
            EditorSection::System => qmk_system_groups(),
            EditorSection::BootPower => qmk_boot_power_groups(),
            EditorSection::Backlight => qmk_backlight_groups(),
            EditorSection::Rgb => qmk_rgb_groups(),
            EditorSection::RgbMatrix => qmk_rgb_matrix_groups(),
            EditorSection::Audio => qmk_audio_groups(),
            EditorSection::Mouse => qmk_mouse_groups(),
            EditorSection::Special => qmk_special_groups(),
            EditorSection::Custom => qmk_custom_groups(),
            EditorSection::KeyToggle
            | EditorSection::Bluetooth
            | EditorSection::Output
            | EditorSection::Layers
            | EditorSection::LayerMod
            | EditorSection::RawHex => &[],
        }
    }

    fn layer_groups(
        &self,
        layer_count: usize,
        layer_names: &[String],
        tap: crate::keymap_editor::LayerTapTarget,
    ) -> Vec<CandidateGroup> {
        common::build_layer_groups(
            &QmkKeyPresenter,
            layer_count,
            layer_names,
            tap,
            &QMK_LAYER_OPS,
            |_| &[],
        )
    }

    fn parse_raw_keycode(&self, raw: &str) -> Option<KeySpec> {
        let code = u16::from_str_radix(raw, 16).ok()?;
        Some(super::codec::qmk_to_keyspec(code))
    }
}
