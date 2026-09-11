//! Unified candidate key categories and groups for the keymap editor.
//!
//! Generates candidate keys represented as pure [`KeySpec`] domain objects with
//! attached search tokens (including standard names and QMK/ZMK aliases).

use super::picker::{Candidate, CandidateGroup};
use crate::hid_labels::Modifiers;
use crate::key_spec::{
    AudioAction, BacklightAction, BluetoothAction, CustomBinding, CustomKind, HidKey, KeySpec,
    LayerActivation, LayerInfo, LightingAction, MouseAction, MouseButton, OutputTarget,
    PowerAction, RgbAction, RgbMatrixAction,
};
use std::sync::OnceLock;

/// Standard typing and navigation keys (USB HID Usage Page 0x07).
pub fn keyboard_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let usages = crate::protocols::all_keyboard_usages();
        let candidates = usages
            .into_iter()
            .map(|id| {
                let action = KeySpec::KeyPress {
                    key: HidKey::keyboard(id),
                    modifiers: Modifiers::default(),
                };
                let mut cand = Candidate::from_action(action, &[]);
                if cand.key.symbol.is_none() && cand.key.tap.is_empty() {
                    cand.key.symbol = Some(format!("0x{:02X}", id));
                }
                cand = cand.with_search_token(format!("{:04x}", id));
                for token in crate::protocols::protocol_search_tokens_for_hid(0x07, id) {
                    cand = cand.with_search_token(token);
                }
                attach_friendly_keyboard_aliases(cand, id)
            })
            .collect();

        CandidateGroup {
            name: "Keyboard",
            candidates,
        }
    })
}

fn attach_friendly_keyboard_aliases(mut cand: Candidate, id: u16) -> Candidate {
    let aliases: &[&str] = match id {
        0x28 => &["enter", "return"],
        0x29 => &["esc", "escape"],
        0x2A => &["backspace", "bspc"],
        0x2B => &["tab"],
        0x2C => &["space", "spc"],
        0x2D => &["minus"],
        0x2E => &["equal"],
        0x2F | 0x30 => &["bracket"],
        0x31 => &["backslash"],
        0x33 => &["semicolon"],
        0x34 => &["quote"],
        0x35 => &["grave", "tilde"],
        0x36 => &["comma"],
        0x37 => &["dot", "period"],
        0x38 => &["slash"],
        0x39 => &["caps", "capslock"],
        0x46 => &["print", "printscreen", "prtsc"],
        0x47 => &["scroll", "scrolllock"],
        0x48 => &["pause", "break"],
        0x49 => &["insert", "ins"],
        0x4A => &["home"],
        0x4B => &["pageup", "pgup"],
        0x4C => &["delete", "del"],
        0x4D => &["end"],
        0x4E => &["pagedown", "pgdn"],
        0x4F => &["right"],
        0x50 => &["left"],
        0x51 => &["down"],
        0x52 => &["up"],
        0x53..=0x67 => &["keypad"],
        0xE0 | 0xE4 => &["ctrl", "control"],
        0xE1 | 0xE5 => &["shift"],
        0xE2 | 0xE6 => &["alt"],
        0xE3 | 0xE7 => &["gui", "win", "cmd", "super"],
        _ => &[],
    };
    for &alias in aliases {
        cand = cand.with_search_token(alias);
    }
    cand
}

/// Media and consumer controls (USB HID Usage Page 0x0C).
pub fn media_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let usages = crate::protocols::all_consumer_usages();
        let candidates = usages
            .into_iter()
            .map(|id| {
                let action = KeySpec::KeyPress {
                    key: HidKey::consumer(id),
                    modifiers: Modifiers::default(),
                };
                let mut cand = Candidate::from_action(action, &[]);
                if cand.key.symbol.is_none() && cand.key.tap.is_empty() {
                    cand.key.symbol = Some(format!("0x{:04X}", id));
                }
                cand = cand.with_search_token(format!("{:04x}", id));
                for token in crate::protocols::protocol_search_tokens_for_hid(0x0C, id) {
                    cand = cand.with_search_token(token);
                }
                attach_friendly_media_aliases(cand, id)
            })
            .collect();

        CandidateGroup {
            name: "Media",
            candidates,
        }
    })
}

