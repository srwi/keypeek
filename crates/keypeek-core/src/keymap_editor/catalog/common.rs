//! Common candidate key builders for keymap editor profiles.
//!
//! Provides builders for key candidate groups represented as [`KeySpec`] objects.

use crate::key_presenter::KeyPresenter;
use crate::key_spec::Modifiers;
use crate::key_spec::{
    AudioAction, BacklightAction, BluetoothAction, CustomBinding, CustomKind, HidKey, KeySpec,
    LayerActivation, LightingAction, MouseAction, MouseButton, OutputTarget, PowerAction,
    RgbAction, RgbMatrixAction,
};
use crate::keymap_editor::candidate::{Candidate, CandidateGroup};

/// Creates a candidate from a [`KeySpec`] and attaches search tokens.
pub fn action_candidate(spec: KeySpec, names: &[&str], presenter: &dyn KeyPresenter) -> Candidate {
    Candidate::from_action(spec, presenter, &[]).with_search_tokens(names.iter().copied())
}

/// Attaches common search aliases to keyboard usages.
pub fn attach_friendly_keyboard_aliases(cand: Candidate, id: u16) -> Candidate {
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
    cand.with_search_tokens(aliases.iter().copied())
}

/// Attaches standard friendly aliases (e.g. "mute", "volup", "play") to consumer usages.
pub fn attach_friendly_media_aliases(cand: Candidate, id: u16) -> Candidate {
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
    cand.with_search_tokens(aliases.iter().copied())
}

/// Builds candidates for keyboard usages (USB HID Page 0x07).
pub fn build_keyboard_candidates(
    presenter: &dyn KeyPresenter,
    usages: impl IntoIterator<Item = u16>,
    token_fn: impl Fn(u16) -> Vec<String>,
) -> Vec<Candidate> {
    usages
        .into_iter()
        .map(|id| {
            let action = KeySpec::KeyPress {
                key: HidKey::keyboard(id),
                modifiers: Modifiers::default(),
            };
            let mut cand = Candidate::from_action(action, presenter, &[]);
            if cand.key.symbol.is_none() && cand.key.tap.is_empty() {
                cand.key.symbol = Some(format!("0x{:02X}", id));
            }
            cand = cand.with_search_tokens(token_fn(id));
            attach_friendly_keyboard_aliases(cand, id)
        })
        .collect()
}

/// Builds candidates for consumer/media usages (USB HID Page 0x0C).
pub fn build_media_candidates(
    presenter: &dyn KeyPresenter,
    usages: impl IntoIterator<Item = u16>,
    token_fn: impl Fn(u16) -> Vec<String>,
) -> Vec<Candidate> {
    usages
        .into_iter()
        .map(|id| {
            let action = KeySpec::KeyPress {
                key: HidKey::consumer(id),
                modifiers: Modifiers::default(),
            };
            let mut cand = Candidate::from_action(action, presenter, &[]);
            if cand.key.symbol.is_none() && cand.key.tap.is_empty() {
                cand.key.symbol = Some(format!("0x{:04X}", id));
            }
            cand = cand.with_search_tokens(token_fn(id));
            attach_friendly_media_aliases(cand, id)
        })
        .collect()
}

