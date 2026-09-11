//! Unified candidate key categories and groups for the keymap editor.
//!
//! Generates candidate keys represented as pure [`KeySpec`] domain objects with
//! attached search tokens (including standard names and QMK/ZMK aliases).

use super::picker::{Candidate, CandidateGroup};
use crate::hid_labels::Modifiers;
use crate::key_spec::{
    BacklightAction, BluetoothAction, CustomBinding, CustomKind, HidKey, KeySpec, LayerActivation,
    LayerInfo, LightingAction, MouseAction, MouseButton, OutputTarget, PowerAction, RgbAction,
};
use std::sync::OnceLock;

/// Standard typing and navigation keys (USB HID Usage Page 0x07).
pub fn keyboard_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let mut candidates = Vec::with_capacity(140);

        // Letters A-Z: 0x04..=0x1D
        for id in 0x04..=0x1D {
            let letter = (b'A' + (id - 0x04) as u8) as char;
            let kc_name = format!("KC_{letter}");
            let cand = hid_candidate(0x07, id).with_search_token(kc_name);
            candidates.push(cand);
        }

        // Digits 1-9, 0: 0x1E..=0x27
        for id in 0x1E..=0x27 {
            let digit = if id == 0x27 {
                '0'
            } else {
                (b'1' + (id - 0x1E) as u8) as char
            };
            let kc_name = format!("KC_{digit}");
            let cand = hid_candidate(0x07, id).with_search_token(kc_name);
            candidates.push(cand);
        }

        // Return, Esc, Backspace, Tab, Space: 0x28..=0x2C
        candidates.push(
            hid_candidate(0x07, 0x28)
                .with_search_token("enter")
                .with_search_token("return")
                .with_search_token("KC_ENT")
                .with_search_token("KC_ENTER"),
        );
        candidates.push(
            hid_candidate(0x07, 0x29)
                .with_search_token("esc")
                .with_search_token("escape")
                .with_search_token("KC_ESC")
                .with_search_token("KC_ESCAPE"),
        );
        candidates.push(
            hid_candidate(0x07, 0x2A)
                .with_search_token("backspace")
                .with_search_token("bspc")
                .with_search_token("KC_BSPC")
                .with_search_token("KC_BACKSPACE"),
        );
        candidates.push(
            hid_candidate(0x07, 0x2B)
                .with_search_token("tab")
                .with_search_token("KC_TAB"),
        );
        candidates.push(
            hid_candidate(0x07, 0x2C)
                .with_search_token("space")
                .with_search_token("spc")
                .with_search_token("KC_SPC")
                .with_search_token("KC_SPACE"),
        );

        // Punctuation & symbols: 0x2D..=0x38
        for (id, names) in [
            (0x2D, &["minus", "KC_MINS", "KC_MINUS"][..]),
            (0x2E, &["equal", "KC_EQL", "KC_EQUAL"][..]),
            (0x2F, &["bracket", "KC_LBRC", "KC_LEFT_BRACKET"][..]),
            (0x30, &["bracket", "KC_RBRC", "KC_RIGHT_BRACKET"][..]),
            (0x31, &["backslash", "KC_BSLS", "KC_BACKSLASH"][..]),
            (0x32, &["nonus_hash", "KC_NUHS", "KC_NONUS_HASH"][..]),
            (0x33, &["semicolon", "KC_SCLN", "KC_SEMICOLON"][..]),
            (0x34, &["quote", "KC_QUOT", "KC_QUOTE"][..]),
            (0x35, &["grave", "tilde", "KC_GRV", "KC_GRAVE"][..]),
            (0x36, &["comma", "KC_COMM", "KC_COMMA"][..]),
            (0x37, &["dot", "period", "KC_DOT"][..]),
            (0x38, &["slash", "KC_SLSH", "KC_SLASH"][..]),
        ] {
            let mut cand = hid_candidate(0x07, id);
            for name in names {
                cand = cand.with_search_token(name);
            }
            candidates.push(cand);
        }

        // CapsLock: 0x39
        candidates.push(
            hid_candidate(0x07, 0x39)
                .with_search_token("caps")
                .with_search_token("capslock")
                .with_search_token("KC_CAPS")
                .with_search_token("KC_CAPS_LOCK"),
        );

        // Function keys F1..=F12: 0x3A..=0x45
        for id in 0x3A..=0x45 {
            let num = id - 0x3A + 1;
            let cand = hid_candidate(0x07, id)
                .with_search_token(format!("f{num}"))
                .with_search_token(format!("KC_F{num}"));
            candidates.push(cand);
        }

        // Navigation & control: 0x46..=0x52
        for (id, names) in [
            (
                0x46,
                &[
                    "print",
                    "printscreen",
                    "prtsc",
                    "KC_PSCR",
                    "KC_PRINT_SCREEN",
                ][..],
            ),
            (
                0x47,
                &["scroll", "scrolllock", "KC_SCRL", "KC_SCROLL_LOCK"][..],
            ),
            (0x48, &["pause", "break", "KC_PAUS", "KC_PAUSE"][..]),
            (0x49, &["insert", "ins", "KC_INS", "KC_INSERT"][..]),
            (0x4A, &["home", "KC_HOME"][..]),
            (0x4B, &["pageup", "pgup", "KC_PGUP", "KC_PAGE_UP"][..]),
            (0x4C, &["delete", "del", "KC_DEL", "KC_DELETE"][..]),
            (0x4D, &["end", "KC_END"][..]),
            (0x4E, &["pagedown", "pgdn", "KC_PGDN", "KC_PAGE_DOWN"][..]),
            (0x4F, &["right", "KC_RGHT", "KC_RIGHT"][..]),
            (0x50, &["left", "KC_LEFT"][..]),
            (0x51, &["down", "KC_DOWN"][..]),
            (0x52, &["up", "KC_UP"][..]),
        ] {
            let mut cand = hid_candidate(0x07, id);
            for name in names {
                cand = cand.with_search_token(name);
            }
            candidates.push(cand);
        }

        // Keypad: 0x53..=0x67
        for id in 0x53..=0x67 {
            candidates.push(hid_candidate(0x07, id).with_search_token("keypad"));
        }

        // F13..=F24: 0x68..=0x73
        for id in 0x68..=0x73 {
            let num = id - 0x68 + 13;
            let cand = hid_candidate(0x07, id)
                .with_search_token(format!("f{num}"))
                .with_search_token(format!("KC_F{num}"));
            candidates.push(cand);
        }

        // Modifiers: 0xE0..=0xE7
        for (id, names) in [
            (0xE0, &["ctrl", "lctrl", "KC_LCTL", "KC_LEFT_CTRL"][..]),
            (0xE1, &["shift", "lshift", "KC_LSFT", "KC_LEFT_SHIFT"][..]),
            (0xE2, &["alt", "lalt", "KC_LALT", "KC_LEFT_ALT"][..]),
            (
                0xE3,
                &["gui", "win", "cmd", "lgui", "KC_LGUI", "KC_LEFT_GUI"][..],
            ),
            (0xE4, &["ctrl", "rctrl", "KC_RCTL", "KC_RIGHT_CTRL"][..]),
            (0xE5, &["shift", "rshift", "KC_RSFT", "KC_RIGHT_SHIFT"][..]),
            (0xE6, &["alt", "ralt", "KC_RALT", "KC_RIGHT_ALT"][..]),
            (
                0xE7,
                &["gui", "win", "cmd", "rgui", "KC_RGUI", "KC_RIGHT_GUI"][..],
            ),
        ] {
            let mut cand = hid_candidate(0x07, id);
            for name in names {
                cand = cand.with_search_token(name);
            }
            candidates.push(cand);
        }

        CandidateGroup {
            name: "Keyboard",
            candidates,
        }
    })
}