fn attach_friendly_media_aliases(mut cand: Candidate, id: u16) -> Candidate {
    let aliases: &[&str] = match id {
        0xE2 => &["mute", "audio"],
        0xE9 => &["volume", "volup"],
        0xEA => &["volume", "voldn"],
        0xCD => &["play", "pause"],
        0xB5 => &["next", "track"],
        0xB6 => &["prev", "previous", "track"],
        0xB7 => &["stop"],
        0xB8 => &["eject"],
        0x192 => &["calc", "calculator"],
        0x18A => &["mail", "email"],
        0x221 => &["search", "browser"],
        0x223 => &["home", "browser"],
        0x224 => &["back", "browser"],
        0x225 => &["forward", "browser"],
        0x226 => &["stop", "browser"],
        0x227 => &["refresh", "browser"],
        0x6F => &["brightness", "briup"],
        0x70 => &["brightness", "bridn"],
        _ => &[],
    };
    for &alias in aliases {
        cand = cand.with_search_token(alias);
    }
    cand
}

/// Candidate groups suitable for tap targets (e.g. Mod-Tap and Layer-Tap).
pub fn tap_categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(|| vec![keyboard_group().clone(), media_group().clone()])
}

/// Bluetooth wireless connection controls.
pub fn bluetooth_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let mut actions = vec![
            (
                BluetoothAction::Clear,
                &["bt clear", "bt clr", "disconnect", "bt_clr"][..],
            ),
            (BluetoothAction::Next, &["bt next", "bt nxt", "bt_nxt"][..]),
            (BluetoothAction::Prev, &["bt prev", "bt prv", "bt_prv"][..]),
            (
                BluetoothAction::ClearAll,
                &["bt clear all", "bt clr all", "bt_clr_all"][..],
            ),
        ];

        for i in 0..=9 {
            actions.push((BluetoothAction::Select(i), &["bt sel", "bt_sel"][..]));
        }
        for i in 0..=9 {
            actions.push((BluetoothAction::Disconnect(i), &["bt disc", "bt_disc"][..]));
        }

        let candidates = actions
            .into_iter()
            .map(|(action, names)| {
                let mut cand = Candidate::from_action(KeySpec::Bluetooth(action), &[]);
                for name in names {
                    cand = cand.with_search_token(*name);
                }
                match action {
                    BluetoothAction::Select(n) => {
                        cand = cand
                            .with_search_token(format!("bt {n}"))
                            .with_search_token(format!("bt sel {n}"))
                            .with_search_token(format!("bt_sel_{n}"));
                    }
                    BluetoothAction::Disconnect(n) => {
                        cand = cand
                            .with_search_token(format!("bt disc {n}"))
                            .with_search_token(format!("bt_disc_{n}"));
                    }
                    _ => {}
                }
                cand
            })
            .collect();

        CandidateGroup {
            name: "Bluetooth",
            candidates,
        }
    })
}

/// Output target selection (USB / Bluetooth).
pub fn output_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let targets = [
            (
                OutputTarget::Toggle,
                &["output toggle", "out tog", "out_tog"][..],
            ),
            (OutputTarget::Usb, &["output usb", "out usb", "out_usb"][..]),
            (OutputTarget::Ble, &["output ble", "out ble", "out_ble"][..]),
            (
                OutputTarget::None,
                &["output none", "out none", "out_none"][..],
            ),
        ];

        let candidates = targets
            .into_iter()
            .map(|(target, names)| action_candidate(KeySpec::Output(target), names))
            .collect();

        CandidateGroup {
            name: "Output",
            candidates,
        }
    })
}

/// HID system controls (USB HID Usage Page 0x0F... system page keys).
pub fn system_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let sys_keys = [
            (
                HidKey::system(0x81),
                &["system power", "sys power", "power down", "KC_SYSTEM_POWER"][..],
            ),
            (
                HidKey::system(0x82),
                &["system sleep", "sys sleep", "sleep", "KC_SYSTEM_SLEEP"][..],
            ),
            (
                HidKey::system(0x83),
                &["system wake", "sys wake", "wake", "KC_SYSTEM_WAKE"][..],
            ),
        ];
        let candidates = sys_keys
            .into_iter()
            .map(|(key, names)| {
                action_candidate(
                    KeySpec::KeyPress {
                        key,
                        modifiers: Modifiers::default(),
                    },
                    names,
                )
            })
            .collect();

        CandidateGroup {
            name: "System",
            candidates,
        }
    })
}