/// Builds the Bluetooth candidate group with optional extra firmware-specific tokens.
pub fn build_bluetooth_group(
    presenter: &dyn KeyPresenter,
    extra_tokens_fn: impl Fn(BluetoothAction) -> &'static [&'static str],
) -> CandidateGroup {
    let mut actions = vec![
        (
            BluetoothAction::Clear,
            &["bt clear", "bt clr", "disconnect"][..],
        ),
        (BluetoothAction::Next, &["bt next", "bt nxt"][..]),
        (BluetoothAction::Prev, &["bt prev", "bt prv"][..]),
        (
            BluetoothAction::ClearAll,
            &["bt clear all", "bt clr all"][..],
        ),
    ];

    for i in 0..=9 {
        actions.push((BluetoothAction::Select(i), &["bt sel"][..]));
    }
    for i in 0..=9 {
        actions.push((BluetoothAction::Disconnect(i), &["bt disc"][..]));
    }

    let candidates = actions
        .into_iter()
        .map(|(action, names)| {
            let mut cand = action_candidate(KeySpec::Bluetooth(action), names, presenter)
                .with_search_tokens(extra_tokens_fn(action).iter().copied());
            match action {
                BluetoothAction::Select(n) => {
                    cand = cand
                        .with_search_token(format!("bt {n}"))
                        .with_search_token(format!("bt sel {n}"));
                }
                BluetoothAction::Disconnect(n) => {
                    cand = cand.with_search_token(format!("bt disc {n}"));
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
}

/// Builds the Output candidate group with optional extra firmware-specific tokens.
pub fn build_output_group(
    presenter: &dyn KeyPresenter,
    extra_tokens_fn: impl Fn(OutputTarget) -> &'static [&'static str],
) -> CandidateGroup {
    let targets = [
        (OutputTarget::Toggle, &["output toggle", "out tog"][..]),
        (OutputTarget::Usb, &["output usb", "out usb"][..]),
        (OutputTarget::Ble, &["output ble", "out ble"][..]),
        (OutputTarget::None, &["output none", "out none"][..]),
    ];

    let candidates = targets
        .into_iter()
        .map(|(target, names)| {
            action_candidate(KeySpec::Output(target), names, presenter)
                .with_search_tokens(extra_tokens_fn(target).iter().copied())
        })
        .collect();

    CandidateGroup {
        name: "Output",
        candidates,
    }
}

/// Builds the System candidate group with optional extra firmware-specific tokens.
pub fn build_system_group(
    presenter: &dyn KeyPresenter,
    extra_tokens_fn: impl Fn(u16) -> &'static [&'static str],
) -> CandidateGroup {
    let sys_keys = [
        (
            HidKey::system(0x81),
            &["system power", "sys power", "power down"][..],
        ),
        (
            HidKey::system(0x82),
            &["system sleep", "sys sleep", "sleep"][..],
        ),
        (
            HidKey::system(0x83),
            &["system wake", "sys wake", "wake"][..],
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
                presenter,
            )
            .with_search_tokens(extra_tokens_fn(key.id).iter().copied())
        })
        .collect();

    CandidateGroup {
        name: "System",
        candidates,
    }
}

/// Builds the Boot & Power candidate group with optional extra firmware-specific tokens.
pub fn build_boot_power_group(
    presenter: &dyn KeyPresenter,
    actions: &[(PowerAction, &[&str])],
    extra_tokens_fn: impl Fn(PowerAction) -> &'static [&'static str],
) -> CandidateGroup {
    let candidates = actions
        .iter()
        .map(|(action, names)| {
            action_candidate(KeySpec::Power(*action), names, presenter)
                .with_search_tokens(extra_tokens_fn(*action).iter().copied())
        })
        .collect();

    CandidateGroup {
        name: "Boot & Power",
        candidates,
    }
}

/// Builds the Backlight candidate group with optional extra firmware-specific tokens.
pub fn build_backlight_group(
    presenter: &dyn KeyPresenter,
    extra_tokens_fn: impl Fn(BacklightAction) -> &'static [&'static str],
) -> CandidateGroup {
    let bl_actions = [
        (BacklightAction::Toggle, &["bl toggle", "backlight"][..]),
        (BacklightAction::On, &["bl on", "backlight on"][..]),
        (BacklightAction::Off, &["bl off", "backlight off"][..]),
        (BacklightAction::Inc, &["bl inc", "backlight up"][..]),
        (BacklightAction::Dec, &["bl dec", "backlight down"][..]),
        (BacklightAction::Cycle, &["bl step", "backlight cycle"][..]),
        (
            BacklightAction::BreathingToggle,
            &["bl breath", "bl breathing"][..],
        ),
    ];

    let candidates = bl_actions
        .into_iter()
        .map(|(action, names)| {
            action_candidate(
                KeySpec::Lighting(LightingAction::Backlight(action)),
                names,
                presenter,
            )
            .with_search_tokens(extra_tokens_fn(action).iter().copied())
        })
        .collect();

    CandidateGroup {
        name: "Backlight",
        candidates,
    }
}