/// Media and consumer controls (USB HID Usage Page 0x0C).
pub fn media_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let items: &[(u16, &[&str])] = &[
            (
                0xE2,
                &["mute", "audio", "KC_MUTE", "KC_AUDIO_MUTE", "C_MUTE"],
            ),
            (
                0xE9,
                &[
                    "volume",
                    "volup",
                    "KC_VOLU",
                    "KC_AUDIO_VOL_UP",
                    "C_VOL_UP",
                    "C_VOLUME_UP",
                ],
            ),
            (
                0xEA,
                &[
                    "volume",
                    "voldn",
                    "KC_VOLD",
                    "KC_AUDIO_VOL_DOWN",
                    "C_VOL_DN",
                    "C_VOLUME_DOWN",
                ],
            ),
            (
                0xCD,
                &[
                    "play",
                    "pause",
                    "KC_MPLY",
                    "KC_MEDIA_PLAY_PAUSE",
                    "C_PLAY_PAUSE",
                    "C_PP",
                ],
            ),
            (
                0xB5,
                &["next", "track", "KC_MNXT", "KC_MEDIA_NEXT_TRACK", "C_NEXT"],
            ),
            (
                0xB6,
                &[
                    "prev",
                    "previous",
                    "track",
                    "KC_MPRV",
                    "KC_MEDIA_PREV_TRACK",
                    "C_PREV",
                ],
            ),
            (0xB7, &["stop", "KC_MSTP", "KC_MEDIA_STOP", "C_STOP"]),
            (
                0xB8,
                &["eject", "KC_EJCT", "KC_MEDIA_EJECT", "C_MEDIA_EJECT"],
            ),
            (0x192, &["calc", "calculator", "KC_CALC", "C_CALCULATOR"]),
            (0x18A, &["mail", "email", "KC_MAIL", "C_EMAIL"]),
            (
                0x221,
                &[
                    "search",
                    "browser",
                    "KC_WSCH",
                    "KC_WWW_SEARCH",
                    "C_AC_SEARCH",
                ],
            ),
            (
                0x223,
                &["home", "browser", "KC_WHOM", "KC_WWW_HOME", "C_AC_HOME"],
            ),
            (
                0x224,
                &["back", "browser", "KC_WBAK", "KC_WWW_BACK", "C_AC_BACK"],
            ),
            (
                0x225,
                &[
                    "forward",
                    "browser",
                    "KC_WFWD",
                    "KC_WWW_FORWARD",
                    "C_AC_FORWARD",
                ],
            ),
            (
                0x226,
                &["stop", "browser", "KC_WSTP", "KC_WWW_STOP", "C_AC_STOP"],
            ),
            (
                0x227,
                &[
                    "refresh",
                    "browser",
                    "KC_WREF",
                    "KC_WWW_REFRESH",
                    "C_AC_REFRESH",
                ],
            ),
            (
                0x6F,
                &["brightness", "briup", "KC_BRIU", "KC_BRIGHTNESS_UP"],
            ),
            (
                0x70,
                &["brightness", "bridn", "KC_BRID", "KC_BRIGHTNESS_DOWN"],
            ),
        ];

        let candidates = items
            .iter()
            .map(|&(id, names)| {
                let mut cand = hid_candidate(0x0C, id);
                for name in names {
                    cand = cand.with_search_token(name);
                }
                cand
            })
            .collect();

        CandidateGroup {
            name: "Media",
            candidates,
        }
    })
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
        let actions = [
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
            (
                BluetoothAction::Select(0),
                &["bt 0", "bt sel 0", "bt_sel_0"][..],
            ),
            (
                BluetoothAction::Select(1),
                &["bt 1", "bt sel 1", "bt_sel_1"][..],
            ),
            (
                BluetoothAction::Select(2),
                &["bt 2", "bt sel 2", "bt_sel_2"][..],
            ),
            (
                BluetoothAction::Select(3),
                &["bt 3", "bt sel 3", "bt_sel_3"][..],
            ),
            (
                BluetoothAction::Select(4),
                &["bt 4", "bt sel 4", "bt_sel_4"][..],
            ),
            (
                BluetoothAction::Disconnect(0),
                &["bt disc 0", "bt_disc_0"][..],
            ),
            (
                BluetoothAction::Disconnect(1),
                &["bt disc 1", "bt_disc_1"][..],
            ),
            (
                BluetoothAction::Disconnect(2),
                &["bt disc 2", "bt_disc_2"][..],
            ),
        ];

        let candidates = actions
            .into_iter()
            .map(|(action, names)| action_candidate(KeySpec::Bluetooth(action), names))
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