/// Boot, reset, and power-control actions.
pub fn boot_power_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let actions = [
            (
                PowerAction::Reset,
                &["reset", "reboot", "sys_reset", "QK_BOOT"][..],
            ),
            (
                PowerAction::Bootloader,
                &["bootloader", "dfu", "flash", "boot"][..],
            ),
            (
                PowerAction::SoftOff,
                &["soft off", "power off", "shutdown"][..],
            ),
            (
                PowerAction::UnlockKeymap,
                &["unlock", "keymap unlock", "studio unlock"][..],
            ),
            (PowerAction::Toggle, &["ext pwr tog", "power toggle"][..]),
            (PowerAction::On, &["ext pwr on", "power on"][..]),
            (PowerAction::Off, &["ext pwr off", "power off"][..]),
        ];

        let candidates = actions
            .into_iter()
            .map(|(action, names)| action_candidate(KeySpec::Power(action), names))
            .collect();

        CandidateGroup {
            name: "Boot & Power",
            candidates,
        }
    })
}

/// Lighting controls (Backlight & RGB Underglow/Matrix).
pub fn lighting_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        let bl_actions = [
            (
                BacklightAction::Toggle,
                &["bl toggle", "backlight", "BL_TOGG", "QK_BACKLIGHT_TOGGLE"][..],
            ),
            (
                BacklightAction::On,
                &["bl on", "backlight on", "BL_ON", "QK_BACKLIGHT_ON"][..],
            ),
            (
                BacklightAction::Off,
                &["bl off", "backlight off", "BL_OFF", "QK_BACKLIGHT_OFF"][..],
            ),
            (
                BacklightAction::Inc,
                &["bl inc", "backlight up", "BL_UP", "QK_BACKLIGHT_UP"][..],
            ),
            (
                BacklightAction::Dec,
                &["bl dec", "backlight down", "BL_DOWN", "QK_BACKLIGHT_DOWN"][..],
            ),
            (
                BacklightAction::Cycle,
                &["bl step", "backlight cycle", "BL_STEP", "QK_BACKLIGHT_STEP"][..],
            ),
            (
                BacklightAction::BreathingToggle,
                &[
                    "bl breath",
                    "bl breathing",
                    "BL_BRTG",
                    "QK_BACKLIGHT_TOGGLE_BREATHING",
                ][..],
            ),
        ];

        let backlight_candidates = bl_actions
            .into_iter()
            .map(|(action, names)| {
                action_candidate(KeySpec::Lighting(LightingAction::Backlight(action)), names)
            })
            .collect();

        let rgb_actions = [
            (
                RgbAction::Toggle,
                &["rgb toggle", "rgb tog", "RGB_TOG", "QK_UNDERGLOW_TOGGLE"][..],
            ),
            (RgbAction::On, &["rgb on", "underglow on", "RGB_ON"][..]),
            (RgbAction::Off, &["rgb off", "underglow off", "RGB_OFF"][..]),
            (
                RgbAction::EffectInc,
                &["rgb mode", "rgb next", "RGB_MOD", "QK_UNDERGLOW_MODE_NEXT"][..],
            ),
            (
                RgbAction::EffectDec,
                &["rgb prev", "RGB_RMOD", "QK_UNDERGLOW_MODE_PREVIOUS"][..],
            ),
            (
                RgbAction::HueInc,
                &["rgb hue+", "hue up", "RGB_HUI", "QK_UNDERGLOW_HUE_UP"][..],
            ),
            (
                RgbAction::HueDec,
                &["rgb hue-", "hue down", "RGB_HUD", "QK_UNDERGLOW_HUE_DOWN"][..],
            ),
            (
                RgbAction::SatInc,
                &[
                    "rgb sat+",
                    "sat up",
                    "RGB_SAI",
                    "QK_UNDERGLOW_SATURATION_UP",
                ][..],
            ),
            (
                RgbAction::SatDec,
                &[
                    "rgb sat-",
                    "sat down",
                    "RGB_SAD",
                    "QK_UNDERGLOW_SATURATION_DOWN",
                ][..],
            ),
            (
                RgbAction::BrightInc,
                &["rgb val+", "bri up", "RGB_VAI", "QK_UNDERGLOW_VALUE_UP"][..],
            ),
            (
                RgbAction::BrightDec,
                &["rgb val-", "bri down", "RGB_VAD", "QK_UNDERGLOW_VALUE_DOWN"][..],
            ),
            (
                RgbAction::SpeedInc,
                &["rgb speed+", "spd up", "RGB_SPI", "QK_UNDERGLOW_SPEED_UP"][..],
            ),
            (
                RgbAction::SpeedDec,
                &[
                    "rgb speed-",
                    "spd down",
                    "RGB_SPD",
                    "QK_UNDERGLOW_SPEED_DOWN",
                ][..],
            ),
            (RgbAction::EffectSet, &["rgb effect set", "eff set"][..]),
            (RgbAction::Color, &["rgb color", "color"][..]),
        ];

        let rgb_candidates: Vec<Candidate> = rgb_actions
            .into_iter()
            .map(|(action, names)| {
                action_candidate(KeySpec::Lighting(LightingAction::Rgb(action)), names)
            })
            .collect();

        vec![
            CandidateGroup {
                name: "Backlight",
                candidates: backlight_candidates,
            },
            CandidateGroup {
                name: "RGB Underglow",
                candidates: rgb_candidates,
            },
        ]
    })
}

