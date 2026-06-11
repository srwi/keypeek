use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use zmk_studio_api::Behavior;

use super::hid_usage::hid_usage_to_layout_key;

pub fn behavior_to_layout_key(behavior: &Behavior) -> Option<LayoutKey> {
    match behavior {
        Behavior::Transparent => None,

        Behavior::None => Some(LayoutKey {
            tap: Label::new(""),
            ..Default::default()
        }),
        Behavior::KeyPress(keycode) => Some(hid_usage_to_layout_key(*keycode)),
        Behavior::KeyToggle(keycode) => {
            let mut key = hid_usage_to_layout_key(*keycode);
            key.hold = Some(Label::new("Toggle"));
            Some(key)
        }
        Behavior::MomentaryLayer { layer_id } => Some(layer_layout_key("MO", *layer_id)),
        Behavior::ToggleLayer { layer_id } => Some(layer_layout_key("TG", *layer_id)),
        Behavior::ToLayer { layer_id } => Some(layer_layout_key("TO", *layer_id)),
        Behavior::StickyLayer { layer_id } => Some(layer_layout_key("SL", *layer_id)),
        Behavior::LayerTap { layer_id, tap } => {
            let tap_key = hid_usage_to_layout_key(*tap);
            Some(LayoutKey {
                tap: tap_key.tap,
                hold: Some(Label::with_short(
                    format!("L{}", layer_id),
                    format!("L{}", layer_id),
                )),
                symbol: tap_key.symbol,
                kind: KeycodeKind::Modifier,
                layer_ref: Some(*layer_id as u8),
            })
        }
        Behavior::ModTap { hold, tap } => {
            let hold_key = hid_usage_to_layout_key(*hold);
            let tap_key = hid_usage_to_layout_key(*tap);
            let hold_label = if let Some(symbol) = hold_key.symbol {
                Label::new(symbol)
            } else {
                hold_key.tap
            };
            Some(LayoutKey {
                tap: tap_key.tap,
                hold: Some(hold_label),
                symbol: tap_key.symbol,
                kind: KeycodeKind::Basic,
                layer_ref: None,
            })
        }
        Behavior::StickyKey(keycode) => {
            let key = hid_usage_to_layout_key(*keycode);
            Some(LayoutKey {
                tap: Label::with_short(
                    format!("OS {}", key.tap.full),
                    format!("OS{}", key.tap.short.as_deref().unwrap_or(&key.tap.full)),
                ),
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

fn layer_layout_key(abbreviation: &str, layer_id: u32) -> LayoutKey {
    LayoutKey {
        tap: Label::with_short(
            format!("{} {}", abbreviation, layer_id),
            format!("{}{}", abbreviation, layer_id),
        ),
        kind: KeycodeKind::Special,
        layer_ref: Some(layer_id as u8),
        ..Default::default()
    }
}