/// Builds the RGB Underglow candidate group with optional extra firmware-specific tokens.
pub fn build_rgb_underglow_group(
    presenter: &dyn KeyPresenter,
    extra_tokens_fn: impl Fn(RgbAction) -> &'static [&'static str],
) -> CandidateGroup {
    let rgb_actions = [
        (RgbAction::Toggle, &["rgb toggle", "rgb tog"][..]),
        (RgbAction::On, &["rgb on", "underglow on"][..]),
        (RgbAction::Off, &["rgb off", "underglow off"][..]),
        (RgbAction::EffectInc, &["rgb mode", "rgb next"][..]),
        (RgbAction::EffectDec, &["rgb prev"][..]),
        (RgbAction::HueInc, &["rgb hue+", "hue up"][..]),
        (RgbAction::HueDec, &["rgb hue-", "hue down"][..]),
        (RgbAction::SatInc, &["rgb sat+", "sat up"][..]),
        (RgbAction::SatDec, &["rgb sat-", "sat down"][..]),
        (RgbAction::BrightInc, &["rgb val+", "bri up"][..]),
        (RgbAction::BrightDec, &["rgb val-", "bri down"][..]),
        (RgbAction::SpeedInc, &["rgb speed+", "spd up"][..]),
        (RgbAction::SpeedDec, &["rgb speed-", "spd down"][..]),
        (RgbAction::EffectSet, &["rgb effect set", "eff set"][..]),
        (RgbAction::Color, &["rgb color", "color"][..]),
    ];

    let candidates = rgb_actions
        .into_iter()
        .map(|(action, names)| {
            action_candidate(
                KeySpec::Lighting(LightingAction::Rgb(action)),
                names,
                presenter,
            )
            .with_search_tokens(extra_tokens_fn(action).iter().copied())
        })
        .collect();

    CandidateGroup {
        name: "RGB Underglow",
        candidates,
    }
}

/// Builds the RGB Matrix candidate group with optional extra firmware-specific tokens.
pub fn build_rgb_matrix_group(
    presenter: &dyn KeyPresenter,
    extra_tokens_fn: impl Fn(RgbMatrixAction) -> &'static [&'static str],
) -> CandidateGroup {
    let actions = [
        (
            RgbMatrixAction::Toggle,
            &["rgb matrix toggle", "rgb_tog"][..],
        ),
        (
            RgbMatrixAction::ModeNext,
            &["rgb matrix next", "rgb_mod"][..],
        ),
        (
            RgbMatrixAction::ModePrev,
            &["rgb matrix prev", "rgb_rmod"][..],
        ),
        (RgbMatrixAction::HueInc, &["rgb matrix hue+", "rgb_hui"][..]),
        (RgbMatrixAction::HueDec, &["rgb matrix hue-", "rgb_hud"][..]),
        (RgbMatrixAction::SatInc, &["rgb matrix sat+", "rgb_sai"][..]),
        (RgbMatrixAction::SatDec, &["rgb matrix sat-", "rgb_sad"][..]),
        (
            RgbMatrixAction::BrightInc,
            &["rgb matrix val+", "rgb_vai"][..],
        ),
        (
            RgbMatrixAction::BrightDec,
            &["rgb matrix val-", "rgb_vad"][..],
        ),
        (
            RgbMatrixAction::SpeedInc,
            &["rgb matrix speed+", "rgb_spi"][..],
        ),
        (
            RgbMatrixAction::SpeedDec,
            &["rgb matrix speed-", "rgb_spd"][..],
        ),
    ];
    let candidates = actions
        .into_iter()
        .map(|(act, names)| {
            action_candidate(
                KeySpec::Lighting(LightingAction::RgbMatrix(act)),
                names,
                presenter,
            )
            .with_search_tokens(extra_tokens_fn(act).iter().copied())
        })
        .collect();
    CandidateGroup {
        name: "RGB Matrix",
        candidates,
    }
}

/// Builds the Audio candidate group with optional extra firmware-specific tokens.
pub fn build_audio_group(
    presenter: &dyn KeyPresenter,
    extra_tokens_fn: impl Fn(AudioAction) -> &'static [&'static str],
) -> CandidateGroup {
    let actions = [
        (AudioAction::On, &["audio on", "au_on"][..]),
        (AudioAction::Off, &["audio off", "au_off"][..]),
        (AudioAction::Toggle, &["audio toggle", "au_tog"][..]),
        (AudioAction::ClickyToggle, &["clicky toggle", "ck_tog"][..]),
        (AudioAction::ClickyOn, &["clicky on", "ck_on"][..]),
        (AudioAction::ClickyOff, &["clicky off", "ck_off"][..]),
        (AudioAction::ClickyUp, &["clicky up", "ck_up"][..]),
        (AudioAction::ClickyDown, &["clicky down", "ck_down"][..]),
        (AudioAction::ClickyReset, &["clicky reset", "ck_rst"][..]),
        (AudioAction::MusicOn, &["music on", "mu_on"][..]),
        (AudioAction::MusicOff, &["music off", "mu_off"][..]),
        (AudioAction::MusicToggle, &["music toggle", "mu_tog"][..]),
        (AudioAction::MusicModeNext, &["music mode", "mu_mod"][..]),
        (AudioAction::VoiceNext, &["audio voice+", "voice next"][..]),
        (AudioAction::VoicePrev, &["audio voice-", "voice prev"][..]),
    ];
    let candidates = actions
        .into_iter()
        .map(|(act, names)| {
            action_candidate(KeySpec::Audio(act), names, presenter)
                .with_search_tokens(extra_tokens_fn(act).iter().copied())
        })
        .collect();
    CandidateGroup {
        name: "Audio",
        candidates,
    }
}