/// RGB Matrix per-key lighting controls.
pub fn rgb_matrix_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let actions = [
            (
                RgbMatrixAction::Toggle,
                &["rgb matrix toggle", "rgb_tog", "RGB_MATRIX_TOGGLE"][..],
            ),
            (
                RgbMatrixAction::ModeNext,
                &["rgb matrix next", "rgb_mod", "RGB_MATRIX_MODE_NEXT"][..],
            ),
            (
                RgbMatrixAction::ModePrev,
                &["rgb matrix prev", "rgb_rmod", "RGB_MATRIX_MODE_PREVIOUS"][..],
            ),
            (
                RgbMatrixAction::HueInc,
                &["rgb matrix hue+", "rgb_hui", "RGB_MATRIX_HUE_UP"][..],
            ),
            (
                RgbMatrixAction::HueDec,
                &["rgb matrix hue-", "rgb_hud", "RGB_MATRIX_HUE_DOWN"][..],
            ),
            (
                RgbMatrixAction::SatInc,
                &["rgb matrix sat+", "rgb_sai", "RGB_MATRIX_SATURATION_UP"][..],
            ),
            (
                RgbMatrixAction::SatDec,
                &["rgb matrix sat-", "rgb_sad", "RGB_MATRIX_SATURATION_DOWN"][..],
            ),
            (
                RgbMatrixAction::BrightInc,
                &["rgb matrix val+", "rgb_vai", "RGB_MATRIX_VALUE_UP"][..],
            ),
            (
                RgbMatrixAction::BrightDec,
                &["rgb matrix val-", "rgb_vad", "RGB_MATRIX_VALUE_DOWN"][..],
            ),
            (
                RgbMatrixAction::SpeedInc,
                &["rgb matrix speed+", "rgb_spi", "RGB_MATRIX_SPEED_UP"][..],
            ),
            (
                RgbMatrixAction::SpeedDec,
                &["rgb matrix speed-", "rgb_spd", "RGB_MATRIX_SPEED_DOWN"][..],
            ),
        ];
        let candidates = actions
            .into_iter()
            .map(|(act, names)| {
                action_candidate(KeySpec::Lighting(LightingAction::RgbMatrix(act)), names)
            })
            .collect();
        CandidateGroup {
            name: "RGB Matrix",
            candidates,
        }
    })
}

/// Audio and sound synthesizer controls.
pub fn audio_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let actions = [
            (AudioAction::On, &["audio on", "au_on", "QK_AUDIO_ON"][..]),
            (
                AudioAction::Off,
                &["audio off", "au_off", "QK_AUDIO_OFF"][..],
            ),
            (
                AudioAction::Toggle,
                &["audio toggle", "au_tog", "QK_AUDIO_TOGGLE"][..],
            ),
            (
                AudioAction::ClickyToggle,
                &["clicky toggle", "ck_tog", "QK_AUDIO_CLICKY_TOGGLE"][..],
            ),
            (
                AudioAction::ClickyOn,
                &["clicky on", "ck_on", "QK_AUDIO_CLICKY_ON"][..],
            ),
            (
                AudioAction::ClickyOff,
                &["clicky off", "ck_off", "QK_AUDIO_CLICKY_OFF"][..],
            ),
            (
                AudioAction::ClickyUp,
                &["clicky up", "ck_up", "QK_AUDIO_CLICKY_UP"][..],
            ),
            (
                AudioAction::ClickyDown,
                &["clicky down", "ck_down", "QK_AUDIO_CLICKY_DOWN"][..],
            ),
            (
                AudioAction::ClickyReset,
                &["clicky reset", "ck_rst", "QK_AUDIO_CLICKY_RESET"][..],
            ),
            (
                AudioAction::MusicOn,
                &["music on", "mu_on", "QK_MUSIC_ON"][..],
            ),
            (
                AudioAction::MusicOff,
                &["music off", "mu_off", "QK_MUSIC_OFF"][..],
            ),
            (
                AudioAction::MusicToggle,
                &["music toggle", "mu_tog", "QK_MUSIC_TOGGLE"][..],
            ),
            (
                AudioAction::MusicModeNext,
                &["music mode", "mu_mod", "QK_MUSIC_MODE_NEXT"][..],
            ),
            (
                AudioAction::VoiceNext,
                &["audio voice+", "voice next", "QK_AUDIO_VOICE_NEXT"][..],
            ),
            (
                AudioAction::VoicePrev,
                &["audio voice-", "voice prev", "QK_AUDIO_VOICE_PREVIOUS"][..],
            ),
        ];
        let candidates = actions
            .into_iter()
            .map(|(act, names)| action_candidate(KeySpec::Audio(act), names))
            .collect();
        CandidateGroup {
            name: "Audio",
            candidates,
        }
    })
}

