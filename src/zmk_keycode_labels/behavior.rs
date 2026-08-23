use crate::layout_key::{behavior_names, BorderStyle, KeycodeKind, Label, LayoutKey};
use zmk_studio_api::{Behavior, BehaviorParam, HidUsage};

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
                shifted: key.shifted,
                ralt: key.ralt,
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
            tap: Label::new("Reset"),
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
        Behavior::Bluetooth { command, value } => {
            let label = match *command {
                0 => Label::new("BT Clr"),
                1 => Label::new("BT Nxt"),
                2 => Label::new("BT Prv"),
                3 => Label::with_short(format!("BT Sel {}", value), format!("BT{}", value)),
                4 => Label::with_short("BT Clr All", "BTClr"),
                5 => Label::with_short(format!("BT Disc {}", value), format!("BTD{}", value)),
                n => Label::new(format!("BT {}", n)),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::OutputSelection { value } => {
            let label = match *value {
                0 => Label::with_short("Out Tog", "OutTg"),
                1 => Label::new("Out USB"),
                2 => Label::new("Out BLE"),
                3 => Label::with_short("Out None", "OutNo"),
                n => Label::new(format!("Out {}", n)),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::ExternalPower { value } => {
            let label = match *value {
                0 => Label::with_short("ExtPwr Off", "EPOff"),
                1 => Label::with_short("ExtPwr On", "EPOn"),
                2 => Label::with_short("ExtPwr Tog", "EPTog"),
                n => Label::with_short(format!("ExtPwr {}", n), format!("EP{}", n)),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::Backlight { command, value } => {
            let label = match *command {
                0 => Label::new("BL On"),
                1 => Label::new("BL Off"),
                2 => Label::new("BL Tog"),
                3 => Label::with_short("BL Inc", "BL+"),
                4 => Label::with_short("BL Dec", "BL-"),
                5 => Label::with_short("BL Cycle", "BLCyc"),
                6 => Label::with_short(format!("BL Set {}", value), format!("BL{}", value)),
                n => Label::new(format!("BL {}", n)),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::Underglow { command, .. } => {
            let label = match *command {
                0 => Label::new("RGB Tog"),
                1 => Label::new("RGB On"),
                2 => Label::new("RGB Off"),
                3 => Label::with_short("Hue +", "Hue+"),
                4 => Label::with_short("Hue -", "Hue-"),
                5 => Label::with_short("Sat +", "Sat+"),
                6 => Label::with_short("Sat -", "Sat-"),
                7 => Label::with_short("Bright +", "Bri+"),
                8 => Label::with_short("Bright -", "Bri-"),
                9 => Label::with_short("Speed +", "Spd+"),
                10 => Label::with_short("Speed -", "Spd-"),
                11 => Label::with_short("Effect +", "Eff+"),
                12 => Label::with_short("Effect -", "Eff-"),
                13 => Label::with_short("Effect Set", "EffS"),
                14 => Label::with_short("RGB Color", "Color"),
                n => Label::new(format!("RGB {}", n)),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::MouseKeyPress { value } => {
            let label = match *value {
                1 => Label::with_short("L Click", "LClk"),
                2 => Label::with_short("R Click", "RClk"),
                4 => Label::with_short("M Click", "MClk"),
                8 => Label::with_short("Mouse 4", "MB4"),
                16 => Label::with_short("Mouse 5", "MB5"),
                n => Label::with_short(format!("Mouse {}", n), format!("M{}", n)),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::MouseMove { value } => {
            let (x, y) = decode_mouse_xy(*value);
            let label = match (x.signum(), y.signum()) {
                (0, -1) => Label::with_short("Mouse Up", "MsUp"),
                (0, 1) => Label::with_short("Mouse Down", "MsDn"),
                (-1, 0) => Label::with_short("Mouse Left", "MsLt"),
                (1, 0) => Label::with_short("Mouse Right", "MsRt"),
                _ => Label::with_short(format!("Move {}", value), format!("Mv{}", value)),
            };
            Some(LayoutKey {
                tap: label,
                ..Default::default()
            })
        }
        Behavior::MouseScroll { value } => {
            let (x, y) = decode_mouse_xy(*value);
            let label = match (x.signum(), y.signum()) {
                (0, 1) => Label::with_short("Scroll Up", "ScrUp"),
                (0, -1) => Label::with_short("Scroll Down", "ScrDn"),
                (-1, 0) => Label::with_short("Scroll Left", "ScrLt"),
                (1, 0) => Label::with_short("Scroll Right", "ScrRt"),
                _ => Label::with_short(format!("Scroll {}", value), format!("Scr{}", value)),
            };
            Some(LayoutKey {
                tap: label,
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

fn layer_tap_layout_key(layer_id: u32, tap: HidUsage, behavior: Option<Label>) -> LayoutKey {
    let tap_key = hid_usage_to_layout_key(tap);
    LayoutKey {
        tap: tap_key.tap,
        behavior,
        shifted: tap_key.shifted,
        ralt: tap_key.ralt,
        symbol: tap_key.symbol,
        kind: KeycodeKind::Modifier,
        layer_ref: Some(layer_id as u8),
        border: BorderStyle::None,
        ..Default::default()
    }
}

fn hold_tap_layout_key(hold: HidUsage, tap: HidUsage, behavior: Option<Label>) -> LayoutKey {
    let hold_key = hid_usage_to_layout_key(hold);
    let tap_key = hid_usage_to_layout_key(tap);
    let hold_label = match hold_key.symbol {
        Some(sym) => Label::new(sym),
        None => hold_key.tap,
    };
    LayoutKey {
        tap: tap_key.tap,
        behavior,
        argument: Some(hold_label),
        shifted: tap_key.shifted,
        ralt: tap_key.ralt,
        mod_mask: hold_key.mod_mask,
        symbol: tap_key.symbol,
        kind: KeycodeKind::Basic,
        layer_ref: None,
        border: BorderStyle::None,
    }
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

/// Decode a ZMK pointing value: `(x << 16) | (y & 0xFFFF)` (dt-bindings/zmk/pointing.h).
fn decode_mouse_xy(value: u32) -> (i16, i16) {
    let x = ((value >> 16) & 0xFFFF) as i16;
    let y = (value & 0xFFFF) as i16;
    (x, y)
}

/// Build a pure layer-switch key: the target layer is the centered label and
/// `border` is the sole indicator; there are no legend strips.
fn layer_layout_key(border: BorderStyle, layer_id: u32, layer_names: &[String]) -> LayoutKey {
    LayoutKey {
        tap: layer_arg_label(layer_names, layer_id),
        kind: KeycodeKind::Modifier,
        layer_ref: Some(layer_id as u8),
        border,
        ..Default::default()
    }
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
