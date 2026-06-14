use crate::layout_key::{behavior_names, KeycodeKind, Label, LayoutKey};
use zmk_studio_api::Behavior;

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
        Behavior::MomentaryLayer { layer_id } => Some(layer_layout_key(
            behavior_names::MOMENTARY.label(),
            *layer_id,
            layer_names,
        )),
        Behavior::ToggleLayer { layer_id } => Some(layer_layout_key(
            behavior_names::TOGGLE.label(),
            *layer_id,
            layer_names,
        )),
        Behavior::ToLayer { layer_id } => Some(layer_layout_key(
            behavior_names::TO_LAYER.label(),
            *layer_id,
            layer_names,
        )),
        Behavior::StickyLayer { layer_id } => Some(layer_layout_key(
            behavior_names::STICKY_LAYER.label(),
            *layer_id,
            layer_names,
        )),
        Behavior::LayerTap { layer_id, tap } => {
            let tap_key = hid_usage_to_layout_key(*tap);
            Some(LayoutKey {
                tap: tap_key.tap,
                behavior: Some(behavior_names::LAYER_TAP.label()),
                argument: Some(layer_arg_label(layer_names, *layer_id)),
                shifted: tap_key.shifted,
                symbol: tap_key.symbol,
                kind: KeycodeKind::Modifier,
                layer_ref: Some(*layer_id as u8),
            })
        }
        Behavior::ModTap { hold, tap } => {
            let hold_key = hid_usage_to_layout_key(*hold);
            let tap_key = hid_usage_to_layout_key(*tap);
            // A glyph modifier has no shorter form; a text modifier carries its
            // short name in `tap`, so the argument strip can shrink to fit.
            let mod_label = match hold_key.symbol {
                Some(sym) => Label::new(sym),
                None => hold_key.tap,
            };
            Some(LayoutKey {
                tap: tap_key.tap,
                behavior: Some(behavior_names::MOD_TAP.label()),
                argument: Some(mod_label),
                shifted: tap_key.shifted,
                symbol: tap_key.symbol,
                kind: KeycodeKind::Basic,
                layer_ref: None,
            })
        }
        Behavior::StickyKey(keycode) => {
            let key = hid_usage_to_layout_key(*keycode);
            Some(LayoutKey {
                tap: key.tap,
                behavior: Some(behavior_names::STICKY_KEY.label()),
                shifted: key.shifted,
                symbol: key.symbol,
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }
        Behavior::CapsWord => Some(LayoutKey {
            tap: Label::with_short("Caps Word", "CW"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::KeyRepeat => Some(LayoutKey {
            tap: Label::with_short("Key Repeat", "Rep"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::Reset => Some(LayoutKey {
            tap: Label::new("Reset"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::Bootloader => Some(LayoutKey {
            tap: Label::with_short("Bootloader", "Boot"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::SoftOff => Some(LayoutKey {
            tap: Label::with_short("Soft Off", "Off"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::StudioUnlock => Some(LayoutKey {
            tap: Label::with_short("Studio Unlock", "Unlock"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::GraveEscape => Some(LayoutKey {
            tap: Label::with_short("Grave Esc", "G/E"),
            kind: KeycodeKind::Special,
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
                kind: KeycodeKind::Special,
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
                kind: KeycodeKind::Special,
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
                kind: KeycodeKind::Special,
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
                kind: KeycodeKind::Special,
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
                kind: KeycodeKind::Special,
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
                kind: KeycodeKind::Special,
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
                kind: KeycodeKind::Special,
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
                kind: KeycodeKind::Special,
                ..Default::default()
            })
        }
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

/// Decode a ZMK pointing move/scroll value into signed (x, y) components.
/// ZMK packs these as `(x << 16) | (y & 0xFFFF)` (see dt-bindings/zmk/pointing.h).
fn decode_mouse_xy(value: u32) -> (i16, i16) {
    let x = ((value >> 16) & 0xFFFF) as i16;
    let y = (value & 0xFFFF) as i16;
    (x, y)
}

fn layer_layout_key(behavior: Label, layer_id: u32, layer_names: &[String]) -> LayoutKey {
    LayoutKey {
        tap: layer_arg_label(layer_names, layer_id),
        behavior: Some(behavior),
        kind: KeycodeKind::Special,
        layer_ref: Some(layer_id as u8),
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