/// Mouse button and movement controls.
pub fn mouse_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        let buttons = [
            (
                MouseButton::Left,
                &["mouse left", "btn1", "left click", "MS_BTN1"][..],
            ),
            (
                MouseButton::Right,
                &["mouse right", "btn2", "right click", "MS_BTN2"][..],
            ),
            (
                MouseButton::Middle,
                &["mouse middle", "btn3", "middle click", "MS_BTN3"][..],
            ),
            (MouseButton::Button4, &["mouse btn4", "MS_BTN4"][..]),
            (MouseButton::Button5, &["mouse btn5", "MS_BTN5"][..]),
            (MouseButton::Other(6), &["mouse btn6", "MS_BTN6"][..]),
            (MouseButton::Other(7), &["mouse btn7", "MS_BTN7"][..]),
            (MouseButton::Other(8), &["mouse btn8", "MS_BTN8"][..]),
        ];

        let button_candidates = buttons
            .into_iter()
            .map(|(btn, names)| action_candidate(KeySpec::Mouse(MouseAction::Press(btn)), names))
            .collect();

        let move_directions = [
            (
                MouseAction::Move { x: 0, y: -1 },
                &["mouse up", "move up", "ms up", "MS_UP"][..],
            ),
            (
                MouseAction::Move { x: 0, y: 1 },
                &["mouse down", "move down", "ms down", "MS_DOWN"][..],
            ),
            (
                MouseAction::Move { x: -1, y: 0 },
                &["mouse left", "move left", "ms left", "MS_LEFT"][..],
            ),
            (
                MouseAction::Move { x: 1, y: 0 },
                &["mouse right", "move right", "ms right", "MS_RIGHT"][..],
            ),
        ];

        let move_candidates = move_directions
            .into_iter()
            .map(|(action, names)| action_candidate(KeySpec::Mouse(action), names))
            .collect();

        let scroll_directions = [
            (
                MouseAction::Scroll { x: 0, y: 1 },
                &["scroll up", "wh up", "MS_WH_UP"][..],
            ),
            (
                MouseAction::Scroll { x: 0, y: -1 },
                &["scroll down", "wh down", "MS_WH_DOWN"][..],
            ),
            (
                MouseAction::Scroll { x: -1, y: 0 },
                &["scroll left", "wh left", "MS_WH_LEFT"][..],
            ),
            (
                MouseAction::Scroll { x: 1, y: 0 },
                &["scroll right", "wh right", "MS_WH_RIGHT"][..],
            ),
        ];

        let scroll_candidates = scroll_directions
            .into_iter()
            .map(|(action, names)| action_candidate(KeySpec::Mouse(action), names))
            .collect();

        let accels = [
            (0u8, &["mouse accel 0", "accel 0", "MS_ACL0"][..]),
            (1u8, &["mouse accel 1", "accel 1", "MS_ACL1"][..]),
            (2u8, &["mouse accel 2", "accel 2", "MS_ACL2"][..]),
        ];

        let accel_candidates = accels
            .into_iter()
            .map(|(level, names)| {
                action_candidate(KeySpec::Mouse(MouseAction::Acceleration(level)), names)
            })
            .collect();

        vec![
            CandidateGroup {
                name: "Mouse Buttons",
                candidates: button_candidates,
            },
            CandidateGroup {
                name: "Mouse Movement",
                candidates: move_candidates,
            },
            CandidateGroup {
                name: "Mouse Scroll",
                candidates: scroll_candidates,
            },
            CandidateGroup {
                name: "Mouse Acceleration",
                candidates: accel_candidates,
            },
        ]
    })
}