/// Builds the Mouse candidate groups (Buttons, Movement, Scroll, and optionally Acceleration).
pub fn build_mouse_groups(
    presenter: &dyn KeyPresenter,
    include_acceleration: bool,
    extra_tokens_fn: impl Fn(&KeySpec) -> &'static [&'static str],
) -> Vec<CandidateGroup> {
    let buttons = [
        (MouseButton::Left, &["mouse left", "btn1", "left click"][..]),
        (
            MouseButton::Right,
            &["mouse right", "btn2", "right click"][..],
        ),
        (
            MouseButton::Middle,
            &["mouse middle", "btn3", "middle click"][..],
        ),
        (MouseButton::Button4, &["mouse btn4"][..]),
        (MouseButton::Button5, &["mouse btn5"][..]),
        (MouseButton::Other(6), &["mouse btn6"][..]),
        (MouseButton::Other(7), &["mouse btn7"][..]),
        (MouseButton::Other(8), &["mouse btn8"][..]),
    ];

    let button_candidates = buttons
        .into_iter()
        .map(|(btn, names)| {
            let spec = KeySpec::Mouse(MouseAction::Press(btn));
            let extra = extra_tokens_fn(&spec);
            action_candidate(spec, names, presenter).with_search_tokens(extra.iter().copied())
        })
        .collect();

    let move_directions = [
        (
            MouseAction::Move { x: 0, y: -1 },
            &["mouse up", "move up", "ms up"][..],
        ),
        (
            MouseAction::Move { x: 0, y: 1 },
            &["mouse down", "move down", "ms down"][..],
        ),
        (
            MouseAction::Move { x: -1, y: 0 },
            &["mouse left", "move left", "ms left"][..],
        ),
        (
            MouseAction::Move { x: 1, y: 0 },
            &["mouse right", "move right", "ms right"][..],
        ),
    ];

    let move_candidates = move_directions
        .into_iter()
        .map(|(action, names)| {
            let spec = KeySpec::Mouse(action);
            let extra = extra_tokens_fn(&spec);
            action_candidate(spec, names, presenter).with_search_tokens(extra.iter().copied())
        })
        .collect();

    let scroll_directions = [
        (
            MouseAction::Scroll { x: 0, y: 1 },
            &["scroll up", "wh up"][..],
        ),
        (
            MouseAction::Scroll { x: 0, y: -1 },
            &["scroll down", "wh down"][..],
        ),
        (
            MouseAction::Scroll { x: -1, y: 0 },
            &["scroll left", "wh left"][..],
        ),
        (
            MouseAction::Scroll { x: 1, y: 0 },
            &["scroll right", "wh right"][..],
        ),
    ];

    let scroll_candidates = scroll_directions
        .into_iter()
        .map(|(action, names)| {
            let spec = KeySpec::Mouse(action);
            let extra = extra_tokens_fn(&spec);
            action_candidate(spec, names, presenter).with_search_tokens(extra.iter().copied())
        })
        .collect();

    let mut groups = vec![
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
    ];

    if include_acceleration {
        let accels = [
            (0u8, &["mouse accel 0", "accel 0"][..]),
            (1u8, &["mouse accel 1", "accel 1"][..]),
            (2u8, &["mouse accel 2", "accel 2"][..]),
        ];

        let accel_candidates = accels
            .into_iter()
            .map(|(level, names)| {
                let spec = KeySpec::Mouse(MouseAction::Acceleration(level));
                let extra = extra_tokens_fn(&spec);
                action_candidate(spec, names, presenter).with_search_tokens(extra.iter().copied())
            })
            .collect();

        groups.push(CandidateGroup {
            name: "Mouse Acceleration",
            candidates: accel_candidates,
        });
    }

    groups
}

