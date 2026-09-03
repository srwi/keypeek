use crate::hid_labels::Modifiers;
use crate::layout_key::{behavior_names, BorderStyle, KeycodeKind, Label, LayoutKey};
use zmk_studio_api::{
    BacklightCommand, Behavior, BehaviorParam, BluetoothCommand, ExternalPowerCommand, HidUsage,
    MouseButton, OutputSelection, UnderglowCommand,
};

use super::hid_usage::hid_usage_to_layout_key;

pub fn behavior_to_layout_key(behavior: &Behavior, layer_names: &[String]) -> Option<LayoutKey> {
    match behavior {
        Behavior::Transparent => None,

        Behavior::None => Some(LayoutKey {
            tap: Label::new(""),
            ..Default::default()
        }),
        Behavior::KeyPress(keycode) => Some(hid_usage_to_layout_key(*keycode)),
        Behavior::KeyToggle(keycode) => {
            let mut key = hid_usage_to_layout_key(*keycode);
            key.behavior = Some(behavior_names::KEY_TOGGLE.label());
            Some(key)
        }
        Behavior::MomentaryLayer { layer_id } => {
            Some(layer_layout_key(BorderStyle::None, *layer_id, layer_names))
        }
        Behavior::ToggleLayer { layer_id } => {
            Some(layer_layout_key(BorderStyle::Solid, *layer_id, layer_names))
        }
        Behavior::ToLayer { layer_id } => {
            Some(layer_layout_key(BorderStyle::Solid, *layer_id, layer_names))
        }
        Behavior::StickyLayer { layer_id } => Some(layer_layout_key(
            BorderStyle::Dashed,
            *layer_id,
            layer_names,
        )),
        Behavior::LayerTap { layer_id, tap } => Some(layer_tap_layout_key(*layer_id, *tap, None)),
        Behavior::ModTap { hold, tap } => Some(hold_tap_layout_key(
            *hold,
            *tap,
            Some(behavior_names::MOD_TAP.label()),
        )),
        Behavior::StickyKey(keycode) => {
            let key = hid_usage_to_layout_key(*keycode);
            Some(LayoutKey {
                tap: key.tap,
                behavior: Some(behavior_names::STICKY_KEY.label()),
                argument: key.argument,
                shifted: key.shifted,
                ralt: key.ralt,
                ralt_shifted: key.ralt_shifted,
                mod_mask: key.mod_mask,
                symbol: key.symbol,
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        Behavior::CapsWord => Some(LayoutKey {
            tap: Label::with_short("Caps Word", "CW"),
            ..Default::default()
        }),
        Behavior::KeyRepeat => Some(LayoutKey {
            tap: Label::with_short("Key Repeat", "Rep"),
            ..Default::default()
        }),
        Behavior::Reset => Some(LayoutKey {
            tap: Label::with_short("Reset", "Rst"),
            ..Default::default()
        }),
        Behavior::Bootloader => Some(LayoutKey {
            tap: Label::with_short("Bootloader", "Boot"),
            ..Default::default()
        }),
        Behavior::SoftOff => Some(LayoutKey {
            tap: Label::with_short("Soft Off", "Off"),
            ..Default::default()
        }),
        Behavior::StudioUnlock => Some(LayoutKey {
            tap: Label::with_short("Studio Unlock", "Unlock"),
            ..Default::default()
        }),
        Behavior::GraveEscape => Some(LayoutKey {
            tap: Label::with_short("Grave Esc", "G/E"),
            ..Default::default()
        }),
        Behavior::Bluetooth(cmd) => {
            let label = match cmd {
                BluetoothCommand::Clear => Label::new("BT Clr"),
                BluetoothCommand::Next => Label::new("BT Nxt"),
                BluetoothCommand::Prev => Label::new("BT Prv"),
                BluetoothCommand::Select(n) => {
                    Label::with_short(format!("BT Sel {n}"), format!("BT{n}"))
                }
                BluetoothCommand::ClearAll => Label::with_short("BT Clr All", "BTClr"),
                BluetoothCommand::Disconnect(n) => {
                    Label::with_short(format!("BT Disc {n}"), format!("BTD{n}"))
                }
                BluetoothCommand::Other { command, value: 0 } => {
                    Label::new(format!("BT {command}"))
                }
                BluetoothCommand::Other { command, value } => {
                    Label::new(format!("BT {command} {value}"))
                }
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::OutputSelection(out) => {
            let label = match out {
                OutputSelection::Toggle => Label::with_short("Out Tog", "OutTg"),
                OutputSelection::Usb => Label::new("Out USB"),
                OutputSelection::Ble => Label::new("Out BLE"),
                OutputSelection::None => Label::with_short("Out None", "OutNo"),
                OutputSelection::Other(n) => Label::new(format!("Out {n}")),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::ExternalPower(ep) => {
            let label = match ep {
                ExternalPowerCommand::Off => Label::with_short("ExtPwr Off", "EPOff"),
                ExternalPowerCommand::On => Label::with_short("ExtPwr On", "EPOn"),
                ExternalPowerCommand::Toggle => Label::with_short("ExtPwr Tog", "EPTog"),
                ExternalPowerCommand::Other(n) => {
                    Label::with_short(format!("ExtPwr {n}"), format!("EP{n}"))
                }
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::Backlight(bl) => {
            let label = match bl {
                BacklightCommand::On => Label::new("BL On"),
                BacklightCommand::Off => Label::new("BL Off"),
                BacklightCommand::Toggle => Label::with_short("BL Toggle", "BLTog"),
                BacklightCommand::Inc => Label::with_short("BL Inc", "BL+"),
                BacklightCommand::Dec => Label::with_short("BL Dec", "BL-"),
                BacklightCommand::Cycle => Label::with_short("BL Cycle", "BLCyc"),
                BacklightCommand::Set(n) => {
                    Label::with_short(format!("BL Set {n}"), format!("BL{n}"))
                }
                BacklightCommand::Other { command, value: 0 } => {
                    Label::new(format!("BL {command}"))
                }
                BacklightCommand::Other { command, value } => {
                    Label::new(format!("BL {command} {value}"))
                }
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::Underglow(ug) => {
            let label = match ug {
                UnderglowCommand::Toggle => Label::with_short("RGB Toggle", "RGBTg"),
                UnderglowCommand::On => Label::with_short("RGB On", "RGBOn"),
                UnderglowCommand::Off => Label::with_short("RGB Off", "RGBOff"),
                UnderglowCommand::HueInc => Label::with_short("Hue +", "Hue+"),
                UnderglowCommand::HueDec => Label::with_short("Hue -", "Hue-"),
                UnderglowCommand::SatInc => Label::with_short("Sat +", "Sat+"),
                UnderglowCommand::SatDec => Label::with_short("Sat -", "Sat-"),
                UnderglowCommand::BrightInc => Label::with_short("Bright +", "Bri+"),
                UnderglowCommand::BrightDec => Label::with_short("Bright -", "Bri-"),
                UnderglowCommand::SpeedInc => Label::with_short("Speed +", "Spd+"),
                UnderglowCommand::SpeedDec => Label::with_short("Speed -", "Spd-"),
                UnderglowCommand::EffectInc => Label::with_short("Effect +", "Eff+"),
                UnderglowCommand::EffectDec => Label::with_short("Effect -", "Eff-"),
                UnderglowCommand::EffectSet => Label::with_short("Effect Set", "EffS"),
                UnderglowCommand::Color => Label::with_short("RGB Color", "Color"),
                UnderglowCommand::Other { command, value: 0 } => {
                    Label::new(format!("RGB {command}"))
                }
                UnderglowCommand::Other { command, value } => {
                    Label::new(format!("RGB {command} {value}"))
                }
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::MouseKeyPress(btn) => {
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
        Behavior::MouseMove { x, y } => {
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
        Behavior::MouseScroll { x, y } => {
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
        Behavior::Custom {
            display_name,
            param1,
            param2,
            ..
        } => Some(custom_layout_key(
            display_name,
            *param1,
            *param2,
            layer_names,
        )),
        Behavior::Unknown {
            behavior_id,
            param1,
            param2,
        } => {
            let label = if *param2 != 0 {
                format!("0x{:X} {} {}", behavior_id, param1, param2)
            } else if *param1 != 0 {
                format!("0x{:X} {}", behavior_id, param1)
            } else {
                format!("0x{:X}", behavior_id)
            };
            Some(LayoutKey {
                tap: Label::new(label),
                ..Default::default()
            })
        }
    }
}

fn mod_mask_to_glyphs(m: u8) -> Label {
    Modifiers::from_zmk_mask(m).label()
}

fn layer_tap_layout_key(layer_id: u32, tap: HidUsage, behavior: Option<Label>) -> LayoutKey {
    let tap_key = hid_usage_to_layout_key(tap);
    crate::hid_labels::layer_tap_key(layer_id as u8, tap_key, behavior)
}

fn hold_tap_layout_key(hold: HidUsage, tap: HidUsage, behavior: Option<Label>) -> LayoutKey {
    let hold_key = hid_usage_to_layout_key(hold);
    let tap_key = hid_usage_to_layout_key(tap);
    let hold_label = if let Some(arg) = hold_key.argument {
        arg
    } else if let Some(sym) = hold_key.symbol {
        Label::new(sym)
    } else if hold.modifier_mask() != 0 {
        mod_mask_to_glyphs(hold.modifier_mask())
    } else {
        hold_key.tap
    };
    crate::hid_labels::mod_tap_key(tap_key, hold_label, hold_key.mod_mask, behavior)
}

fn custom_layout_key(
    display_name: &str,
    param1: BehaviorParam,
    param2: BehaviorParam,
    layer_names: &[String],
) -> LayoutKey {
    let name = behavior_label(display_name);
    let named = |mut key: LayoutKey| {
        key.behavior = Some(name.clone());
        key
    };

    match (param1, param2) {
        // A hold-tap passes its first parameter to the hold side and its second
        // parameter to the tap side. Thus the tap legend shows the tapped key.
        (BehaviorParam::Keycode(hold), BehaviorParam::Keycode(tap)) => {
            hold_tap_layout_key(hold, tap, Some(name.clone()))
        }
        // Holding activates the layer. `layer_ref` shows this layer. The
        // firmware does not report whether the hold side is momentary or a
        // toggle. Assume momentary: it is the common case, and the border is
        // only a hint.
        (BehaviorParam::LayerId(layer_id), BehaviorParam::Keycode(tap)) => {
            layer_tap_layout_key(layer_id, tap, Some(name.clone()))
        }
        // One parameter carries the behavior. The other side (for example a
        // macro) is not reported. The name is shown in its place.
        (BehaviorParam::Keycode(keycode), BehaviorParam::Unused)
        | (BehaviorParam::Unused, BehaviorParam::Keycode(keycode)) => {
            named(hid_usage_to_layout_key(keycode))
        }
        (BehaviorParam::LayerId(layer_id), BehaviorParam::Unused)
        | (BehaviorParam::Unused, BehaviorParam::LayerId(layer_id)) => {
            named(layer_layout_key(BorderStyle::None, layer_id, layer_names))
        }
        (BehaviorParam::Unused, BehaviorParam::Unused) => LayoutKey {
            tap: name,
            ..Default::default()
        },
        // For any other pair, show the name. Show a summary of the parameters
        // below it.
        (first, second) => LayoutKey {
            tap: name,
            argument: param_summary(first, second, layer_names),
            ..Default::default()
        },
    }
}

/// A label for a behavior's reported name. The label can shrink.
///
/// The name is the keymap's `display-name`, or the devicetree node name when
/// the keymap sets none. Node names can be long. A multi-word name uses its
/// initials as the short form. For example, `home_row_mod_left` shows `HRML`
/// on a key that is too narrow for the full name.
fn behavior_label(display_name: &str) -> Label {
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

/// A summary of the parameters for the bottom strip. Used when the parameters
/// have no shape of their own to render.
fn param_summary(
    param1: BehaviorParam,
    param2: BehaviorParam,
    layer_names: &[String],
) -> Option<Label> {
    let text = |param: BehaviorParam| match param {
        BehaviorParam::Unused => None,
        BehaviorParam::Keycode(keycode) => {
            let key = hid_usage_to_layout_key(keycode);
            key.symbol.or(Some(key.tap.full))
        }
        BehaviorParam::LayerId(layer_id) => Some(layer_arg_label(layer_names, layer_id).full),
        BehaviorParam::Number(value) => Some(value.to_string()),
    };

    let parts: Vec<String> = [param1, param2].into_iter().filter_map(text).collect();
    (!parts.is_empty()).then(|| Label::new(parts.join(" ")))
}

/// Build a pure layer-switch key: the target layer is the centered label and
/// `border` is the sole indicator; there are no legend strips.
fn layer_layout_key(border: BorderStyle, layer_id: u32, layer_names: &[String]) -> LayoutKey {
    crate::hid_labels::layer_switch_key(
        layer_id as u8,
        layer_arg_label(layer_names, layer_id),
        border,
    )
}

fn layer_arg_label(layer_names: &[String], layer_id: u32) -> Label {
    match layer_name(layer_names, layer_id) {
        Some(name) => Label::new(name),
        None => Label::new(format!("L{}", layer_id)),
    }
}

fn layer_name(layer_names: &[String], id: u32) -> Option<&str> {
    layer_names
        .get(id as usize)
        .map(String::as_str)
        .filter(|name| !name.is_empty())
}