/// Special typing and navigation actions.
pub fn special_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let candidates = vec![
            action_candidate(
                KeySpec::Transparent,
                &["transparent", "trans", "pass", "KC_TRNS"],
            ),
            action_candidate(KeySpec::None, &["none", "noop", "unbound", "KC_NO"]),
            action_candidate(
                KeySpec::CapsWord,
                &["caps word", "caps_word", "cw", "QK_CAPS_WORD_TOGGLE"],
            ),
            action_candidate(
                KeySpec::KeyRepeat,
                &["key repeat", "key_repeat", "repeat", "QK_KEY_REPEAT"],
            ),
            action_candidate(
                KeySpec::GraveEscape,
                &["grave escape", "grave_esc", "QK_GRAVE_ESCAPE"],
            ),
        ];

        CandidateGroup {
            name: "Special",
            candidates,
        }
    })
}

/// Candidate groups for user macros and firmware extensions.
pub fn custom_groups() -> &'static [CandidateGroup] {
    static GROUPS: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        let kinds = [
            ("Macro", CustomKind::Macro),
            ("Tap Dance", CustomKind::TapDance),
            ("User", CustomKind::User),
            ("Keyboard", CustomKind::Keyboard),
        ];

        kinds
            .into_iter()
            .map(|(title, kind)| {
                let candidates = (0..32)
                    .map(|i| {
                        let spec = KeySpec::Custom(CustomBinding {
                            kind,
                            id: i,
                            name: None,
                            param1: None,
                            param2: None,
                        });
                        Candidate::from_action(spec, &[])
                            .with_search_token(title)
                            .with_search_token(format!("{title} {i}"))
                    })
                    .collect();

                CandidateGroup {
                    name: title,
                    candidates,
                }
            })
            .collect()
    })
}

/// Candidate groups for all supported layer operation types across real layers.
pub fn layer_groups(
    layer_count: usize,
    layer_infos: &[LayerInfo],
    layer_names: &[String],
    tap_key: Option<HidKey>,
) -> Vec<CandidateGroup> {
    let count = layer_count.min(layer_infos.len().max(layer_count)).min(32);
    let ops = [
        (
            "Momentary",
            LayerActivation::Momentary,
            &["mo", "momentary"][..],
        ),
        (
            "Toggle",
            LayerActivation::Toggle,
            &["tg", "toggle"][..],
        ),
        ("Switch To Layer", LayerActivation::To, &["to", "switch"][..]),
        (
            "Sticky Layer",
            LayerActivation::Sticky,
            &["sl", "sticky", "oneshot"][..],
        ),
        (
            "Set Default Layer",
            LayerActivation::Default,
            &["df", "default"][..],
        ),
        (
            "Tap Toggle",
            LayerActivation::TapToggle,
            &["tt", "tap toggle"][..],
        ),
    ];

    let mut groups: Vec<CandidateGroup> = ops
        .into_iter()
        .map(|(name, activation, search_tokens)| {
            let candidates = (0..count)
                .map(|layer| {
                    let mut cand = Candidate::from_action(
                        KeySpec::Layer {
                            layer: layer as u8,
                            activation,
                        },
                        layer_names,
                    );
                    for token in search_tokens {
                        cand = cand.with_search_token(token);
                    }
                    cand = cand.with_search_token(format!("l{layer}"));
                    cand
                })
                .collect();

            CandidateGroup { name, candidates }
        })
        .collect();

    let tap = tap_key.unwrap_or_else(|| HidKey::keyboard(0x2C));
    let lt_candidates = (0..count)
        .map(|layer| {
            let mut cand = Candidate::from_action(
                KeySpec::LayerTap {
                    layer: layer as u8,
                    tap,
                    tap_modifiers: Modifiers::default(),
                },
                layer_names,
            );
            cand = cand.with_search_token("lt");
            cand = cand.with_search_token("layer tap");
            cand = cand.with_search_token(format!("lt{layer}"));
            cand = cand.with_search_token(format!("l{layer}"));
            cand
        })
        .collect();

    groups.push(CandidateGroup {
        name: "Layer Tap",
        candidates: lt_candidates,
    });

    groups
}

fn action_candidate(spec: KeySpec, names: &[&str]) -> Candidate {
    let mut cand = Candidate::from_action(spec, &[]);
    for name in names {
        cand = cand.with_search_token(*name);
    }
    cand
}
