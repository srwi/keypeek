use crate::layout_key::modifier_symbols::*;
use crate::layout_key::{KeycodeKind, Label, LayoutKey};

/// Static lookup for USB HID Usage Page 0x07 (Keyboard/Keypad).
pub(crate) fn hid_keyboard_key_static(usage_id: u16) -> Option<LayoutKey> {
    match usage_id {
        0x00 => Some(LayoutKey {
            tap: Label::new(""),
            ..Default::default()
        }),
        0x01 => Some(LayoutKey {
            tap: Label::new(""),
            ..Default::default()
        }),
        0x04 => Some(LayoutKey {
            tap: Label::new("A"),
            ..Default::default()
        }),
        0x05 => Some(LayoutKey {
            tap: Label::new("B"),
            ..Default::default()
        }),
        0x06 => Some(LayoutKey {
            tap: Label::new("C"),
            ..Default::default()
        }),
        0x07 => Some(LayoutKey {
            tap: Label::new("D"),
            ..Default::default()
        }),
        0x08 => Some(LayoutKey {
            tap: Label::new("E"),
            ..Default::default()
        }),
        0x09 => Some(LayoutKey {
            tap: Label::new("F"),
            ..Default::default()
        }),
        0x0A => Some(LayoutKey {
            tap: Label::new("G"),
            ..Default::default()
        }),
        0x0B => Some(LayoutKey {
            tap: Label::new("H"),
            ..Default::default()
        }),
        0x0C => Some(LayoutKey {
            tap: Label::new("I"),
            ..Default::default()
        }),
        0x0D => Some(LayoutKey {
            tap: Label::new("J"),
            ..Default::default()
        }),
        0x0E => Some(LayoutKey {
            tap: Label::new("K"),
            ..Default::default()
        }),
        0x0F => Some(LayoutKey {
            tap: Label::new("L"),
            ..Default::default()
        }),
        0x10 => Some(LayoutKey {
            tap: Label::new("M"),
            ..Default::default()
        }),
        0x11 => Some(LayoutKey {
            tap: Label::new("N"),
            ..Default::default()
        }),
        0x12 => Some(LayoutKey {
            tap: Label::new("O"),
            ..Default::default()
        }),
        0x13 => Some(LayoutKey {
            tap: Label::new("P"),
            ..Default::default()
        }),
        0x14 => Some(LayoutKey {
            tap: Label::new("Q"),
            ..Default::default()
        }),
        0x15 => Some(LayoutKey {
            tap: Label::new("R"),
            ..Default::default()
        }),
        0x16 => Some(LayoutKey {
            tap: Label::new("S"),
            ..Default::default()
        }),
        0x17 => Some(LayoutKey {
            tap: Label::new("T"),
            ..Default::default()
        }),
        0x18 => Some(LayoutKey {
            tap: Label::new("U"),
            ..Default::default()
        }),
        0x19 => Some(LayoutKey {
            tap: Label::new("V"),
            ..Default::default()
        }),
        0x1A => Some(LayoutKey {
            tap: Label::new("W"),
            ..Default::default()
        }),
        0x1B => Some(LayoutKey {
            tap: Label::new("X"),
            ..Default::default()
        }),
        0x1C => Some(LayoutKey {
            tap: Label::new("Y"),
            ..Default::default()
        }),
        0x1D => Some(LayoutKey {
            tap: Label::new("Z"),
            ..Default::default()
        }),
        0x1E => Some(LayoutKey {
            tap: Label::new("1"),
            shifted: Some("!".to_string()),
            ..Default::default()
        }),
        0x1F => Some(LayoutKey {
            tap: Label::new("2"),
            shifted: Some("@".to_string()),
            ..Default::default()
        }),
        0x20 => Some(LayoutKey {
            tap: Label::new("3"),
            shifted: Some("#".to_string()),
            ..Default::default()
        }),
        0x21 => Some(LayoutKey {
            tap: Label::new("4"),
            shifted: Some("$".to_string()),
            ..Default::default()
        }),
        0x22 => Some(LayoutKey {
            tap: Label::new("5"),
            shifted: Some("%".to_string()),
            ..Default::default()
        }),
        0x23 => Some(LayoutKey {
            tap: Label::new("6"),
            shifted: Some("^".to_string()),
            ..Default::default()
        }),
        0x24 => Some(LayoutKey {
            tap: Label::new("7"),
            shifted: Some("&".to_string()),
            ..Default::default()
        }),
        0x25 => Some(LayoutKey {
            tap: Label::new("8"),
            shifted: Some("*".to_string()),
            ..Default::default()
        }),
        0x26 => Some(LayoutKey {
            tap: Label::new("9"),
            shifted: Some("(".to_string()),
            ..Default::default()
        }),
        0x27 => Some(LayoutKey {
            tap: Label::new("0"),
            shifted: Some(")".to_string()),
            ..Default::default()
        }),
        0x28 => Some(LayoutKey {
            tap: Label::new("Enter"),
            symbol: Some(egui_phosphor::regular::ARROW_ELBOW_DOWN_LEFT.to_string()),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        0x29 => Some(LayoutKey {
            tap: Label::new("Esc"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        0x2A => Some(LayoutKey {
            tap: Label::new("Backspace"),
            symbol: Some(egui_phosphor::regular::BACKSPACE.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        0x2B => Some(LayoutKey {
            tap: Label::new("Tab"),
            symbol: Some(egui_phosphor::regular::ARROWS_LEFT_RIGHT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        0x2C => Some(LayoutKey {
            tap: Label::with_short("Space", "Spc"),
            ..Default::default()
        }),
        0x2D => Some(LayoutKey {
            tap: Label::new("-"),
            shifted: Some("_".to_string()),
            ..Default::default()
        }),
        0x2E => Some(LayoutKey {
            tap: Label::new("="),
            shifted: Some("+".to_string()),
            ..Default::default()
        }),
        0x2F => Some(LayoutKey {
            tap: Label::new("["),
            shifted: Some("{".to_string()),
            ..Default::default()
        }),
        0x30 => Some(LayoutKey {
            tap: Label::new("]"),
            shifted: Some("}".to_string()),
            ..Default::default()
        }),
        0x31 => Some(LayoutKey {
            tap: Label::new("\\"),
            shifted: Some("|".to_string()),
            ..Default::default()
        }),
        0x32 => Some(LayoutKey {
            tap: Label::new(
                crate::os_layout::base_char(0x32).unwrap_or_else(|| "NUHS".to_string()),
            ),
            shifted: crate::os_layout::shifted_char(0x32),
            ..Default::default()
        }),
        0x33 => Some(LayoutKey {
            tap: Label::new(";"),
            shifted: Some(":".to_string()),
            ..Default::default()
        }),
        0x34 => Some(LayoutKey {
            tap: Label::new("\'"),
            shifted: Some("\"".to_string()),
            ..Default::default()
        }),
        0x35 => Some(LayoutKey {
            tap: Label::new("`"),
            shifted: Some("~".to_string()),
            ..Default::default()
        }),
        0x36 => Some(LayoutKey {
            tap: Label::new(","),
            shifted: Some("<".to_string()),
            ..Default::default()
        }),
        0x37 => Some(LayoutKey {
            tap: Label::new("."),
            shifted: Some(">".to_string()),
            ..Default::default()
        }),
        0x38 => Some(LayoutKey {
            tap: Label::new("/"),
            shifted: Some("?".to_string()),
            ..Default::default()
        }),
        0x39 => Some(LayoutKey {
            tap: Label::with_short("Capslock", "Caps"),
            symbol: Some(egui_phosphor::regular::ARROW_FAT_LINE_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        0x3A => Some(LayoutKey {
            tap: Label::new("F1"),
            ..Default::default()
        }),
        0x3B => Some(LayoutKey {
            tap: Label::new("F2"),
            ..Default::default()
        }),
        0x3C => Some(LayoutKey {
            tap: Label::new("F3"),
            ..Default::default()
        }),
        0x3D => Some(LayoutKey {
            tap: Label::new("F4"),
            ..Default::default()
        }),
        0x3E => Some(LayoutKey {
            tap: Label::new("F5"),
            ..Default::default()
        }),
        0x3F => Some(LayoutKey {
            tap: Label::new("F6"),
            ..Default::default()
        }),
        0x40 => Some(LayoutKey {
            tap: Label::new("F7"),
            ..Default::default()
        }),
        0x41 => Some(LayoutKey {
            tap: Label::new("F8"),
            ..Default::default()
        }),
        0x42 => Some(LayoutKey {
            tap: Label::new("F9"),
            ..Default::default()
        }),
        0x43 => Some(LayoutKey {
            tap: Label::new("F10"),
            ..Default::default()
        }),
        0x44 => Some(LayoutKey {
            tap: Label::new("F11"),
            ..Default::default()
        }),
        0x45 => Some(LayoutKey {
            tap: Label::new("F12"),
            ..Default::default()
        }),
        0x46 => Some(LayoutKey {
            tap: Label::with_short("Print Screen", "PrtSc"),
            ..Default::default()
        }),
        0x47 => Some(LayoutKey {
            tap: Label::with_short("Scroll Lock", "ScrLk"),
            ..Default::default()
        }),
        0x48 => Some(LayoutKey {
            tap: Label::with_short("Pause", "Paus"),
            ..Default::default()
        }),
        0x49 => Some(LayoutKey {
            tap: Label::with_short("Insert", "Ins"),
            ..Default::default()
        }),
        0x4A => Some(LayoutKey {
            tap: Label::new("Home"),
            ..Default::default()
        }),
        0x4B => Some(LayoutKey {
            tap: Label::with_short("Page Up", "PgUp"),
            ..Default::default()
        }),
        0x4C => Some(LayoutKey {
            tap: Label::with_short("Delete", "Del"),
            ..Default::default()
        }),
        0x4D => Some(LayoutKey {
            tap: Label::new("End"),
            ..Default::default()
        }),
        0x4E => Some(LayoutKey {
            tap: Label::with_short("Page Down", "PgDn"),
            ..Default::default()
        }),
        0x4F => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_RIGHT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        0x50 => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_LEFT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        0x51 => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_DOWN.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        0x52 => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        0x53 => Some(LayoutKey {
            tap: Label::with_short("Num Lock", "NumLk"),
            ..Default::default()
        }),
        0x54 => Some(LayoutKey {
            tap: Label::new("÷"),
            ..Default::default()
        }),
        0x55 => Some(LayoutKey {
            tap: Label::new("×"),
            ..Default::default()
        }),
        0x56 => Some(LayoutKey {
            tap: Label::new("-"),
            ..Default::default()
        }),
        0x57 => Some(LayoutKey {
            tap: Label::new("+"),
            ..Default::default()
        }),
        0x58 => Some(LayoutKey {
            tap: Label::new("Enter"),
            symbol: Some(egui_phosphor::regular::ARROW_ELBOW_DOWN_LEFT.to_string()),
            ..Default::default()
        }),
        0x59 => Some(LayoutKey {
            tap: Label::new("1"),
            ..Default::default()
        }),
        0x5A => Some(LayoutKey {
            tap: Label::new("2"),
            ..Default::default()
        }),
        0x5B => Some(LayoutKey {
            tap: Label::new("3"),
            ..Default::default()
        }),
        0x5C => Some(LayoutKey {
            tap: Label::new("4"),
            ..Default::default()
        }),
        0x5D => Some(LayoutKey {
            tap: Label::new("5"),
            ..Default::default()
        }),
        0x5E => Some(LayoutKey {
            tap: Label::new("6"),
            ..Default::default()
        }),
        0x5F => Some(LayoutKey {
            tap: Label::new("7"),
            ..Default::default()
        }),
        0x60 => Some(LayoutKey {
            tap: Label::new("8"),
            ..Default::default()
        }),
        0x61 => Some(LayoutKey {
            tap: Label::new("9"),
            ..Default::default()
        }),
        0x62 => Some(LayoutKey {
            tap: Label::new("0"),
            ..Default::default()
        }),
        0x63 => Some(LayoutKey {
            tap: Label::new("."),
            ..Default::default()
        }),
        0x64 => Some(LayoutKey {
            tap: Label::new(
                crate::os_layout::base_char(0x64).unwrap_or_else(|| "NUBS".to_string()),
            ),
            shifted: crate::os_layout::shifted_char(0x64),
            ..Default::default()
        }),
        0x65 => Some(LayoutKey {
            tap: Label::new("Menu"),
            symbol: Some(egui_phosphor::regular::LIST.to_string()),
            ..Default::default()
        }),
        0x66 => Some(LayoutKey {
            tap: Label::new("Power"),
            symbol: Some(egui_phosphor::regular::POWER.to_string()),
            ..Default::default()
        }),
        0x67 => Some(LayoutKey {
            tap: Label::new("="),
            ..Default::default()
        }),
        0x68 => Some(LayoutKey {
            tap: Label::new("F13"),
            ..Default::default()
        }),
        0x69 => Some(LayoutKey {
            tap: Label::new("F14"),
            ..Default::default()
        }),
        0x6A => Some(LayoutKey {
            tap: Label::new("F15"),
            ..Default::default()
        }),
        0x6B => Some(LayoutKey {
            tap: Label::new("F16"),
            ..Default::default()
        }),
        0x6C => Some(LayoutKey {
            tap: Label::new("F17"),
            ..Default::default()
        }),
        0x6D => Some(LayoutKey {
            tap: Label::new("F18"),
            ..Default::default()
        }),
        0x6E => Some(LayoutKey {
            tap: Label::new("F19"),
            ..Default::default()
        }),
        0x6F => Some(LayoutKey {
            tap: Label::new("F20"),
            ..Default::default()
        }),
        0x70 => Some(LayoutKey {
            tap: Label::new("F21"),
            ..Default::default()
        }),
        0x71 => Some(LayoutKey {
            tap: Label::new("F22"),
            ..Default::default()
        }),
        0x72 => Some(LayoutKey {
            tap: Label::new("F23"),
            ..Default::default()
        }),
        0x73 => Some(LayoutKey {
            tap: Label::new("F24"),
            ..Default::default()
        }),
        0x74 => Some(LayoutKey {
            tap: Label::new("Exec"),
            ..Default::default()
        }),
        0x75 => Some(LayoutKey {
            tap: Label::new("Help"),
            ..Default::default()
        }),
        0x76 => Some(LayoutKey {
            tap: Label::new("Menu"),
            ..Default::default()
        }),
        0x77 => Some(LayoutKey {
            tap: Label::new("Select"),
            ..Default::default()
        }),
        0x78 => Some(LayoutKey {
            tap: Label::new("Stop"),
            ..Default::default()
        }),
        0x79 => Some(LayoutKey {
            tap: Label::new("Again"),
            ..Default::default()
        }),
        0x7A => Some(LayoutKey {
            tap: Label::new("Undo"),
            ..Default::default()
        }),
        0x7B => Some(LayoutKey {
            tap: Label::new("Cut"),
            ..Default::default()
        }),
        0x7C => Some(LayoutKey {
            tap: Label::new("Copy"),
            ..Default::default()
        }),
        0x7D => Some(LayoutKey {
            tap: Label::new("Paste"),
            ..Default::default()
        }),
        0x7E => Some(LayoutKey {
            tap: Label::new("Find"),
            ..Default::default()
        }),
        0x7F => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_X.to_string()),
            ..Default::default()
        }),
        0x80 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
            ..Default::default()
        }),
        0x81 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
            ..Default::default()
        }),
        0x82 => Some(LayoutKey {
            tap: Label::with_short("Locking Caps Lock", "LCaps"),
            ..Default::default()
        }),
        0x83 => Some(LayoutKey {
            tap: Label::with_short("Locking Num Lock", "LNum"),
            ..Default::default()
        }),
        0x84 => Some(LayoutKey {
            tap: Label::with_short("Locking Scroll Lock", "LScrl"),
            ..Default::default()
        }),
        0x85 => Some(LayoutKey {
            tap: Label::new(","),
            ..Default::default()
        }),
        0x86 => Some(LayoutKey {
            tap: Label::new("="),
            ..Default::default()
        }),
        0x87 => Some(LayoutKey {
            tap: Label::new("Int1"),
            ..Default::default()
        }),
        0x88 => Some(LayoutKey {
            tap: Label::new("Int2"),
            ..Default::default()
        }),
        0x89 => Some(LayoutKey {
            tap: Label::new("Int3"),
            ..Default::default()
        }),
        0x8A => Some(LayoutKey {
            tap: Label::new("Int4"),
            ..Default::default()
        }),
        0x8B => Some(LayoutKey {
            tap: Label::new("Int5"),
            ..Default::default()
        }),
        0x8C => Some(LayoutKey {
            tap: Label::new("Int6"),
            ..Default::default()
        }),
        0x8D => Some(LayoutKey {
            tap: Label::new("Int7"),
            ..Default::default()
        }),
        0x8E => Some(LayoutKey {
            tap: Label::new("Int8"),
            ..Default::default()
        }),
        0x8F => Some(LayoutKey {
            tap: Label::new("Int9"),
            ..Default::default()
        }),
        0x90 => Some(LayoutKey {
            tap: Label::new("Lang1"),
            ..Default::default()
        }),
        0x91 => Some(LayoutKey {
            tap: Label::new("Lang2"),
            ..Default::default()
        }),
        0x92 => Some(LayoutKey {
            tap: Label::new("Lang3"),
            ..Default::default()
        }),
        0x93 => Some(LayoutKey {
            tap: Label::new("Lang4"),
            ..Default::default()
        }),
        0x94 => Some(LayoutKey {
            tap: Label::new("Lang5"),
            ..Default::default()
        }),
        0x95 => Some(LayoutKey {
            tap: Label::new("Lang6"),
            ..Default::default()
        }),
        0x96 => Some(LayoutKey {
            tap: Label::new("Lang7"),
            ..Default::default()
        }),
        0x97 => Some(LayoutKey {
            tap: Label::new("Lang8"),
            ..Default::default()
        }),
        0x98 => Some(LayoutKey {
            tap: Label::new("Lang9"),
            ..Default::default()
        }),
        0x99 => Some(LayoutKey {
            tap: Label::new("Alt Erase"),
            ..Default::default()
        }),
        0x9A => Some(LayoutKey {
            tap: Label::new("SysReq"),
            ..Default::default()
        }),
        0x9B => Some(LayoutKey {
            tap: Label::new("Cancel"),
            ..Default::default()
        }),
        0x9C => Some(LayoutKey {
            tap: Label::new("Clear"),
            ..Default::default()
        }),
        0x9D => Some(LayoutKey {
            tap: Label::new("Prior"),
            ..Default::default()
        }),
        0x9E => Some(LayoutKey {
            tap: Label::new("Return"),
            ..Default::default()
        }),
        0x9F => Some(LayoutKey {
            tap: Label::new("Separator"),
            ..Default::default()
        }),
        0xA0 => Some(LayoutKey {
            tap: Label::new("Out"),
            ..Default::default()
        }),
        0xA1 => Some(LayoutKey {
            tap: Label::new("Oper"),
            ..Default::default()
        }),
        0xA2 => Some(LayoutKey {
            tap: Label::new("Clear Again"),
            ..Default::default()
        }),
        0xA3 => Some(LayoutKey {
            tap: Label::new("CrSel"),
            ..Default::default()
        }),
        0xA4 => Some(LayoutKey {
            tap: Label::new("ExSel"),
            ..Default::default()
        }),
        0xB0 => Some(LayoutKey {
            tap: Label::new("00"),
            ..Default::default()
        }),
        0xB1 => Some(LayoutKey {
            tap: Label::new("000"),
            ..Default::default()
        }),
        0xB2 => Some(LayoutKey {
            tap: Label::new(","),
            ..Default::default()
        }),
        0xB3 => Some(LayoutKey {
            tap: Label::new("."),
            ..Default::default()
        }),
        0xB4 => Some(LayoutKey {
            tap: Label::new("$"),
            ..Default::default()
        }),
        0xB5 => Some(LayoutKey {
            tap: Label::new("¢"),
            ..Default::default()
        }),
        0xB6 => Some(LayoutKey {
            tap: Label::new("("),
            ..Default::default()
        }),
        0xB7 => Some(LayoutKey {
            tap: Label::new(")"),
            ..Default::default()
        }),
        0xB8 => Some(LayoutKey {
            tap: Label::new("{"),
            ..Default::default()
        }),
        0xB9 => Some(LayoutKey {
            tap: Label::new("}"),
            ..Default::default()
        }),
        0xBA => Some(LayoutKey {
            tap: Label::new("Tab"),
            ..Default::default()
        }),
        0xBB => Some(LayoutKey {
            tap: Label::new("Backspace"),
            ..Default::default()
        }),
        0xBC => Some(LayoutKey {
            tap: Label::new("A"),
            ..Default::default()
        }),
        0xBD => Some(LayoutKey {
            tap: Label::new("B"),
            ..Default::default()
        }),
        0xBE => Some(LayoutKey {
            tap: Label::new("C"),
            ..Default::default()
        }),
        0xBF => Some(LayoutKey {
            tap: Label::new("D"),
            ..Default::default()
        }),
        0xC0 => Some(LayoutKey {
            tap: Label::new("E"),
            ..Default::default()
        }),
        0xC1 => Some(LayoutKey {
            tap: Label::new("F"),
            ..Default::default()
        }),
        0xC2 => Some(LayoutKey {
            tap: Label::new("XOR"),
            ..Default::default()
        }),
        0xC3 => Some(LayoutKey {
            tap: Label::new("^"),
            ..Default::default()
        }),
        0xC4 => Some(LayoutKey {
            tap: Label::new("%"),
            ..Default::default()
        }),
        0xC5 => Some(LayoutKey {
            tap: Label::new("<"),
            ..Default::default()
        }),
        0xC6 => Some(LayoutKey {
            tap: Label::new(">"),
            ..Default::default()
        }),
        0xC7 => Some(LayoutKey {
            tap: Label::new("&"),
            ..Default::default()
        }),
        0xC8 => Some(LayoutKey {
            tap: Label::new("&&"),
            ..Default::default()
        }),
        0xC9 => Some(LayoutKey {
            tap: Label::new("|"),
            ..Default::default()
        }),
        0xCA => Some(LayoutKey {
            tap: Label::new("||"),
            ..Default::default()
        }),
        0xCB => Some(LayoutKey {
            tap: Label::new(":"),
            ..Default::default()
        }),
        0xCC => Some(LayoutKey {
            tap: Label::new("#"),
            ..Default::default()
        }),
        0xCD => Some(LayoutKey {
            tap: Label::new("Space"),
            ..Default::default()
        }),
        0xCE => Some(LayoutKey {
            tap: Label::new("@"),
            ..Default::default()
        }),
        0xCF => Some(LayoutKey {
            tap: Label::new("!"),
            ..Default::default()
        }),
        0xD0 => Some(LayoutKey {
            tap: Label::new("Mem Store"),
            ..Default::default()
        }),
        0xD1 => Some(LayoutKey {
            tap: Label::new("Mem Recall"),
            ..Default::default()
        }),
        0xD2 => Some(LayoutKey {
            tap: Label::new("Mem Clear"),
            ..Default::default()
        }),
        0xD3 => Some(LayoutKey {
            tap: Label::new("Mem Add"),
            ..Default::default()
        }),
        0xD4 => Some(LayoutKey {
            tap: Label::new("Mem Sub"),
            ..Default::default()
        }),
        0xD5 => Some(LayoutKey {
            tap: Label::new("Mem Mul"),
            ..Default::default()
        }),
        0xD6 => Some(LayoutKey {
            tap: Label::new("Mem Div"),
            ..Default::default()
        }),
        0xD7 => Some(LayoutKey {
            tap: Label::new("+/-"),
            ..Default::default()
        }),
        0xD8 => Some(LayoutKey {
            tap: Label::new("Clear"),
            ..Default::default()
        }),
        0xD9 => Some(LayoutKey {
            tap: Label::new("Clear Entry"),
            ..Default::default()
        }),
        0xDA => Some(LayoutKey {
            tap: Label::new("Binary"),
            ..Default::default()
        }),
        0xDB => Some(LayoutKey {
            tap: Label::new("Octal"),
            ..Default::default()
        }),
        0xDC => Some(LayoutKey {
            tap: Label::new("Decimal"),
            ..Default::default()
        }),
        0xDD => Some(LayoutKey {
            tap: Label::new("Hex"),
            ..Default::default()
        }),
        0xE0 => Some(modifier_key(&MOD_CTRL, 0)),
        0xE1 => Some(modifier_key(&MOD_SHIFT, crate::layout_key::HELD_MOD_SHIFT)),
        0xE2 => Some(modifier_key(
            &MOD_ALT,
            crate::layout_key::PLAIN_ALT_MOD_MASK,
        )),
        0xE3 => Some(modifier_key(&MOD_GUI, 0)),
        0xE4 => Some(modifier_key(&MOD_CTRL, 0)),
        0xE5 => Some(modifier_key(&MOD_SHIFT, crate::layout_key::HELD_MOD_SHIFT)),
        // RAlt is the layout's Level-3 shift, on layouts that define one.
        0xE6 => Some(modifier_key(&MOD_ALT, crate::layout_key::HELD_MOD_RALT)),
        0xE7 => Some(modifier_key(&MOD_GUI, 0)),
        0xE8 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::PLAY_PAUSE.to_string()),
            ..Default::default()
        }),
        0xE9 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::STOP.to_string()),
            ..Default::default()
        }),
        0xEA => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_BACK.to_string()),
            ..Default::default()
        }),
        0xEB => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_FORWARD.to_string()),
            ..Default::default()
        }),
        0xEC => Some(LayoutKey {
            tap: Label::with_short("Eject", "Ejct"),
            ..Default::default()
        }),
        0xED => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
            ..Default::default()
        }),
        0xEE => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
            ..Default::default()
        }),
        0xEF => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_X.to_string()),
            ..Default::default()
        }),
        0xF0 => Some(LayoutKey {
            tap: Label::new("WWW"),
            ..Default::default()
        }),
        0xF1 => Some(LayoutKey {
            tap: Label::new("Back"),
            ..Default::default()
        }),
        0xF2 => Some(LayoutKey {
            tap: Label::new("Forward"),
            ..Default::default()
        }),
        0xF3 => Some(LayoutKey {
            tap: Label::new("Stop"),
            ..Default::default()
        }),
        0xF4 => Some(LayoutKey {
            tap: Label::new("Find"),
            ..Default::default()
        }),
        0xF5 => Some(LayoutKey {
            tap: Label::with_short("Scroll Up", "ScrUp"),
            ..Default::default()
        }),
        0xF6 => Some(LayoutKey {
            tap: Label::with_short("Scroll Down", "ScrDn"),
            ..Default::default()
        }),
        0xF7 => Some(LayoutKey {
            tap: Label::new("Edit"),
            ..Default::default()
        }),
        0xF8 => Some(LayoutKey {
            tap: Label::new("Sleep"),
            ..Default::default()
        }),
        0xF9 => Some(LayoutKey {
            tap: Label::with_short("Screensaver", "Lock"),
            ..Default::default()
        }),
        0xFA => Some(LayoutKey {
            tap: Label::new("Refresh"),
            ..Default::default()
        }),
        0xFB => Some(LayoutKey {
            tap: Label::new("Calc"),
            ..Default::default()
        }),
        _ => None,
    }
}

/// Resolves a USB HID Keyboard/Keypad Page (0x07) usage ID with active OS layout localization.
pub fn hid_keyboard_key(usage_id: u16) -> Option<LayoutKey> {
    let mut key = hid_keyboard_key_static(usage_id)?;
    super::os_layout::apply_os_overrides(&mut key, usage_id);
    Some(key)
}