/// Builds the Special candidate group with optional extra firmware-specific tokens.
pub fn build_special_group(
    presenter: &dyn KeyPresenter,
    extra_tokens_fn: impl Fn(&KeySpec) -> &'static [&'static str],
) -> CandidateGroup {
    let actions: [(KeySpec, &[&str]); 5] = [
        (KeySpec::Transparent, &["transparent", "trans", "pass"]),
        (KeySpec::None, &["none", "noop", "unbound"]),
        (KeySpec::CapsWord, &["caps word", "caps_word", "cw"]),
        (KeySpec::KeyRepeat, &["key repeat", "key_repeat", "repeat"]),
        (KeySpec::GraveEscape, &["grave escape", "grave_esc"]),
    ];

    let candidates = actions
        .into_iter()
        .map(|(spec, names)| {
            let extra = extra_tokens_fn(&spec);
            action_candidate(spec, names, presenter).with_search_tokens(extra.iter().copied())
        })
        .collect();

    CandidateGroup {
        name: "Special",
        candidates,
    }
}

/// Builds candidate groups for user macros and firmware extensions.
pub fn build_custom_groups(presenter: &dyn KeyPresenter) -> Vec<CandidateGroup> {
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
                    Candidate::from_action(spec, presenter, &[])
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
}

/// Candidate groups for supported layer operation types across real layers.
pub fn build_layer_groups(
    presenter: &dyn KeyPresenter,
    layer_count: usize,
    layer_names: &[String],
    tap: crate::keymap_editor::LayerTapTarget,
    ops: &[(&'static str, LayerActivation, &'static [&'static str])],
    extra_tokens_fn: impl Fn(LayerActivation) -> &'static [&'static str],
) -> Vec<CandidateGroup> {
    let count = layer_count.min(32);

    let mut groups: Vec<CandidateGroup> = ops
        .iter()
        .map(|(name, activation, search_tokens)| {
            let candidates = (0..count)
                .map(|layer| {
                    Candidate::from_action(
                        KeySpec::Layer {
                            layer: layer as u8,
                            activation: *activation,
                        },
                        presenter,
                        layer_names,
                    )
                    .with_search_tokens(*search_tokens)
                    .with_search_tokens(extra_tokens_fn(*activation).iter().copied())
                    .with_search_token(format!("l{layer}"))
                })
                .collect();

            CandidateGroup { name, candidates }
        })
        .collect();

    let tap_key = tap.key.unwrap_or_else(|| HidKey::keyboard(0x2C));
    let lt_candidates = (0..count)
        .map(|layer| {
            Candidate::from_action(
                KeySpec::LayerTap {
                    layer: layer as u8,
                    tap: tap_key,
                    tap_modifiers: tap.modifiers,
                },
                presenter,
                layer_names,
            )
            .with_search_tokens(["lt", "layer tap"])
            .with_search_token(format!("lt{layer}"))
            .with_search_token(format!("l{layer}"))
        })
        .collect();

    groups.push(CandidateGroup {
        name: "Layer Tap",
        candidates: lt_candidates,
    });

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPresenter;
    impl KeyPresenter for TestPresenter {
        fn present_key(
            &self,
            _spec: &KeySpec,
            _layer_names: &[String],
        ) -> Option<crate::layout_key::LayoutKey> {
            Some(crate::layout_key::LayoutKey::default())
        }
    }

    #[test]
    fn common_keyboard_candidates_produce_expected_specs() {
        let cands = build_keyboard_candidates(&TestPresenter, vec![0x04, 0x28], |_| {
            vec!["token".to_string()]
        });
        assert_eq!(cands.len(), 2);
        assert_eq!(
            cands[0].binding,
            KeySpec::KeyPress {
                key: HidKey::keyboard(0x04),
                modifiers: Modifiers::default(),
            }
        );
        assert!(cands[0].matches_query("token"));
        // Enter should get friendly alias
        assert!(cands[1].matches_query("enter"));
    }

    #[test]
    fn common_bluetooth_candidates_produce_bluetooth_specs() {
        let group = build_bluetooth_group(&TestPresenter, |_| &["test_extra"]);
        assert_eq!(group.name, "Bluetooth");
        assert!(group
            .candidates
            .iter()
            .any(|c| matches!(c.binding, KeySpec::Bluetooth(BluetoothAction::Clear))));
        assert!(group.candidates[0].matches_query("test_extra"));
    }
}
