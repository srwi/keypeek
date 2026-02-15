use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use zmk_studio_api::{Behavior, HidUsage, Keycode};

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
                kind: KeycodeKind::Special,
                layer_ref: Some(*layer_id as u8),
            })
        }
        Behavior::ModTap { hold, tap } => {
            let hold_key = hid_usage_to_layout_key(*hold);
            let tap_key = hid_usage_to_layout_key(*tap);
            Some(LayoutKey {
                tap: tap_key.tap,
                hold: Some(hold_key.tap),
                symbol: tap_key.symbol,
                kind: KeycodeKind::Modifier,
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
        Behavior::Bluetooth { command, .. } => {
            let label = match *command {
                0 => "BT Clr",
                1 => "BT Nxt",
                2 => "BT Prv",
                n => {
                    return Some(LayoutKey {
                        tap: Label::new(format!("BT {}", n)),
                        kind: KeycodeKind::Special,
                        ..Default::default()
                    })
                }
            };
            Some(LayoutKey {
                tap: Label::new(label),
                kind: KeycodeKind::Special,
                ..Default::default()
            })
        }
        Behavior::OutputSelection { value } => Some(LayoutKey {
            tap: Label::with_short(format!("Out {}", value), format!("Out{}", value)),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::ExternalPower { value } => Some(LayoutKey {
            tap: Label::with_short(format!("ExtPwr {}", value), format!("EP{}", value)),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::Backlight { command, .. } => Some(LayoutKey {
            tap: Label::with_short(format!("BL {}", command), format!("BL{}", command)),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::Underglow { command, .. } => Some(LayoutKey {
            tap: Label::with_short(format!("RGB {}", command), format!("RGB{}", command)),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::MouseKeyPress { value } => Some(LayoutKey {
            tap: Label::with_short(format!("Mouse {}", value), format!("M{}", value)),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::MouseMove { value } => Some(LayoutKey {
            tap: Label::with_short(format!("Move {}", value), format!("Mv{}", value)),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Behavior::MouseScroll { value } => Some(LayoutKey {
            tap: Label::with_short(format!("Scroll {}", value), format!("Scr{}", value)),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
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

fn hid_usage_to_layout_key(usage: HidUsage) -> LayoutKey {
    if usage.modifiers() == 0 {
        if let Some(keycode) = usage.known_keycode() {
            return keycode_to_layout_key(&keycode);
        }

        return LayoutKey {
            tap: Label::new(format!("0x{:08X}", usage.to_hid_usage())),
            ..Default::default()
        };
    }

    if let Some(named_key) = usage.known_keycode() {
        return keycode_to_layout_key(&named_key);
    }

    let base = usage.base();
    let base_label = if let Some(base_keycode) = base.known_keycode() {
        keycode_to_layout_key(&base_keycode).tap.full
    } else {
        format!("0x{:08X}", base.to_hid_usage())
    };

    let mut rendered = base_label;
    for modifier in usage.modifier_labels().iter().rev() {
        rendered = format!("{modifier}({rendered})");
    }

    LayoutKey {
        tap: Label::new(rendered),
        kind: KeycodeKind::Modifier,
        ..Default::default()
    }
}

fn keycode_to_layout_key(keycode: &Keycode) -> LayoutKey {
    if let Some(key) = keycode_label(keycode) {
        return key;
    }

    // Fallback: Use the canonical ZMK name
    let name = keycode.to_name();
    LayoutKey {
        tap: Label::new(name),
        ..Default::default()
    }
}

fn keycode_label(keycode: &Keycode) -> Option<LayoutKey> {
    match keycode {
        Keycode::SYSTEM_POWER => Some(LayoutKey {
            tap: Label::new("Power"),
            ..Default::default()
        }),
        Keycode::SYSTEM_SLEEP => Some(LayoutKey {
            tap: Label::new("Sleep"),
            ..Default::default()
        }),
        Keycode::SYSTEM_WAKE_UP => Some(LayoutKey {
            tap: Label::new("Wake"),
            ..Default::default()
        }),
        Keycode::A => Some(LayoutKey {
            tap: Label::new("A"),
            ..Default::default()
        }),
        Keycode::B => Some(LayoutKey {
            tap: Label::new("B"),
            ..Default::default()
        }),
        Keycode::C => Some(LayoutKey {
            tap: Label::new("C"),
            ..Default::default()
        }),
        Keycode::D => Some(LayoutKey {
            tap: Label::new("D"),
            ..Default::default()
        }),
        Keycode::E => Some(LayoutKey {
            tap: Label::new("E"),
            ..Default::default()
        }),
        Keycode::F => Some(LayoutKey {
            tap: Label::new("F"),
            ..Default::default()
        }),
        Keycode::G => Some(LayoutKey {
            tap: Label::new("G"),
            ..Default::default()
        }),
        Keycode::H => Some(LayoutKey {
            tap: Label::new("H"),
            ..Default::default()
        }),
        Keycode::I => Some(LayoutKey {
            tap: Label::new("I"),
            ..Default::default()
        }),
        Keycode::J => Some(LayoutKey {
            tap: Label::new("J"),
            ..Default::default()
        }),
        Keycode::K => Some(LayoutKey {
            tap: Label::new("K"),
            ..Default::default()
        }),
        Keycode::L => Some(LayoutKey {
            tap: Label::new("L"),
            ..Default::default()
        }),
        Keycode::M => Some(LayoutKey {
            tap: Label::new("M"),
            ..Default::default()
        }),
        Keycode::N => Some(LayoutKey {
            tap: Label::new("N"),
            ..Default::default()
        }),
        Keycode::O => Some(LayoutKey {
            tap: Label::new("O"),
            ..Default::default()
        }),
        Keycode::P => Some(LayoutKey {
            tap: Label::new("P"),
            ..Default::default()
        }),
        Keycode::Q => Some(LayoutKey {
            tap: Label::new("Q"),
            ..Default::default()
        }),
        Keycode::R => Some(LayoutKey {
            tap: Label::new("R"),
            ..Default::default()
        }),
        Keycode::S => Some(LayoutKey {
            tap: Label::new("S"),
            ..Default::default()
        }),
        Keycode::T => Some(LayoutKey {
            tap: Label::new("T"),
            ..Default::default()
        }),
        Keycode::U => Some(LayoutKey {
            tap: Label::new("U"),
            ..Default::default()
        }),
        Keycode::V => Some(LayoutKey {
            tap: Label::new("V"),
            ..Default::default()
        }),
        Keycode::W => Some(LayoutKey {
            tap: Label::new("W"),
            ..Default::default()
        }),
        Keycode::X => Some(LayoutKey {
            tap: Label::new("X"),
            ..Default::default()
        }),
        Keycode::Y => Some(LayoutKey {
            tap: Label::new("Y"),
            ..Default::default()
        }),
        Keycode::Z => Some(LayoutKey {
            tap: Label::new("Z"),
            ..Default::default()
        }),
        Keycode::NUMBER_1 => Some(LayoutKey {
            tap: Label::new("!\n1"),
            ..Default::default()
        }),
        Keycode::NUMBER_2 => Some(LayoutKey {
            tap: Label::new("@\n2"),
            ..Default::default()
        }),
        Keycode::NUMBER_3 => Some(LayoutKey {
            tap: Label::new("#\n3"),
            ..Default::default()
        }),
        Keycode::NUMBER_4 => Some(LayoutKey {
            tap: Label::new("$\n4"),
            ..Default::default()
        }),
        Keycode::NUMBER_5 => Some(LayoutKey {
            tap: Label::new("%\n5"),
            ..Default::default()
        }),
        Keycode::NUMBER_6 => Some(LayoutKey {
            tap: Label::new("^\n6"),
            ..Default::default()
        }),
        Keycode::NUMBER_7 => Some(LayoutKey {
            tap: Label::new("&\n7"),
            ..Default::default()
        }),
        Keycode::NUMBER_8 => Some(LayoutKey {
            tap: Label::new("*\n8"),
            ..Default::default()
        }),
        Keycode::NUMBER_9 => Some(LayoutKey {
            tap: Label::new("(\n9"),
            ..Default::default()
        }),
        Keycode::NUMBER_0 => Some(LayoutKey {
            tap: Label::new(")\n0"),
            ..Default::default()
        }),
        Keycode::RETURN => Some(LayoutKey {
            tap: Label::new("Enter"),
            symbol: Some(egui_phosphor::regular::ARROW_ELBOW_DOWN_LEFT.to_string()),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Keycode::ESCAPE => Some(LayoutKey {
            tap: Label::new("Esc"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Keycode::BACKSPACE => Some(LayoutKey {
            tap: Label::new("Backspace"),
            symbol: Some(egui_phosphor::regular::BACKSPACE.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::TAB => Some(LayoutKey {
            tap: Label::new("Tab"),
            symbol: Some(egui_phosphor::regular::ARROWS_LEFT_RIGHT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::SPACE => Some(LayoutKey {
            tap: Label::with_short("Space", "Spc"),
            ..Default::default()
        }),
        Keycode::MINUS => Some(LayoutKey {
            tap: Label::new("_\n-"),
            ..Default::default()
        }),
        Keycode::EQUAL => Some(LayoutKey {
            tap: Label::new("+\n="),
            ..Default::default()
        }),
        Keycode::BACKSLASH => Some(LayoutKey {
            tap: Label::new("|\n\\"),
            ..Default::default()
        }),
        Keycode::NON_US_HASH => Some(LayoutKey {
            tap: Label::new("NUHS"),
            ..Default::default()
        }),
        Keycode::SEMICOLON => Some(LayoutKey {
            tap: Label::new(":\n;"),
            ..Default::default()
        }),
        Keycode::SINGLE_QUOTE => Some(LayoutKey {
            tap: Label::new("\"\n\'"),
            ..Default::default()
        }),
        Keycode::GRAVE => Some(LayoutKey {
            tap: Label::new("~\n`"),
            ..Default::default()
        }),
        Keycode::COMMA => Some(LayoutKey {
            tap: Label::new("<\n,"),
            ..Default::default()
        }),
        Keycode::PERIOD => Some(LayoutKey {
            tap: Label::new(">\n."),
            ..Default::default()
        }),
        Keycode::SLASH => Some(LayoutKey {
            tap: Label::new("?\n/"),
            ..Default::default()
        }),
        Keycode::CAPSLOCK => Some(LayoutKey {
            tap: Label::with_short("Capslock", "Caps"),
            symbol: Some(egui_phosphor::regular::ARROW_FAT_LINE_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::F1 => Some(LayoutKey {
            tap: Label::new("F1"),
            ..Default::default()
        }),
        Keycode::F2 => Some(LayoutKey {
            tap: Label::new("F2"),
            ..Default::default()
        }),
        Keycode::F3 => Some(LayoutKey {
            tap: Label::new("F3"),
            ..Default::default()
        }),
        Keycode::F4 => Some(LayoutKey {
            tap: Label::new("F4"),
            ..Default::default()
        }),
        Keycode::F5 => Some(LayoutKey {
            tap: Label::new("F5"),
            ..Default::default()
        }),
        Keycode::F6 => Some(LayoutKey {
            tap: Label::new("F6"),
            ..Default::default()
        }),
        Keycode::F7 => Some(LayoutKey {
            tap: Label::new("F7"),
            ..Default::default()
        }),
        Keycode::F8 => Some(LayoutKey {
            tap: Label::new("F8"),
            ..Default::default()
        }),
        Keycode::F9 => Some(LayoutKey {
            tap: Label::new("F9"),
            ..Default::default()
        }),
        Keycode::F10 => Some(LayoutKey {
            tap: Label::new("F10"),
            ..Default::default()
        }),
        Keycode::F11 => Some(LayoutKey {
            tap: Label::new("F11"),
            ..Default::default()
        }),
        Keycode::F12 => Some(LayoutKey {
            tap: Label::new("F12"),
            ..Default::default()
        }),
        Keycode::PRINTSCREEN => Some(LayoutKey {
            tap: Label::with_short("Print Screen", "PrtSc"),
            ..Default::default()
        }),
        Keycode::SCROLLLOCK => Some(LayoutKey {
            tap: Label::with_short("Scroll Lock", "ScrLk"),
            ..Default::default()
        }),
        Keycode::PAUSE_BREAK => Some(LayoutKey {
            tap: Label::with_short("Pause", "Paus"),
            ..Default::default()
        }),
        Keycode::INSERT => Some(LayoutKey {
            tap: Label::with_short("Insert", "Ins"),
            ..Default::default()
        }),
        Keycode::HOME => Some(LayoutKey {
            tap: Label::new("Home"),
            ..Default::default()
        }),
        Keycode::PAGE_UP => Some(LayoutKey {
            tap: Label::with_short("Page Up", "PgUp"),
            ..Default::default()
        }),
        Keycode::DELETE => Some(LayoutKey {
            tap: Label::with_short("Delete", "Del"),
            ..Default::default()
        }),
        Keycode::END => Some(LayoutKey {
            tap: Label::new("End"),
            ..Default::default()
        }),
        Keycode::PAGE_DOWN => Some(LayoutKey {
            tap: Label::with_short("Page Down", "PgDn"),
            ..Default::default()
        }),
        Keycode::RIGHT_ARROW => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_RIGHT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::LEFT_ARROW => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_LEFT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::DOWN_ARROW => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_DOWN.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::UP_ARROW => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KP_NUMLOCK => Some(LayoutKey {
            tap: Label::with_short("Num\nLock", "NumLk"),
            ..Default::default()
        }),
        Keycode::KP_DIVIDE => Some(LayoutKey {
            tap: Label::new("÷"),
            ..Default::default()
        }),
        Keycode::KP_ASTERISK => Some(LayoutKey {
            tap: Label::new("×"),
            ..Default::default()
        }),
        Keycode::KP_SUBTRACT => Some(LayoutKey {
            tap: Label::new("-"),
            ..Default::default()
        }),
        Keycode::KP_PLUS => Some(LayoutKey {
            tap: Label::new("+"),
            ..Default::default()
        }),
        Keycode::KP_ENTER => Some(LayoutKey {
            tap: Label::new("Enter"),
            symbol: Some(egui_phosphor::regular::ARROW_ELBOW_DOWN_LEFT.to_string()),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_1 => Some(LayoutKey {
            tap: Label::new("1"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_2 => Some(LayoutKey {
            tap: Label::new("2"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_3 => Some(LayoutKey {
            tap: Label::new("3"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_4 => Some(LayoutKey {
            tap: Label::new("4"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_5 => Some(LayoutKey {
            tap: Label::new("5"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_6 => Some(LayoutKey {
            tap: Label::new("6"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_7 => Some(LayoutKey {
            tap: Label::new("7"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_8 => Some(LayoutKey {
            tap: Label::new("8"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_9 => Some(LayoutKey {
            tap: Label::new("9"),
            ..Default::default()
        }),
        Keycode::KP_NUMBER_0 => Some(LayoutKey {
            tap: Label::new("0"),
            ..Default::default()
        }),
        Keycode::KP_DOT => Some(LayoutKey {
            tap: Label::new("."),
            ..Default::default()
        }),
        Keycode::KP_LEFT_PARENTHESIS => Some(LayoutKey {
            tap: Label::new("("),
            ..Default::default()
        }),
        Keycode::KP_RIGHT_PARENTHESIS => Some(LayoutKey {
            tap: Label::new(")"),
            ..Default::default()
        }),
        Keycode::KP_CLEAR => Some(LayoutKey {
            tap: Label::new("Clear"),
            ..Default::default()
        }),
        Keycode::KP_COMMA => Some(LayoutKey {
            tap: Label::new(","),
            ..Default::default()
        }),
        Keycode::KP_EQUAL_AS400 => Some(LayoutKey {
            tap: Label::new("="),
            ..Default::default()
        }),
        Keycode::KP_EQUAL => Some(LayoutKey {
            tap: Label::new("="),
            ..Default::default()
        }),
        Keycode::K_CONTEXT_MENU => Some(LayoutKey {
            tap: Label::new("Menu"),
            symbol: Some(egui_phosphor::regular::LIST.to_string()),
            ..Default::default()
        }),
        Keycode::K_POWER => Some(LayoutKey {
            tap: Label::new("Power"),
            symbol: Some(egui_phosphor::regular::POWER.to_string()),
            ..Default::default()
        }),
        Keycode::F13 => Some(LayoutKey {
            tap: Label::new("F13"),
            ..Default::default()
        }),
        Keycode::F14 => Some(LayoutKey {
            tap: Label::new("F14"),
            ..Default::default()
        }),
        Keycode::F15 => Some(LayoutKey {
            tap: Label::new("F15"),
            ..Default::default()
        }),
        Keycode::F16 => Some(LayoutKey {
            tap: Label::new("F16"),
            ..Default::default()
        }),
        Keycode::F17 => Some(LayoutKey {
            tap: Label::new("F17"),
            ..Default::default()
        }),
        Keycode::F18 => Some(LayoutKey {
            tap: Label::new("F18"),
            ..Default::default()
        }),
        Keycode::F19 => Some(LayoutKey {
            tap: Label::new("F19"),
            ..Default::default()
        }),
        Keycode::F20 => Some(LayoutKey {
            tap: Label::new("F20"),
            ..Default::default()
        }),
        Keycode::F21 => Some(LayoutKey {
            tap: Label::new("F21"),
            ..Default::default()
        }),
        Keycode::F22 => Some(LayoutKey {
            tap: Label::new("F22"),
            ..Default::default()
        }),
        Keycode::F23 => Some(LayoutKey {
            tap: Label::new("F23"),
            ..Default::default()
        }),
        Keycode::F24 => Some(LayoutKey {
            tap: Label::new("F24"),
            ..Default::default()
        }),
        Keycode::K_EXECUTE => Some(LayoutKey {
            tap: Label::new("Exec"),
            ..Default::default()
        }),
        Keycode::K_HELP => Some(LayoutKey {
            tap: Label::new("Help"),
            ..Default::default()
        }),
        Keycode::K_MENU => Some(LayoutKey {
            tap: Label::new("Menu"),
            ..Default::default()
        }),
        Keycode::K_SELECT => Some(LayoutKey {
            tap: Label::new("Select"),
            ..Default::default()
        }),
        Keycode::K_STOP => Some(LayoutKey {
            tap: Label::new("Stop"),
            ..Default::default()
        }),
        Keycode::K_AGAIN => Some(LayoutKey {
            tap: Label::new("Again"),
            ..Default::default()
        }),
        Keycode::K_UNDO => Some(LayoutKey {
            tap: Label::new("Undo"),
            ..Default::default()
        }),
        Keycode::K_CUT => Some(LayoutKey {
            tap: Label::new("Cut"),
            ..Default::default()
        }),
        Keycode::K_COPY => Some(LayoutKey {
            tap: Label::new("Copy"),
            ..Default::default()
        }),
        Keycode::K_PASTE => Some(LayoutKey {
            tap: Label::new("Paste"),
            ..Default::default()
        }),
        Keycode::K_FIND => Some(LayoutKey {
            tap: Label::new("Find"),
            ..Default::default()
        }),
        Keycode::K_MUTE => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_X.to_string()),
            ..Default::default()
        }),
        Keycode::K_VOLUME_UP => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
            ..Default::default()
        }),
        Keycode::K_VOLUME_DOWN => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
            ..Default::default()
        }),
        Keycode::LOCKING_CAPS => Some(LayoutKey {
            tap: Label::with_short("Locking Caps Lock", "LCaps"),
            ..Default::default()
        }),
        Keycode::LOCKING_NUM => Some(LayoutKey {
            tap: Label::with_short("Locking Num Lock", "LNum"),
            ..Default::default()
        }),
        Keycode::LOCKING_SCROLL => Some(LayoutKey {
            tap: Label::with_short("Locking Scroll Lock", "LScrl"),
            ..Default::default()
        }),
        Keycode::INTERNATIONAL_1 => Some(LayoutKey {
            tap: Label::new("Int1"),
            ..Default::default()
        }),
        Keycode::INT_KATAKANAHIRAGANA => Some(LayoutKey {
            tap: Label::new("Int2"),
            ..Default::default()
        }),
        Keycode::INTERNATIONAL_3 => Some(LayoutKey {
            tap: Label::new("Int3"),
            ..Default::default()
        }),
        Keycode::INTERNATIONAL_4 => Some(LayoutKey {
            tap: Label::new("Int4"),
            ..Default::default()
        }),
        Keycode::INTERNATIONAL_5 => Some(LayoutKey {
            tap: Label::new("Int5"),
            ..Default::default()
        }),
        Keycode::INTERNATIONAL_6 => Some(LayoutKey {
            tap: Label::new("Int6"),
            ..Default::default()
        }),
        Keycode::INTERNATIONAL_7 => Some(LayoutKey {
            tap: Label::new("Int7"),
            ..Default::default()
        }),
        Keycode::INTERNATIONAL_8 => Some(LayoutKey {
            tap: Label::new("Int8"),
            ..Default::default()
        }),
        Keycode::INTERNATIONAL_9 => Some(LayoutKey {
            tap: Label::new("Int9"),
            ..Default::default()
        }),
        Keycode::LANG_HANGEUL => Some(LayoutKey {
            tap: Label::new("Lang1"),
            ..Default::default()
        }),
        Keycode::LANG_HANJA => Some(LayoutKey {
            tap: Label::new("Lang2"),
            ..Default::default()
        }),
        Keycode::LANG_KATAKANA => Some(LayoutKey {
            tap: Label::new("Lang3"),
            ..Default::default()
        }),
        Keycode::LANG_HIRAGANA => Some(LayoutKey {
            tap: Label::new("Lang4"),
            ..Default::default()
        }),
        Keycode::LANG_ZENKAKUHANKAKU => Some(LayoutKey {
            tap: Label::new("Lang5"),
            ..Default::default()
        }),
        Keycode::LANGUAGE_6 => Some(LayoutKey {
            tap: Label::new("Lang6"),
            ..Default::default()
        }),
        Keycode::LANGUAGE_7 => Some(LayoutKey {
            tap: Label::new("Lang7"),
            ..Default::default()
        }),
        Keycode::LANGUAGE_8 => Some(LayoutKey {
            tap: Label::new("Lang8"),
            ..Default::default()
        }),
        Keycode::LANGUAGE_9 => Some(LayoutKey {
            tap: Label::new("Lang9"),
            ..Default::default()
        }),
        Keycode::ALT_ERASE => Some(LayoutKey {
            tap: Label::new("Alt Erase"),
            ..Default::default()
        }),
        Keycode::ATTENTION => Some(LayoutKey {
            tap: Label::new("SysReq"),
            ..Default::default()
        }),
        Keycode::K_CANCEL => Some(LayoutKey {
            tap: Label::new("Cancel"),
            ..Default::default()
        }),
        Keycode::CLEAR => Some(LayoutKey {
            tap: Label::new("Clear"),
            ..Default::default()
        }),
        Keycode::PRIOR => Some(LayoutKey {
            tap: Label::new("Prior"),
            ..Default::default()
        }),
        Keycode::RETURN2 => Some(LayoutKey {
            tap: Label::new("Return"),
            ..Default::default()
        }),
        Keycode::SEPARATOR => Some(LayoutKey {
            tap: Label::new("Separator"),
            ..Default::default()
        }),
        Keycode::OUT => Some(LayoutKey {
            tap: Label::new("Out"),
            ..Default::default()
        }),
        Keycode::OPER => Some(LayoutKey {
            tap: Label::new("Oper"),
            ..Default::default()
        }),
        Keycode::CLEAR_AGAIN => Some(LayoutKey {
            tap: Label::new("Clear Again"),
            ..Default::default()
        }),
        Keycode::CRSEL => Some(LayoutKey {
            tap: Label::new("CrSel"),
            ..Default::default()
        }),
        Keycode::EXSEL => Some(LayoutKey {
            tap: Label::new("ExSel"),
            ..Default::default()
        }),
        Keycode::LEFT_CONTROL => Some(LayoutKey {
            tap: Label::new("Ctrl"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::LEFT_SHIFT => Some(LayoutKey {
            tap: Label::new("Shift"),
            symbol: Some(egui_phosphor::regular::ARROW_FAT_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::LEFT_ALT => Some(LayoutKey {
            tap: Label::new("Alt"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::LEFT_COMMAND => Some(LayoutKey {
            tap: Label::new("Win"),
            symbol: Some(egui_phosphor::regular::WINDOWS_LOGO.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::RIGHT_CONTROL => Some(LayoutKey {
            tap: Label::new("Ctrl"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::RIGHT_SHIFT => Some(LayoutKey {
            tap: Label::new("Shift"),
            symbol: Some(egui_phosphor::regular::ARROW_FAT_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::RIGHT_ALT => Some(LayoutKey {
            tap: Label::new("Alt"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::RIGHT_COMMAND => Some(LayoutKey {
            tap: Label::new("Win"),
            symbol: Some(egui_phosphor::regular::WINDOWS_LOGO.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        // These are keyboard-page HID usage codes that don't exist in QMK.
        Keycode::K_PLAY_PAUSE => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::PLAY_PAUSE.to_string()),
            ..Default::default()
        }),
        Keycode::K_STOP2 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::STOP.to_string()),
            ..Default::default()
        }),
        Keycode::K_PREVIOUS => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_BACK.to_string()),
            ..Default::default()
        }),
        Keycode::K_NEXT => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_FORWARD.to_string()),
            ..Default::default()
        }),
        Keycode::K_EJECT => Some(LayoutKey {
            tap: Label::with_short("Eject", "Ejct"),
            ..Default::default()
        }),
        Keycode::K_VOLUME_UP2 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
            ..Default::default()
        }),
        Keycode::K_VOLUME_DOWN2 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
            ..Default::default()
        }),
        Keycode::K_MUTE2 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_X.to_string()),
            ..Default::default()
        }),
        Keycode::K_WWW => Some(LayoutKey {
            tap: Label::new("WWW"),
            ..Default::default()
        }),
        Keycode::K_BACK => Some(LayoutKey {
            tap: Label::new("Back"),
            ..Default::default()
        }),
        Keycode::K_FORWARD => Some(LayoutKey {
            tap: Label::new("Forward"),
            ..Default::default()
        }),
        Keycode::K_STOP3 => Some(LayoutKey {
            tap: Label::new("Stop"),
            ..Default::default()
        }),
        Keycode::K_FIND2 => Some(LayoutKey {
            tap: Label::new("Find"),
            ..Default::default()
        }),
        Keycode::K_SCROLL_UP => Some(LayoutKey {
            tap: Label::with_short("Scroll Up", "Scr↑"),
            ..Default::default()
        }),
        Keycode::K_SCROLL_DOWN => Some(LayoutKey {
            tap: Label::with_short("Scroll Down", "Scr↓"),
            ..Default::default()
        }),
        Keycode::K_EDIT => Some(LayoutKey {
            tap: Label::new("Edit"),
            ..Default::default()
        }),
        Keycode::K_SLEEP => Some(LayoutKey {
            tap: Label::new("Sleep"),
            ..Default::default()
        }),
        Keycode::K_SCREENSAVER => Some(LayoutKey {
            tap: Label::with_short("Screensaver", "Lock"),
            ..Default::default()
        }),
        Keycode::K_REFRESH => Some(LayoutKey {
            tap: Label::new("Refresh"),
            ..Default::default()
        }),
        Keycode::K_CALCULATOR => Some(LayoutKey {
            tap: Label::new("Calc"),
            ..Default::default()
        }),
        Keycode::EXCLAMATION => Some(LayoutKey {
            tap: Label::new("!"),
            ..Default::default()
        }),
        Keycode::AT_SIGN => Some(LayoutKey {
            tap: Label::new("@"),
            ..Default::default()
        }),
        Keycode::POUND => Some(LayoutKey {
            tap: Label::new("#"),
            ..Default::default()
        }),
        Keycode::DOLLAR => Some(LayoutKey {
            tap: Label::new("$"),
            ..Default::default()
        }),
        Keycode::PERCENT => Some(LayoutKey {
            tap: Label::new("%"),
            ..Default::default()
        }),
        Keycode::CARET => Some(LayoutKey {
            tap: Label::new("^"),
            ..Default::default()
        }),
        Keycode::AMPERSAND => Some(LayoutKey {
            tap: Label::new("&"),
            ..Default::default()
        }),
        Keycode::ASTERISK => Some(LayoutKey {
            tap: Label::new("*"),
            ..Default::default()
        }),
        Keycode::UNDERSCORE => Some(LayoutKey {
            tap: Label::new("_"),
            ..Default::default()
        }),
        Keycode::PLUS => Some(LayoutKey {
            tap: Label::new("+"),
            ..Default::default()
        }),
        Keycode::PIPE => Some(LayoutKey {
            tap: Label::new("|"),
            ..Default::default()
        }),
        Keycode::TILDE2 => Some(LayoutKey {
            tap: Label::new("~"),
            ..Default::default()
        }),
        Keycode::COLON => Some(LayoutKey {
            tap: Label::new(":"),
            ..Default::default()
        }),
        Keycode::TILDE => Some(LayoutKey {
            tap: Label::new("~"),
            ..Default::default()
        }),
        Keycode::LESS_THAN => Some(LayoutKey {
            tap: Label::new("<"),
            ..Default::default()
        }),
        Keycode::QUESTION => Some(LayoutKey {
            tap: Label::new("?"),
            ..Default::default()
        }),
        Keycode::CLEAR2 => Some(LayoutKey {
            tap: Label::new("Clear"),
            ..Default::default()
        }),
        Keycode::PIPE2 => Some(LayoutKey {
            tap: Label::new("|"),
            ..Default::default()
        }),
        Keycode::C_POWER => Some(LayoutKey {
            tap: Label::new("Power"),
            ..Default::default()
        }),
        Keycode::C_RESET => Some(LayoutKey {
            tap: Label::new("Reset"),
            ..Default::default()
        }),
        Keycode::C_SLEEP => Some(LayoutKey {
            tap: Label::new("Sleep"),
            ..Default::default()
        }),
        Keycode::C_SLEEP_MODE => Some(LayoutKey {
            tap: Label::with_short("Sleep Mode", "Slp"),
            ..Default::default()
        }),
        Keycode::C_MENU => Some(LayoutKey {
            tap: Label::new("Menu"),
            ..Default::default()
        }),
        Keycode::C_MENU_SELECT => Some(LayoutKey {
            tap: Label::with_short("Menu Select", "MSel"),
            ..Default::default()
        }),
        Keycode::C_MENU_UP => Some(LayoutKey {
            tap: Label::with_short("Menu Up", "M↑"),
            ..Default::default()
        }),
        Keycode::C_MENU_DOWN => Some(LayoutKey {
            tap: Label::with_short("Menu Down", "M↓"),
            ..Default::default()
        }),
        Keycode::C_MENU_LEFT => Some(LayoutKey {
            tap: Label::with_short("Menu Left", "M←"),
            ..Default::default()
        }),
        Keycode::C_MENU_RIGHT => Some(LayoutKey {
            tap: Label::with_short("Menu Right", "M→"),
            ..Default::default()
        }),
        Keycode::C_MENU_ESCAPE => Some(LayoutKey {
            tap: Label::with_short("Menu Escape", "MEsc"),
            ..Default::default()
        }),
        Keycode::C_MENU_INCREASE => Some(LayoutKey {
            tap: Label::with_short("Menu Increase", "M+"),
            ..Default::default()
        }),
        Keycode::C_MENU_DECREASE => Some(LayoutKey {
            tap: Label::with_short("Menu Decrease", "M-"),
            ..Default::default()
        }),
        Keycode::C_DATA_ON_SCREEN => Some(LayoutKey {
            tap: Label::with_short("Data on Screen", "OSD"),
            ..Default::default()
        }),
        Keycode::C_SUBTITLES => Some(LayoutKey {
            tap: Label::with_short("Subtitles", "Sub"),
            ..Default::default()
        }),
        Keycode::C_SNAPSHOT => Some(LayoutKey {
            tap: Label::with_short("Snapshot", "Snap"),
            ..Default::default()
        }),
        Keycode::C_PIP => Some(LayoutKey {
            tap: Label::new("PIP"),
            ..Default::default()
        }),
        Keycode::C_RED_BUTTON => Some(LayoutKey {
            tap: Label::new("Red"),
            ..Default::default()
        }),
        Keycode::C_GREEN_BUTTON => Some(LayoutKey {
            tap: Label::new("Green"),
            ..Default::default()
        }),
        Keycode::C_BLUE_BUTTON => Some(LayoutKey {
            tap: Label::new("Blue"),
            ..Default::default()
        }),
        Keycode::C_YELLOW_BUTTON => Some(LayoutKey {
            tap: Label::new("Yellow"),
            ..Default::default()
        }),
        Keycode::C_ASPECT => Some(LayoutKey {
            tap: Label::with_short("Aspect", "Asp"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_STEP => Some(LayoutKey {
            tap: Label::with_short("Mode Step", "Step"),
            ..Default::default()
        }),
        Keycode::C_RECALL_LAST => Some(LayoutKey {
            tap: Label::with_short("Last Channel", "Last"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_TV => Some(LayoutKey {
            tap: Label::new("TV"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_WWW => Some(LayoutKey {
            tap: Label::new("WWW"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_DVD => Some(LayoutKey {
            tap: Label::new("DVD"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_PHONE => Some(LayoutKey {
            tap: Label::new("Phone"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_GAMES => Some(LayoutKey {
            tap: Label::new("Games"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_CD => Some(LayoutKey {
            tap: Label::new("CD"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_VCR => Some(LayoutKey {
            tap: Label::new("VCR"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_TUNER => Some(LayoutKey {
            tap: Label::new("Tuner"),
            ..Default::default()
        }),
        Keycode::C_QUIT => Some(LayoutKey {
            tap: Label::new("Quit"),
            ..Default::default()
        }),
        Keycode::C_HELP => Some(LayoutKey {
            tap: Label::new("Help"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_TAPE => Some(LayoutKey {
            tap: Label::new("Tape"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_CABLE => Some(LayoutKey {
            tap: Label::new("Cable"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_HOME => Some(LayoutKey {
            tap: Label::with_short("Media Home", "Home"),
            ..Default::default()
        }),
        Keycode::C_CHANNEL_INC => Some(LayoutKey {
            tap: Label::with_short("Channel +", "Ch+"),
            ..Default::default()
        }),
        Keycode::C_CHANNEL_DEC => Some(LayoutKey {
            tap: Label::with_short("Channel -", "Ch-"),
            ..Default::default()
        }),
        Keycode::C_MEDIA_VCR_PLUS => Some(LayoutKey {
            tap: Label::with_short("VCR Plus", "VCR+"),
            ..Default::default()
        }),
        Keycode::C_PLAY => Some(LayoutKey {
            tap: Label::new("Play"),
            ..Default::default()
        }),
        Keycode::C_PAUSE => Some(LayoutKey {
            tap: Label::new("Pause"),
            ..Default::default()
        }),
        Keycode::C_RECORD => Some(LayoutKey {
            tap: Label::with_short("Record", "Rec"),
            ..Default::default()
        }),
        Keycode::C_FAST_FORWARD => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::FAST_FORWARD.to_string()),
            ..Default::default()
        }),
        Keycode::C_REWIND => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::REWIND.to_string()),
            ..Default::default()
        }),
        Keycode::C_NEXT => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_FORWARD.to_string()),
            ..Default::default()
        }),
        Keycode::C_PREVIOUS => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_BACK.to_string()),
            ..Default::default()
        }),
        Keycode::C_STOP => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::STOP.to_string()),
            ..Default::default()
        }),
        Keycode::C_EJECT => Some(LayoutKey {
            tap: Label::with_short("Eject", "Ejct"),
            ..Default::default()
        }),
        Keycode::C_RANDOM_PLAY => Some(LayoutKey {
            tap: Label::with_short("Shuffle", "Shfl"),
            symbol: Some(egui_phosphor::regular::SHUFFLE.to_string()),
            ..Default::default()
        }),
        Keycode::C_REPEAT => Some(LayoutKey {
            tap: Label::with_short("Repeat", "Rpt"),
            symbol: Some(egui_phosphor::regular::REPEAT.to_string()),
            ..Default::default()
        }),
        Keycode::C_SLOW_TRACKING => Some(LayoutKey {
            tap: Label::new("Slow"),
            ..Default::default()
        }),
        Keycode::C_STOP_EJECT => Some(LayoutKey {
            tap: Label::with_short("Stop/Eject", "StEj"),
            ..Default::default()
        }),
        Keycode::C_PLAY_PAUSE => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::PLAY_PAUSE.to_string()),
            ..Default::default()
        }),
        Keycode::C_VOICE_COMMAND => Some(LayoutKey {
            tap: Label::with_short("Voice Command", "Voice"),
            symbol: Some(egui_phosphor::regular::MICROPHONE.to_string()),
            ..Default::default()
        }),
        Keycode::C_MUTE => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_X.to_string()),
            ..Default::default()
        }),
        Keycode::C_BASS_BOOST => Some(LayoutKey {
            tap: Label::with_short("Bass Boost", "Bass"),
            ..Default::default()
        }),
        Keycode::C_VOLUME_UP => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
            ..Default::default()
        }),
        Keycode::C_VOLUME_DOWN => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
            ..Default::default()
        }),
        Keycode::C_SLOW => Some(LayoutKey {
            tap: Label::new("Slow"),
            ..Default::default()
        }),
        Keycode::C_AL_WORD => Some(LayoutKey {
            tap: Label::new("Word"),
            ..Default::default()
        }),
        Keycode::C_AL_TEXT_EDITOR => Some(LayoutKey {
            tap: Label::with_short("Text Editor", "Edit"),
            ..Default::default()
        }),
        Keycode::C_AL_SPREADSHEET => Some(LayoutKey {
            tap: Label::with_short("Spreadsheet", "Sheet"),
            ..Default::default()
        }),
        Keycode::C_AL_DATABASE => Some(LayoutKey {
            tap: Label::new("DB"),
            ..Default::default()
        }),
        Keycode::C_AL_EMAIL => Some(LayoutKey {
            tap: Label::new("Mail"),
            ..Default::default()
        }),
        Keycode::C_AL_NEWS => Some(LayoutKey {
            tap: Label::new("News"),
            ..Default::default()
        }),
        Keycode::C_AL_VOICEMAIL => Some(LayoutKey {
            tap: Label::with_short("Voicemail", "VMail"),
            ..Default::default()
        }),
        Keycode::C_AL_CALENDAR => Some(LayoutKey {
            tap: Label::with_short("Calendar", "Cal"),
            ..Default::default()
        }),
        Keycode::C_AL_JOURNAL => Some(LayoutKey {
            tap: Label::with_short("Journal", "Jrnl"),
            ..Default::default()
        }),
        Keycode::C_AL_FINANCE => Some(LayoutKey {
            tap: Label::with_short("Finance", "Fin"),
            ..Default::default()
        }),
        Keycode::C_AL_CALCULATOR => Some(LayoutKey {
            tap: Label::new("Calc"),
            ..Default::default()
        }),
        Keycode::C_AL_WWW => Some(LayoutKey {
            tap: Label::new("WWW"),
            ..Default::default()
        }),
        Keycode::C_AL_NETWORK_CHAT => Some(LayoutKey {
            tap: Label::new("Chat"),
            ..Default::default()
        }),
        Keycode::C_AL_LOGOFF => Some(LayoutKey {
            tap: Label::with_short("Log Off", "LogOff"),
            ..Default::default()
        }),
        Keycode::C_AL_CONTROL_PANEL => Some(LayoutKey {
            tap: Label::with_short("Control Panel", "Ctrl P"),
            ..Default::default()
        }),
        Keycode::C_AL_HELP => Some(LayoutKey {
            tap: Label::new("Help"),
            ..Default::default()
        }),
        Keycode::C_AL_DOCUMENTS => Some(LayoutKey {
            tap: Label::new("Docs"),
            ..Default::default()
        }),
        Keycode::C_AL_SPELLCHECK => Some(LayoutKey {
            tap: Label::with_short("Spellcheck", "Spell"),
            ..Default::default()
        }),
        Keycode::C_AL_SCREEN_SAVER => Some(LayoutKey {
            tap: Label::with_short("Screen Saver", "ScrSv"),
            ..Default::default()
        }),
        Keycode::C_AL_FILE_BROWSER => Some(LayoutKey {
            tap: Label::new("Files"),
            ..Default::default()
        }),
        Keycode::C_AL_IMAGE_BROWSER => Some(LayoutKey {
            tap: Label::new("Images"),
            ..Default::default()
        }),
        Keycode::C_AL_AUDIO_BROWSER => Some(LayoutKey {
            tap: Label::new("Audio"),
            ..Default::default()
        }),
        Keycode::C_AL_MOVIE_BROWSER => Some(LayoutKey {
            tap: Label::new("Movies"),
            ..Default::default()
        }),
        Keycode::C_AC_NEW => Some(LayoutKey {
            tap: Label::new("New"),
            ..Default::default()
        }),
        Keycode::C_AC_OPEN => Some(LayoutKey {
            tap: Label::new("Open"),
            ..Default::default()
        }),
        Keycode::C_AC_CLOSE => Some(LayoutKey {
            tap: Label::new("Close"),
            ..Default::default()
        }),
        Keycode::C_AC_EXIT => Some(LayoutKey {
            tap: Label::new("Exit"),
            ..Default::default()
        }),
        Keycode::C_AC_SAVE => Some(LayoutKey {
            tap: Label::new("Save"),
            ..Default::default()
        }),
        Keycode::C_AC_PRINT => Some(LayoutKey {
            tap: Label::new("Print"),
            ..Default::default()
        }),
        Keycode::C_AC_PROPERTIES => Some(LayoutKey {
            tap: Label::with_short("Properties", "Props"),
            ..Default::default()
        }),
        Keycode::C_AC_UNDO => Some(LayoutKey {
            tap: Label::new("Undo"),
            ..Default::default()
        }),
        Keycode::C_AC_COPY => Some(LayoutKey {
            tap: Label::new("Copy"),
            ..Default::default()
        }),
        Keycode::C_AC_CUT => Some(LayoutKey {
            tap: Label::new("Cut"),
            ..Default::default()
        }),
        Keycode::C_AC_PASTE => Some(LayoutKey {
            tap: Label::new("Paste"),
            ..Default::default()
        }),
        Keycode::C_AC_FIND => Some(LayoutKey {
            tap: Label::new("Find"),
            ..Default::default()
        }),
        Keycode::C_AC_SEARCH => Some(LayoutKey {
            tap: Label::new("Search"),
            ..Default::default()
        }),
        Keycode::C_AC_GOTO => Some(LayoutKey {
            tap: Label::with_short("Go To", "GoTo"),
            ..Default::default()
        }),
        Keycode::C_AC_HOME => Some(LayoutKey {
            tap: Label::new("Home"),
            ..Default::default()
        }),
        Keycode::C_AC_BACK => Some(LayoutKey {
            tap: Label::new("Back"),
            ..Default::default()
        }),
        Keycode::C_AC_FORWARD => Some(LayoutKey {
            tap: Label::new("Forward"),
            ..Default::default()
        }),
        Keycode::C_AC_STOP => Some(LayoutKey {
            tap: Label::new("Stop"),
            ..Default::default()
        }),
        Keycode::C_AC_REFRESH => Some(LayoutKey {
            tap: Label::new("Refresh"),
            ..Default::default()
        }),
        Keycode::C_AC_FAVOURITES => Some(LayoutKey {
            tap: Label::new("Favorites"),
            ..Default::default()
        }),
        Keycode::C_AC_ZOOM_IN => Some(LayoutKey {
            tap: Label::with_short("Zoom In", "Z+"),
            ..Default::default()
        }),
        Keycode::C_AC_ZOOM_OUT => Some(LayoutKey {
            tap: Label::with_short("Zoom Out", "Z-"),
            ..Default::default()
        }),
        Keycode::C_AC_ZOOM => Some(LayoutKey {
            tap: Label::new("Zoom"),
            ..Default::default()
        }),
        Keycode::C_AC_VIEW_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("View Toggle", "View"),
            ..Default::default()
        }),
        Keycode::C_AC_SCROLL_UP => Some(LayoutKey {
            tap: Label::with_short("Scroll Up", "Scr↑"),
            ..Default::default()
        }),
        Keycode::C_AC_SCROLL_DOWN => Some(LayoutKey {
            tap: Label::with_short("Scroll Down", "Scr↓"),
            ..Default::default()
        }),
        Keycode::C_AC_EDIT => Some(LayoutKey {
            tap: Label::new("Edit"),
            ..Default::default()
        }),
        Keycode::C_AC_CANCEL => Some(LayoutKey {
            tap: Label::new("Cancel"),
            ..Default::default()
        }),
        Keycode::C_AC_INSERT => Some(LayoutKey {
            tap: Label::with_short("Insert", "Ins"),
            ..Default::default()
        }),
        Keycode::C_AC_DEL => Some(LayoutKey {
            tap: Label::with_short("Delete", "Del"),
            ..Default::default()
        }),
        Keycode::C_AC_REDO => Some(LayoutKey {
            tap: Label::new("Redo"),
            ..Default::default()
        }),
        Keycode::C_AC_REPLY => Some(LayoutKey {
            tap: Label::new("Reply"),
            ..Default::default()
        }),
        Keycode::C_AC_FORWARD_MAIL => Some(LayoutKey {
            tap: Label::with_short("Forward Mail", "Fwd"),
            ..Default::default()
        }),
        Keycode::C_AC_SEND => Some(LayoutKey {
            tap: Label::new("Send"),
            ..Default::default()
        }),
        Keycode::C_AC_NEXT_KEYBOARD_LAYOUT_SELECT => Some(LayoutKey {
            tap: Label::new("Globe"),
            symbol: Some(egui_phosphor::regular::GLOBE.to_string()),
            ..Default::default()
        }),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}