/// Hardware power and system controls.
pub fn system_group() -> &'static CandidateGroup {
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
                PowerAction::StudioUnlock,
                &["unlock", "studio unlock", "studio_unlock"][..],
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
            name: "System",
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
        ];

        let rgb_candidates = rgb_actions
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
                name: "RGB Lighting",
                candidates: rgb_candidates,
            },
        ]
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
        ]
    })
}

/// Special typing and navigation actions.
pub fn special_group() -> &'static CandidateGroup {
    static GROUP: OnceLock<CandidateGroup> = OnceLock::new();
    GROUP.get_or_init(|| {
        let actions = [
            (
                KeySpec::Transparent,
                &["transparent", "trans", "pass", "KC_TRNS"][..],
            ),
            (KeySpec::None, &["none", "noop", "unbound", "KC_NO"][..]),
            (
                KeySpec::CapsWord,
                &["caps word", "caps_word", "cw", "QK_CAPS_WORD_TOGGLE"][..],
            ),
            (
                KeySpec::KeyRepeat,
                &["key repeat", "key_repeat", "repeat", "QK_KEY_REPEAT"][..],
            ),
            (
                KeySpec::GraveEscape,
                &["grave escape", "grave_esc", "QK_GRAVE_ESCAPE"][..],
            ),
        ];

        let candidates = actions
            .into_iter()
            .map(|(spec, names)| action_candidate(spec, names))
            .collect();

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
            "Momentary (MO)",
            LayerActivation::Momentary,
            &["mo", "momentary"][..],
        ),
        (
            "Toggle (TG)",
            LayerActivation::Toggle,
            &["tg", "toggle"][..],
        ),
        ("To Layer (TO)", LayerActivation::To, &["to", "switch"][..]),
        (
            "Sticky (SL)",
            LayerActivation::Sticky,
            &["sl", "sticky", "oneshot"][..],
        ),
        (
            "Default (DF)",
            LayerActivation::Default,
            &["df", "default"][..],
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
        name: "Layer Tap (LT)",
        candidates: lt_candidates,
    });

    groups
}

fn hid_candidate(page: u16, id: u16) -> Candidate {
    use std::fmt::Write;
    let spec = KeySpec::KeyPress {
        key: HidKey::new(page, id),
        modifiers: Modifiers::default(),
    };
    let mut cand = Candidate::from_action(spec, &[]);
    let mut hex = String::with_capacity(5);
    let _ = write!(&mut hex, "{:04x}", id);
    cand = cand.with_search_token(hex);
    cand
}

fn action_candidate(spec: KeySpec, names: &[&str]) -> Candidate {
    let mut cand = Candidate::from_action(spec, &[]);
    for name in names {
        cand = cand.with_search_token(*name);
    }
    cand
}
