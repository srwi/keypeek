//! Key presentation layer. Converts [`KeySpec`] domain objects into visual [`LayoutKey`] items.

pub mod builders;
pub use builders::*;

use keypeek_core::{
    behavior_names, AudioAction, BacklightAction, BluetoothAction, BorderStyle, CustomKind,
    CustomParam, KeyPresenter, KeySpec, KeycodeKind, Label, LayerActivation, LayoutKey,
    LightingAction, Modifiers, MouseAction, MouseButton, OutputTarget, PowerAction, RgbAction,
    RgbMatrixAction,
};

/// Standard key presenter for generic HID keys, modifiers, and hardware controls.
#[derive(Debug, Clone, Copy, Default)]
pub struct StandardKeyPresenter;

impl KeyPresenter for StandardKeyPresenter {
    fn present_key(&self, spec: &KeySpec, layer_names: &[String]) -> Option<LayoutKey> {
        present_standard_key(spec, layer_names)
    }
}

/// Standard presentation logic for key specifications.
pub fn present_standard_key(spec: &KeySpec, layer_names: &[String]) -> Option<LayoutKey> {
    match spec {
        KeySpec::Transparent => None,

        KeySpec::None => Some(LayoutKey {
            tap: Label::new(""),
            ..Default::default()
        }),

        KeySpec::KeyPress { key, modifiers } => {
            let base = crate::hid_labels::hid_usage_to_layout_key(key.page, key.id);
            if modifiers.is_empty() {
                base.or_else(|| {
                    Some(LayoutKey {
                        tap: Label::new(format!("0x{:04X}", key.id)),
                        ..Default::default()
                    })
                })
            } else {
                Some(mod_combo_key(key.page, key.id, *modifiers, base))
            }
        }

        KeySpec::KeyToggle { key, modifiers } => {
            let base = crate::hid_labels::hid_usage_to_layout_key(key.page, key.id);
            let mut layout = if modifiers.is_empty() {
                base.unwrap_or_else(|| LayoutKey {
                    tap: Label::new(format!("0x{:04X}", key.id)),
                    ..Default::default()
                })
            } else {
                mod_combo_key(key.page, key.id, *modifiers, base)
            };
            layout.behavior = Some(behavior_names::KEY_TOGGLE.label());
            Some(layout)
        }

        KeySpec::LayerTap {
            layer,
            tap,
            tap_modifiers,
        } => {
            let base = crate::hid_labels::hid_usage_to_layout_key(tap.page, tap.id);
            let tap_key = if tap_modifiers.is_empty() {
                base.unwrap_or_default()
            } else {
                mod_combo_key(tap.page, tap.id, *tap_modifiers, base)
            };
            Some(layer_tap_key(*layer, tap_key, None))
        }

        KeySpec::ModTap {
            hold,
            tap,
            tap_modifiers,
        } => {
            let base = crate::hid_labels::hid_usage_to_layout_key(tap.page, tap.id);
            let tap_key = if tap_modifiers.is_empty() {
                base.unwrap_or_default()
            } else {
                mod_combo_key(tap.page, tap.id, *tap_modifiers, base)
            };
            let mask = hold.to_held_mod_mask();
            Some(mod_tap_key(
                tap_key,
                hold.label(),
                (mask != 0).then_some(mask),
                Some(behavior_names::MOD_TAP.label()),
            ))
        }

        KeySpec::Layer { layer, activation } => {
            let (border, argument) = match activation {
                LayerActivation::Momentary | LayerActivation::TapToggle => {
                    (BorderStyle::None, None)
                }
                LayerActivation::Toggle | LayerActivation::To => (BorderStyle::Solid, None),
                LayerActivation::Sticky => (BorderStyle::Dashed, None),
                LayerActivation::Default => (BorderStyle::Solid, None),
                LayerActivation::LayerMod(mods) => {
                    (BorderStyle::None, (!mods.is_empty()).then(|| mods.label()))
                }
            };
            let name_label = layer_names
                .get(*layer as usize)
                .filter(|n| !n.is_empty())
                .map(|n| Label::new(n.as_str()))
                .unwrap_or_else(|| Label::new(format!("L{}", layer)));
            let mut key = layer_switch_key(*layer, name_label, border);
            if let LayerActivation::LayerMod(mods) = activation {
                key.argument = argument;
                let mask = mods.to_held_mod_mask();
                key.mod_mask = (mask != 0).then_some(mask);
            } else if matches!(activation, LayerActivation::Default) {
                key.layer_ref = None;
            }
            Some(key)
        }

        KeySpec::StickyKey { key, modifiers } => match key {
            Some(k) => {
                let mut layout =
                    crate::hid_labels::hid_usage_to_layout_key(k.page, k.id).unwrap_or_default();
                layout.behavior = Some(behavior_names::STICKY_KEY.label());
                layout.kind = KeycodeKind::Modifier;
                Some(layout)
            }
            None => {
                let mask = modifiers.to_held_mod_mask();
                Some(one_shot_mod_key(
                    modifiers.label(),
                    (mask != 0).then_some(mask),
                    Some(behavior_names::ONE_SHOT_MOD.label()),
                ))
            }
        },

        KeySpec::CapsWord => Some(LayoutKey {
            tap: Label::with_short("Caps Word", "CW"),
            ..Default::default()
        }),

        KeySpec::KeyRepeat => Some(LayoutKey {
            tap: Label::with_short("Key Repeat", "Rep"),
            ..Default::default()
        }),

        KeySpec::GraveEscape => Some(LayoutKey {
            tap: Label::with_short("Grave Esc", "G/E"),
            ..Default::default()
        }),

        KeySpec::Bluetooth(cmd) => {
            let label = match cmd {
                BluetoothAction::Clear => Label::new("BT Clr"),
                BluetoothAction::Next => Label::new("BT Nxt"),
                BluetoothAction::Prev => Label::new("BT Prv"),
                BluetoothAction::Select(n) => {
                    Label::with_short(format!("BT Sel {n}"), format!("BT{n}"))
                }
                BluetoothAction::ClearAll => Label::with_short("BT Clr All", "BTClr"),
                BluetoothAction::Disconnect(n) => {
                    Label::with_short(format!("BT Disc {n}"), format!("BTD{n}"))
                }
                BluetoothAction::Other { command, value: 0 } => Label::new(format!("BT {command}")),
                BluetoothAction::Other { command, value } => {
                    Label::new(format!("BT {command} {value}"))
                }
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }

        KeySpec::Output(out) => {
            let label = match out {
                OutputTarget::Toggle => Label::with_short("Out Tog", "OutTg"),
                OutputTarget::Usb => Label::new("Out USB"),
                OutputTarget::Ble => Label::new("Out BLE"),
                OutputTarget::None => Label::with_short("Out None", "OutNo"),
                OutputTarget::Other(n) => Label::new(format!("Out {n}")),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }

        KeySpec::Power(pwr) => {
            let label = match pwr {
                PowerAction::Off => Label::with_short("ExtPwr Off", "EPOff"),
                PowerAction::On => Label::with_short("ExtPwr On", "EPOn"),
                PowerAction::Toggle => Label::with_short("ExtPwr Tog", "EPTog"),
                PowerAction::SoftOff => Label::with_short("Soft Off", "Off"),
                PowerAction::Reset => Label::with_short("Reset", "Rst"),
                PowerAction::Bootloader => Label::with_short("Bootloader", "Boot"),
                PowerAction::UnlockKeymap => Label::with_short("Studio Unlock", "Unlock"),
                PowerAction::Other(n) => Label::with_short(format!("ExtPwr {n}"), format!("EP{n}")),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }

        KeySpec::Lighting(lighting) => {
            let label = match lighting {
                LightingAction::Backlight(bl) => match bl {
                    BacklightAction::On => Label::new("BL On"),
                    BacklightAction::Off => Label::new("BL Off"),
                    BacklightAction::Toggle => Label::with_short("BL Toggle", "BLTog"),
                    BacklightAction::Inc => Label::with_short("BL Inc", "BL+"),
                    BacklightAction::Dec => Label::with_short("BL Dec", "BL-"),
                    BacklightAction::Cycle => Label::with_short("BL Cycle", "BLCyc"),
                    BacklightAction::BreathingToggle => Label::with_short("BL Breath", "BLBrt"),
                    BacklightAction::Set(n) => {
                        Label::with_short(format!("BL Set {n}"), format!("BL{n}"))
                    }
                    BacklightAction::Other { command, value: 0 } => {
                        Label::new(format!("BL {command}"))
                    }
                    BacklightAction::Other { command, value } => {
                        Label::new(format!("BL {command} {value}"))
                    }
                },
                LightingAction::Rgb(ug) => match ug {
                    RgbAction::Toggle => Label::with_short("RGB Toggle", "RGBTg"),
                    RgbAction::On => Label::with_short("RGB On", "RGBOn"),
                    RgbAction::Off => Label::with_short("RGB Off", "RGBOff"),
                    RgbAction::HueInc => Label::with_short("Hue +", "Hue+"),
                    RgbAction::HueDec => Label::with_short("Hue -", "Hue-"),
                    RgbAction::SatInc => Label::with_short("Sat +", "Sat+"),
                    RgbAction::SatDec => Label::with_short("Sat -", "Sat-"),
                    RgbAction::BrightInc => Label::with_short("Bright +", "Bri+"),
                    RgbAction::BrightDec => Label::with_short("Bright -", "Bri-"),
                    RgbAction::SpeedInc => Label::with_short("Speed +", "Spd+"),
                    RgbAction::SpeedDec => Label::with_short("Speed -", "Spd-"),
                    RgbAction::EffectInc => Label::with_short("Effect +", "Eff+"),
                    RgbAction::EffectDec => Label::with_short("Effect -", "Eff-"),
                    RgbAction::EffectSet => Label::with_short("Effect Set", "EffS"),
                    RgbAction::Color => Label::with_short("RGB Color", "Color"),
                    RgbAction::Other { command, value: 0 } => Label::new(format!("RGB {command}")),
                    RgbAction::Other { command, value } => {
                        Label::new(format!("RGB {command} {value}"))
                    }
                },
                LightingAction::RgbMatrix(mat) => match mat {
                    RgbMatrixAction::Toggle => Label::with_short("RGB Toggle", "RGBTg"),
                    RgbMatrixAction::On => Label::with_short("RGB On", "RGBOn"),
                    RgbMatrixAction::Off => Label::with_short("RGB Off", "RGBOff"),
                    RgbMatrixAction::ModeNext => Label::with_short("RGB Mode +", "RGBM+"),
                    RgbMatrixAction::ModePrev => Label::with_short("RGB Mode -", "RGBM-"),
                    RgbMatrixAction::HueInc => Label::with_short("RGB Hue +", "RGBH+"),
                    RgbMatrixAction::HueDec => Label::with_short("RGB Hue -", "RGBH-"),
                    RgbMatrixAction::SatInc => Label::with_short("RGB Sat +", "RGBS+"),
                    RgbMatrixAction::SatDec => Label::with_short("RGB Sat -", "RGBS-"),
                    RgbMatrixAction::BrightInc => Label::with_short("RGB Val +", "RGBV+"),
                    RgbMatrixAction::BrightDec => Label::with_short("RGB Val -", "RGBV-"),
                    RgbMatrixAction::SpeedInc => Label::with_short("RGB Spd +", "RGBSp+"),
                    RgbMatrixAction::SpeedDec => Label::with_short("RGB Spd -", "RGBSp-"),
                    RgbMatrixAction::Other { command, value: 0 } => {
                        Label::new(format!("RGBM {command}"))
                    }
                    RgbMatrixAction::Other { command, value } => {
                        Label::new(format!("RGBM {command} {value}"))
                    }
                },
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }

        KeySpec::Audio(audio) => {
            let label = match audio {
                AudioAction::On => Label::with_short("Audio On", "AudOn"),
                AudioAction::Off => Label::with_short("Audio Off", "AudOff"),
                AudioAction::Toggle => Label::with_short("Audio Toggle", "AudTg"),
                AudioAction::ClickyToggle => Label::with_short("Clicky Toggle", "ClkTg"),
                AudioAction::ClickyOn => Label::with_short("Clicky Enable", "ClkOn"),
                AudioAction::ClickyOff => Label::with_short("Clicky Disable", "ClkOff"),
                AudioAction::ClickyUp => Label::with_short("Clicky Up", "Clk+"),
                AudioAction::ClickyDown => Label::with_short("Clicky Down", "Clk-"),
                AudioAction::ClickyReset => Label::with_short("Clicky Reset", "ClkRst"),
                AudioAction::MusicOn => Label::with_short("Music On", "MusicOn"),
                AudioAction::MusicOff => Label::with_short("Music Off", "MusicOf"),
                AudioAction::MusicToggle => Label::with_short("Music Toggle", "MusicTg"),
                AudioAction::MusicModeNext => Label::with_short("Music Mode", "MusicMd"),
                AudioAction::VoiceNext => Label::with_short("Voice Next", "Voice+"),
                AudioAction::VoicePrev => Label::with_short("Voice Prev", "Voice-"),
                AudioAction::Other(n) => Label::with_short(format!("Audio {n}"), format!("Aud{n}")),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }

        KeySpec::Mouse(action) => match action {
            MouseAction::Press(btn) => {
                let (tap, symbol) = match btn {
                    MouseButton::Left => (
                        Label::new(""),
                        Some(egui_phosphor::regular::MOUSE_LEFT_CLICK.to_string()),
                    ),
                    MouseButton::Right => (
                        Label::new(""),
                        Some(egui_phosphor::regular::MOUSE_RIGHT_CLICK.to_string()),
                    ),
                    MouseButton::Middle => (
                        Label::new(""),
                        Some(egui_phosphor::regular::MOUSE_MIDDLE_CLICK.to_string()),
                    ),
                    MouseButton::Button4 => (Label::new("Mouse Btn4"), None),
                    MouseButton::Button5 => (Label::new("Mouse Btn5"), None),
                    MouseButton::Other(n) => (
                        Label::with_short(format!("Mouse {n}"), format!("M{n}")),
                        None,
                    ),
                };
                Some(LayoutKey {
                    tap,
                    symbol,
                    ..Default::default()
                })
            }
            MouseAction::Move { x, y } => {
                let (tap, symbol) = match (x.signum(), y.signum()) {
                    (0, -1) => (
                        Label::new(egui_phosphor::regular::ARROW_UP),
                        Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
                    ),
                    (0, 1) => (
                        Label::new(egui_phosphor::regular::ARROW_DOWN),
                        Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
                    ),
                    (-1, 0) => (
                        Label::new(egui_phosphor::regular::ARROW_LEFT),
                        Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
                    ),
                    (1, 0) => (
                        Label::new(egui_phosphor::regular::ARROW_RIGHT),
                        Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
                    ),
                    _ => (
                        Label::with_short(format!("Move ({x}, {y})"), format!("Mv {x},{y}")),
                        None,
                    ),
                };
                Some(LayoutKey {
                    tap,
                    symbol,
                    ..Default::default()
                })
            }
            MouseAction::Scroll { x, y } => {
                let (tap, symbol) = match (x.signum(), y.signum()) {
                    (0, 1) => (
                        Label::new(egui_phosphor::regular::ARROW_UP),
                        Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
                    ),
                    (0, -1) => (
                        Label::new(egui_phosphor::regular::ARROW_DOWN),
                        Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
                    ),
                    (-1, 0) => (
                        Label::new(egui_phosphor::regular::ARROW_LEFT),
                        Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
                    ),
                    (1, 0) => (
                        Label::new(egui_phosphor::regular::ARROW_RIGHT),
                        Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
                    ),
                    _ => (
                        Label::with_short(format!("Scroll ({x}, {y})"), format!("Scr {x},{y}")),
                        None,
                    ),
                };
                Some(LayoutKey {
                    tap,
                    symbol,
                    ..Default::default()
                })
            }
            MouseAction::Acceleration(n) => Some(LayoutKey {
                tap: Label::new(format!("Mouse Acc{n}")),
                ..Default::default()
            }),
        },

        KeySpec::Custom(binding) => {
            if let Some(display_name) = &binding.name {
                return Some(resolve_custom_named_key(
                    display_name,
                    binding.param1,
                    binding.param2,
                    layer_names,
                ));
            }

            match binding.kind {
                CustomKind::TapDance => Some(LayoutKey {
                    tap: Label::new(binding.id.to_string()),
                    behavior: Some(behavior_names::TAP_DANCE.label()),
                    ..Default::default()
                }),
                CustomKind::Macro => Some(LayoutKey {
                    tap: Label::new(binding.id.to_string()),
                    behavior: Some(behavior_names::MACRO.label()),
                    ..Default::default()
                }),
                CustomKind::Keyboard => Some(LayoutKey {
                    tap: Label::new(binding.id.to_string()),
                    behavior: Some(behavior_names::CUSTOM_KB.label()),
                    ..Default::default()
                }),
                CustomKind::User => Some(LayoutKey {
                    tap: Label::new(binding.id.to_string()),
                    behavior: Some(behavior_names::CUSTOM_USER.label()),
                    ..Default::default()
                }),
                CustomKind::Raw => Some(LayoutKey {
                    tap: Label::new(format!("0x{:04X}", binding.id)),
                    ..Default::default()
                }),
            }
        }
    }
}

pub fn resolve_custom_named_key(
    display_name: &str,
    param1: Option<CustomParam>,
    param2: Option<CustomParam>,
    layer_names: &[String],
) -> LayoutKey {
    let name = behavior_label(display_name);

    match (param1, param2) {
        (Some(CustomParam::Key(hold)), Some(CustomParam::Key(tap))) => {
            let hold_key =
                crate::hid_labels::hid_usage_to_layout_key(hold.page, hold.id).unwrap_or_default();
            let tap_key =
                crate::hid_labels::hid_usage_to_layout_key(tap.page, tap.id).unwrap_or_default();
            let hold_label = if hold.page == 0x07 && (0xE0..=0xE7).contains(&hold.id) {
                match hold.id {
                    0xE0 => Modifiers {
                        ctrl: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE1 => Modifiers {
                        shift: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE2 => Modifiers {
                        alt: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE3 => Modifiers {
                        gui: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE4 => Modifiers {
                        right_ctrl: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE5 => Modifiers {
                        right_shift: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE6 => Modifiers {
                        right_alt: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE7 => Modifiers {
                        right_gui: true,
                        ..Default::default()
                    }
                    .label(),
                    _ => hold_key.tap,
                }
            } else if let Some(arg) = hold_key.argument {
                arg
            } else if let Some(sym) = hold_key.symbol {
                Label::new(sym)
            } else {
                hold_key.tap
            };
            mod_tap_key(tap_key, hold_label, hold_key.mod_mask, Some(name))
        }
        (Some(CustomParam::Layer(layer_id)), Some(CustomParam::Key(tap))) => {
            let tap_key =
                crate::hid_labels::hid_usage_to_layout_key(tap.page, tap.id).unwrap_or_default();
            layer_tap_key(layer_id, tap_key, Some(name))
        }
        (Some(CustomParam::Key(key)), None) | (None, Some(CustomParam::Key(key))) => {
            let mut k =
                crate::hid_labels::hid_usage_to_layout_key(key.page, key.id).unwrap_or_default();
            k.behavior = Some(name);
            k
        }
        (Some(CustomParam::Layer(layer_id)), None) | (None, Some(CustomParam::Layer(layer_id))) => {
            let mut k = layer_switch_key(
                layer_id,
                layer_names
                    .get(layer_id as usize)
                    .filter(|n| !n.is_empty())
                    .map(|n| Label::new(n.as_str()))
                    .unwrap_or_else(|| Label::new(format!("L{}", layer_id))),
                BorderStyle::None,
            );
            k.behavior = Some(name);
            k
        }
        (None, None) => LayoutKey {
            tap: name,
            ..Default::default()
        },
        (first, second) => {
            let mut parts = Vec::new();
            for p in [first, second].into_iter().flatten() {
                match p {
                    CustomParam::Key(k) => {
                        let key = crate::hid_labels::hid_usage_to_layout_key(k.page, k.id)
                            .unwrap_or_default();
                        parts.push(key.symbol.unwrap_or(key.tap.full));
                    }
                    CustomParam::Layer(l) => {
                        let l_name = layer_names
                            .get(l as usize)
                            .filter(|n| !n.is_empty())
                            .cloned()
                            .unwrap_or_else(|| format!("L{}", l));
                        parts.push(l_name);
                    }
                    CustomParam::Number(num) => parts.push(num.to_string()),
                }
            }
            LayoutKey {
                tap: name,
                argument: (!parts.is_empty()).then(|| Label::new(parts.join(" "))),
                ..Default::default()
            }
        }
    }
}

pub fn behavior_label(display_name: &str) -> Label {
    let initials: String = display_name
        .split(|c: char| c == '_' || c == '-' || c.is_whitespace())
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect();

    if initials.chars().count() > 1 {
        Label::with_short(display_name, initials)
    } else {
        Label::new(display_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keypeek_core::HidKey;

    #[test]
    fn test_transparent_resolves_to_none() {
        let presenter = StandardKeyPresenter;
        assert_eq!(presenter.present_key(&KeySpec::Transparent, &[]), None);
    }

    #[test]
    fn test_plain_key_press() {
        let key = KeySpec::KeyPress {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers::default(),
        };
        let presenter = StandardKeyPresenter;
        let label = presenter.present_key(&key, &[]).expect("should resolve");
        assert_eq!(label.tap.full, "A");
    }

    #[test]
    fn test_modified_key_press() {
        let mods = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let key = KeySpec::KeyPress {
            key: HidKey::keyboard(0x06),
            modifiers: mods,
        };
        let presenter = StandardKeyPresenter;
        let label = presenter.present_key(&key, &[]).expect("should resolve");
        assert_eq!(label.tap.full, "C");
        assert_eq!(
            label.argument.as_ref().map(|a| a.full.as_str()),
            Some(keypeek_core::modifier_symbols::MOD_CTRL.full)
        );
    }

    #[test]
    fn test_layer_tap() {
        let key = KeySpec::LayerTap {
            layer: 2,
            tap: HidKey::keyboard(0x2C), // Space
            tap_modifiers: Modifiers::default(),
        };
        let presenter = StandardKeyPresenter;
        let label = presenter.present_key(&key, &[]).expect("should resolve");
        assert_eq!(label.layer_ref, Some(2));
    }
}
