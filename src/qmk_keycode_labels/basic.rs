use crate::layout_key::{KeycodeKind, Label, LayoutKey};

use qmk_via_api::keycodes::Keycode;

pub fn get_basic_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    let keycode = Keycode::try_from(keycode_bytes).ok()?;

    match keycode {
        Keycode::KC_NO => Some(LayoutKey {
            tap: Label::new(""),
            ..Default::default()
        }),
        Keycode::KC_TRANSPARENT => Some(LayoutKey {
            tap: Label::new(""),
            ..Default::default()
        }),
        Keycode::KC_A => Some(LayoutKey {
            tap: Label::new("A"),
            ..Default::default()
        }),
        Keycode::KC_B => Some(LayoutKey {
            tap: Label::new("B"),
            ..Default::default()
        }),
        Keycode::KC_C => Some(LayoutKey {
            tap: Label::new("C"),
            ..Default::default()
        }),
        Keycode::KC_D => Some(LayoutKey {
            tap: Label::new("D"),
            ..Default::default()
        }),
        Keycode::KC_E => Some(LayoutKey {
            tap: Label::new("E"),
            ..Default::default()
        }),
        Keycode::KC_F => Some(LayoutKey {
            tap: Label::new("F"),
            ..Default::default()
        }),
        Keycode::KC_G => Some(LayoutKey {
            tap: Label::new("G"),
            ..Default::default()
        }),
        Keycode::KC_H => Some(LayoutKey {
            tap: Label::new("H"),
            ..Default::default()
        }),
        Keycode::KC_I => Some(LayoutKey {
            tap: Label::new("I"),
            ..Default::default()
        }),
        Keycode::KC_J => Some(LayoutKey {
            tap: Label::new("J"),
            ..Default::default()
        }),
        Keycode::KC_K => Some(LayoutKey {
            tap: Label::new("K"),
            ..Default::default()
        }),
        Keycode::KC_L => Some(LayoutKey {
            tap: Label::new("L"),
            ..Default::default()
        }),
        Keycode::KC_M => Some(LayoutKey {
            tap: Label::new("M"),
            ..Default::default()
        }),
        Keycode::KC_N => Some(LayoutKey {
            tap: Label::new("N"),
            ..Default::default()
        }),
        Keycode::KC_O => Some(LayoutKey {
            tap: Label::new("O"),
            ..Default::default()
        }),
        Keycode::KC_P => Some(LayoutKey {
            tap: Label::new("P"),
            ..Default::default()
        }),
        Keycode::KC_Q => Some(LayoutKey {
            tap: Label::new("Q"),
            ..Default::default()
        }),
        Keycode::KC_R => Some(LayoutKey {
            tap: Label::new("R"),
            ..Default::default()
        }),
        Keycode::KC_S => Some(LayoutKey {
            tap: Label::new("S"),
            ..Default::default()
        }),
        Keycode::KC_T => Some(LayoutKey {
            tap: Label::new("T"),
            ..Default::default()
        }),
        Keycode::KC_U => Some(LayoutKey {
            tap: Label::new("U"),
            ..Default::default()
        }),
        Keycode::KC_V => Some(LayoutKey {
            tap: Label::new("V"),
            ..Default::default()
        }),
        Keycode::KC_W => Some(LayoutKey {
            tap: Label::new("W"),
            ..Default::default()
        }),
        Keycode::KC_X => Some(LayoutKey {
            tap: Label::new("X"),
            ..Default::default()
        }),
        Keycode::KC_Y => Some(LayoutKey {
            tap: Label::new("Y"),
            ..Default::default()
        }),
        Keycode::KC_Z => Some(LayoutKey {
            tap: Label::new("Z"),
            ..Default::default()
        }),
        Keycode::KC_1 => Some(LayoutKey {
            tap: Label::new("!\n1"),
            ..Default::default()
        }),
        Keycode::KC_2 => Some(LayoutKey {
            tap: Label::new("@\n2"),
            ..Default::default()
        }),
        Keycode::KC_3 => Some(LayoutKey {
            tap: Label::new("#\n3"),
            ..Default::default()
        }),
        Keycode::KC_4 => Some(LayoutKey {
            tap: Label::new("$\n4"),
            ..Default::default()
        }),
        Keycode::KC_5 => Some(LayoutKey {
            tap: Label::new("%\n5"),
            ..Default::default()
        }),
        Keycode::KC_6 => Some(LayoutKey {
            tap: Label::new("^\n6"),
            ..Default::default()
        }),
        Keycode::KC_7 => Some(LayoutKey {
            tap: Label::new("&\n7"),
            ..Default::default()
        }),
        Keycode::KC_8 => Some(LayoutKey {
            tap: Label::new("*\n8"),
            ..Default::default()
        }),
        Keycode::KC_9 => Some(LayoutKey {
            tap: Label::new("(\n9"),
            ..Default::default()
        }),
        Keycode::KC_0 => Some(LayoutKey {
            tap: Label::new(")\n0"),
            ..Default::default()
        }),
        Keycode::KC_ENTER => Some(LayoutKey {
            tap: Label::new("Enter"),
            symbol: Some(egui_phosphor::regular::ARROW_ELBOW_DOWN_LEFT.to_string()),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Keycode::KC_ESCAPE => Some(LayoutKey {
            tap: Label::new("Esc"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),
        Keycode::KC_BACKSPACE => Some(LayoutKey {
            tap: Label::with_short("Backspace", "Bspc"),
            symbol: Some(egui_phosphor::regular::BACKSPACE.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_TAB => Some(LayoutKey {
            tap: Label::new("Tab"),
            symbol: Some(egui_phosphor::regular::ARROWS_LEFT_RIGHT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_SPACE => Some(LayoutKey {
            tap: Label::with_short("Space", "Spc"),
            ..Default::default()
        }),
        Keycode::KC_MINUS => Some(LayoutKey {
            tap: Label::new("_\n-"),
            ..Default::default()
        }),
        Keycode::KC_EQUAL => Some(LayoutKey {
            tap: Label::new("+\n="),
            ..Default::default()
        }),
        Keycode::KC_LEFT_BRACKET => Some(LayoutKey {
            tap: Label::new("{\n["),
            ..Default::default()
        }),
        Keycode::KC_RIGHT_BRACKET => Some(LayoutKey {
            tap: Label::new("}\n]"),
            ..Default::default()
        }),
        Keycode::KC_BACKSLASH => Some(LayoutKey {
            tap: Label::new("|\n\\"),
            ..Default::default()
        }),
        Keycode::KC_NONUS_HASH => Some(LayoutKey {
            tap: Label::new("NUHS"),
            ..Default::default()
        }),
        Keycode::KC_SEMICOLON => Some(LayoutKey {
            tap: Label::new(":\n;"),
            ..Default::default()
        }),
        Keycode::KC_QUOTE => Some(LayoutKey {
            tap: Label::new("\"\n\'"),
            ..Default::default()
        }),
        Keycode::KC_GRAVE => Some(LayoutKey {
            tap: Label::new("~\n`"),
            ..Default::default()
        }),
        Keycode::KC_COMMA => Some(LayoutKey {
            tap: Label::new("<\n,"),
            ..Default::default()
        }),
        Keycode::KC_DOT => Some(LayoutKey {
            tap: Label::new(">\n."),
            ..Default::default()
        }),
        Keycode::KC_SLASH => Some(LayoutKey {
            tap: Label::new("?\n/"),
            ..Default::default()
        }),
        Keycode::KC_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Capslock", "Caps"),
            symbol: Some(egui_phosphor::regular::ARROW_FAT_LINE_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_F1 => Some(LayoutKey {
            tap: Label::new("F1"),
            ..Default::default()
        }),
        Keycode::KC_F2 => Some(LayoutKey {
            tap: Label::new("F2"),
            ..Default::default()
        }),
        Keycode::KC_F3 => Some(LayoutKey {
            tap: Label::new("F3"),
            ..Default::default()
        }),
        Keycode::KC_F4 => Some(LayoutKey {
            tap: Label::new("F4"),
            ..Default::default()
        }),
        Keycode::KC_F5 => Some(LayoutKey {
            tap: Label::new("F5"),
            ..Default::default()
        }),
        Keycode::KC_F6 => Some(LayoutKey {
            tap: Label::new("F6"),
            ..Default::default()
        }),
        Keycode::KC_F7 => Some(LayoutKey {
            tap: Label::new("F7"),
            ..Default::default()
        }),
        Keycode::KC_F8 => Some(LayoutKey {
            tap: Label::new("F8"),
            ..Default::default()
        }),
        Keycode::KC_F9 => Some(LayoutKey {
            tap: Label::new("F9"),
            ..Default::default()
        }),
        Keycode::KC_F10 => Some(LayoutKey {
            tap: Label::new("F10"),
            ..Default::default()
        }),
        Keycode::KC_F11 => Some(LayoutKey {
            tap: Label::new("F11"),
            ..Default::default()
        }),
        Keycode::KC_F12 => Some(LayoutKey {
            tap: Label::new("F12"),
            ..Default::default()
        }),
        Keycode::KC_PRINT_SCREEN => Some(LayoutKey {
            tap: Label::with_short("Print Screen", "PrtSc"),
            ..Default::default()
        }),
        Keycode::KC_SCROLL_LOCK => Some(LayoutKey {
            tap: Label::with_short("Scroll Lock", "ScrLk"),
            ..Default::default()
        }),
        Keycode::KC_PAUSE => Some(LayoutKey {
            tap: Label::with_short("Pause", "Paus"),
            ..Default::default()
        }),
        Keycode::KC_INSERT => Some(LayoutKey {
            tap: Label::with_short("Insert", "Ins"),
            ..Default::default()
        }),
        Keycode::KC_HOME => Some(LayoutKey {
            tap: Label::new("Home"),
            ..Default::default()
        }),
        Keycode::KC_PAGE_UP => Some(LayoutKey {
            tap: Label::with_short("Page Up", "PgUp"),
            ..Default::default()
        }),
        Keycode::KC_DELETE => Some(LayoutKey {
            tap: Label::with_short("Delete", "Del"),
            ..Default::default()
        }),
        Keycode::KC_END => Some(LayoutKey {
            tap: Label::new("End"),
            ..Default::default()
        }),
        Keycode::KC_PAGE_DOWN => Some(LayoutKey {
            tap: Label::with_short("Page Down", "PgDn"),
            ..Default::default()
        }),
        Keycode::KC_RIGHT => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_RIGHT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_LEFT => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_LEFT.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_DOWN => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_DOWN.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_UP => Some(LayoutKey {
            tap: Label::default(),
            symbol: Some(egui_phosphor::regular::ARROW_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_NUM_LOCK => Some(LayoutKey {
            tap: Label::with_short("Num\nLock", "NumLk"),
            ..Default::default()
        }),
        Keycode::KC_KP_SLASH => Some(LayoutKey {
            tap: Label::new("÷"),
            ..Default::default()
        }),
        Keycode::KC_KP_ASTERISK => Some(LayoutKey {
            tap: Label::new("×"),
            ..Default::default()
        }),
        Keycode::KC_KP_MINUS => Some(LayoutKey {
            tap: Label::new("-"),
            ..Default::default()
        }),
        Keycode::KC_KP_PLUS => Some(LayoutKey {
            tap: Label::new("+"),
            ..Default::default()
        }),
        Keycode::KC_KP_ENTER => Some(LayoutKey {
            tap: Label::new("Enter"),
            symbol: Some(egui_phosphor::regular::ARROW_ELBOW_DOWN_LEFT.to_string()),
            ..Default::default()
        }),
        Keycode::KC_KP_1 => Some(LayoutKey {
            tap: Label::new("1"),
            ..Default::default()
        }),
        Keycode::KC_KP_2 => Some(LayoutKey {
            tap: Label::new("2"),
            ..Default::default()
        }),
        Keycode::KC_KP_3 => Some(LayoutKey {
            tap: Label::new("3"),
            ..Default::default()
        }),
        Keycode::KC_KP_4 => Some(LayoutKey {
            tap: Label::new("4"),
            ..Default::default()
        }),
        Keycode::KC_KP_5 => Some(LayoutKey {
            tap: Label::new("5"),
            ..Default::default()
        }),
        Keycode::KC_KP_6 => Some(LayoutKey {
            tap: Label::new("6"),
            ..Default::default()
        }),
        Keycode::KC_KP_7 => Some(LayoutKey {
            tap: Label::new("7"),
            ..Default::default()
        }),
        Keycode::KC_KP_8 => Some(LayoutKey {
            tap: Label::new("8"),
            ..Default::default()
        }),
        Keycode::KC_KP_9 => Some(LayoutKey {
            tap: Label::new("9"),
            ..Default::default()
        }),
        Keycode::KC_KP_0 => Some(LayoutKey {
            tap: Label::new("0"),
            ..Default::default()
        }),
        Keycode::KC_KP_DOT => Some(LayoutKey {
            tap: Label::new("."),
            ..Default::default()
        }),
        Keycode::KC_NONUS_BACKSLASH => Some(LayoutKey {
            tap: Label::new("NUBS"),
            ..Default::default()
        }),
        Keycode::KC_APPLICATION => Some(LayoutKey {
            tap: Label::new("Menu"),
            symbol: Some(egui_phosphor::regular::LIST.to_string()),
            ..Default::default()
        }),
        Keycode::KC_KB_POWER => Some(LayoutKey {
            tap: Label::new("Power"),
            symbol: Some(egui_phosphor::regular::POWER.to_string()),
            ..Default::default()
        }),
        Keycode::KC_KP_EQUAL => Some(LayoutKey {
            tap: Label::new("="),
            ..Default::default()
        }),
        Keycode::KC_F13 => Some(LayoutKey {
            tap: Label::new("F13"),
            ..Default::default()
        }),
        Keycode::KC_F14 => Some(LayoutKey {
            tap: Label::new("F14"),
            ..Default::default()
        }),
        Keycode::KC_F15 => Some(LayoutKey {
            tap: Label::new("F15"),
            ..Default::default()
        }),
        Keycode::KC_F16 => Some(LayoutKey {
            tap: Label::new("F16"),
            ..Default::default()
        }),
        Keycode::KC_F17 => Some(LayoutKey {
            tap: Label::new("F17"),
            ..Default::default()
        }),
        Keycode::KC_F18 => Some(LayoutKey {
            tap: Label::new("F18"),
            ..Default::default()
        }),
        Keycode::KC_F19 => Some(LayoutKey {
            tap: Label::new("F19"),
            ..Default::default()
        }),
        Keycode::KC_F20 => Some(LayoutKey {
            tap: Label::new("F20"),
            ..Default::default()
        }),
        Keycode::KC_F21 => Some(LayoutKey {
            tap: Label::new("F21"),
            ..Default::default()
        }),
        Keycode::KC_F22 => Some(LayoutKey {
            tap: Label::new("F22"),
            ..Default::default()
        }),
        Keycode::KC_F23 => Some(LayoutKey {
            tap: Label::new("F23"),
            ..Default::default()
        }),
        Keycode::KC_F24 => Some(LayoutKey {
            tap: Label::new("F24"),
            ..Default::default()
        }),
        Keycode::KC_EXECUTE => Some(LayoutKey {
            tap: Label::new("Exec"),
            ..Default::default()
        }),
        Keycode::KC_HELP => Some(LayoutKey {
            tap: Label::new("Help"),
            ..Default::default()
        }),
        Keycode::KC_MENU => Some(LayoutKey {
            tap: Label::new("Menu"),
            ..Default::default()
        }),
        Keycode::KC_SELECT => Some(LayoutKey {
            tap: Label::new("Select"),
            ..Default::default()
        }),
        Keycode::KC_STOP => Some(LayoutKey {
            tap: Label::new("Stop"),
            ..Default::default()
        }),
        Keycode::KC_AGAIN => Some(LayoutKey {
            tap: Label::new("Again"),
            ..Default::default()
        }),
        Keycode::KC_UNDO => Some(LayoutKey {
            tap: Label::new("Undo"),
            ..Default::default()
        }),
        Keycode::KC_CUT => Some(LayoutKey {
            tap: Label::new("Cut"),
            ..Default::default()
        }),
        Keycode::KC_COPY => Some(LayoutKey {
            tap: Label::new("Copy"),
            ..Default::default()
        }),
        Keycode::KC_PASTE => Some(LayoutKey {
            tap: Label::new("Paste"),
            ..Default::default()
        }),
        Keycode::KC_FIND => Some(LayoutKey {
            tap: Label::new("Find"),
            ..Default::default()
        }),
        Keycode::KC_KB_MUTE => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_X.to_string()),
            ..Default::default()
        }),
        Keycode::KC_KB_VOLUME_UP => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
            ..Default::default()
        }),
        Keycode::KC_KB_VOLUME_DOWN => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
            ..Default::default()
        }),
        Keycode::KC_LOCKING_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Locking Caps Lock", "LCaps"),
            ..Default::default()
        }),
        Keycode::KC_LOCKING_NUM_LOCK => Some(LayoutKey {
            tap: Label::with_short("Locking Num Lock", "LNum"),
            ..Default::default()
        }),
        Keycode::KC_LOCKING_SCROLL_LOCK => Some(LayoutKey {
            tap: Label::with_short("Locking Scroll Lock", "LScrl"),
            ..Default::default()
        }),
        Keycode::KC_KP_COMMA => Some(LayoutKey {
            tap: Label::new(","),
            ..Default::default()
        }),
        Keycode::KC_KP_EQUAL_AS400 => Some(LayoutKey {
            tap: Label::new("="),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_1 => Some(LayoutKey {
            tap: Label::new("Int1"),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_2 => Some(LayoutKey {
            tap: Label::new("Int2"),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_3 => Some(LayoutKey {
            tap: Label::new("Int3"),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_4 => Some(LayoutKey {
            tap: Label::new("Int4"),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_5 => Some(LayoutKey {
            tap: Label::new("Int5"),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_6 => Some(LayoutKey {
            tap: Label::new("Int6"),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_7 => Some(LayoutKey {
            tap: Label::new("Int7"),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_8 => Some(LayoutKey {
            tap: Label::new("Int8"),
            ..Default::default()
        }),
        Keycode::KC_INTERNATIONAL_9 => Some(LayoutKey {
            tap: Label::new("Int9"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_1 => Some(LayoutKey {
            tap: Label::new("Lang1"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_2 => Some(LayoutKey {
            tap: Label::new("Lang2"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_3 => Some(LayoutKey {
            tap: Label::new("Lang3"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_4 => Some(LayoutKey {
            tap: Label::new("Lang4"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_5 => Some(LayoutKey {
            tap: Label::new("Lang5"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_6 => Some(LayoutKey {
            tap: Label::new("Lang6"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_7 => Some(LayoutKey {
            tap: Label::new("Lang7"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_8 => Some(LayoutKey {
            tap: Label::new("Lang8"),
            ..Default::default()
        }),
        Keycode::KC_LANGUAGE_9 => Some(LayoutKey {
            tap: Label::new("Lang9"),
            ..Default::default()
        }),
        Keycode::KC_ALTERNATE_ERASE => Some(LayoutKey {
            tap: Label::new("Alt Erase"),
            ..Default::default()
        }),
        Keycode::KC_SYSTEM_REQUEST => Some(LayoutKey {
            tap: Label::new("SysReq"),
            ..Default::default()
        }),
        Keycode::KC_CANCEL => Some(LayoutKey {
            tap: Label::new("Cancel"),
            ..Default::default()
        }),
        Keycode::KC_CLEAR => Some(LayoutKey {
            tap: Label::new("Clear"),
            ..Default::default()
        }),
        Keycode::KC_PRIOR => Some(LayoutKey {
            tap: Label::new("Prior"),
            ..Default::default()
        }),
        Keycode::KC_RETURN => Some(LayoutKey {
            tap: Label::new("Return"),
            ..Default::default()
        }),
        Keycode::KC_SEPARATOR => Some(LayoutKey {
            tap: Label::new("Separator"),
            ..Default::default()
        }),
        Keycode::KC_OUT => Some(LayoutKey {
            tap: Label::new("Out"),
            ..Default::default()
        }),
        Keycode::KC_OPER => Some(LayoutKey {
            tap: Label::new("Oper"),
            ..Default::default()
        }),
        Keycode::KC_CLEAR_AGAIN => Some(LayoutKey {
            tap: Label::new("Clear Again"),
            ..Default::default()
        }),
        Keycode::KC_CRSEL => Some(LayoutKey {
            tap: Label::new("CrSel"),
            ..Default::default()
        }),
        Keycode::KC_EXSEL => Some(LayoutKey {
            tap: Label::new("ExSel"),
            ..Default::default()
        }),
        Keycode::KC_SYSTEM_POWER => Some(LayoutKey {
            tap: Label::new("Power"),
            ..Default::default()
        }),
        Keycode::KC_SYSTEM_SLEEP => Some(LayoutKey {
            tap: Label::new("Sleep"),
            ..Default::default()
        }),
        Keycode::KC_SYSTEM_WAKE => Some(LayoutKey {
            tap: Label::new("Wake"),
            ..Default::default()
        }),
        Keycode::KC_AUDIO_MUTE => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_X.to_string()),
            ..Default::default()
        }),
        Keycode::KC_AUDIO_VOL_UP => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
            ..Default::default()
        }),
        Keycode::KC_AUDIO_VOL_DOWN => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_NEXT_TRACK => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_FORWARD.to_string()),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_PREV_TRACK => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_BACK.to_string()),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_STOP => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::STOP.to_string()),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_PLAY_PAUSE => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::PLAY_PAUSE.to_string()),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_SELECT => Some(LayoutKey {
            tap: Label::with_short("Select", "Sel"),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_EJECT => Some(LayoutKey {
            tap: Label::with_short("Eject", "Ejct"),
            ..Default::default()
        }),
        Keycode::KC_MAIL => Some(LayoutKey {
            tap: Label::new("Mail"),
            ..Default::default()
        }),
        Keycode::KC_CALCULATOR => Some(LayoutKey {
            tap: Label::new("Calc"),
            ..Default::default()
        }),
        Keycode::KC_MY_COMPUTER => Some(LayoutKey {
            tap: Label::new("My Comp"),
            ..Default::default()
        }),
        Keycode::KC_WWW_SEARCH => Some(LayoutKey {
            tap: Label::new("Search"),
            ..Default::default()
        }),
        Keycode::KC_WWW_HOME => Some(LayoutKey {
            tap: Label::new("Home"),
            ..Default::default()
        }),
        Keycode::KC_WWW_BACK => Some(LayoutKey {
            tap: Label::new("Back"),
            ..Default::default()
        }),
        Keycode::KC_WWW_FORWARD => Some(LayoutKey {
            tap: Label::new("Forward"),
            ..Default::default()
        }),
        Keycode::KC_WWW_STOP => Some(LayoutKey {
            tap: Label::new("Stop"),
            ..Default::default()
        }),
        Keycode::KC_WWW_REFRESH => Some(LayoutKey {
            tap: Label::new("Refresh"),
            ..Default::default()
        }),
        Keycode::KC_WWW_FAVORITES => Some(LayoutKey {
            tap: Label::new("Favorites"),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_FAST_FORWARD => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::FAST_FORWARD.to_string()),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_REWIND => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::REWIND.to_string()),
            ..Default::default()
        }),
        Keycode::KC_BRIGHTNESS_UP => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SUN.to_string()),
            ..Default::default()
        }),
        Keycode::KC_BRIGHTNESS_DOWN => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SUN_DIM.to_string()),
            ..Default::default()
        }),
        Keycode::KC_CONTROL_PANEL => Some(LayoutKey {
            tap: Label::with_short("Control Panel", "Ctrl P"),
            ..Default::default()
        }),
        Keycode::KC_ASSISTANT => Some(LayoutKey {
            tap: Label::with_short("Assistant", "Asst"),
            ..Default::default()
        }),
        Keycode::KC_MISSION_CONTROL => Some(LayoutKey {
            tap: Label::with_short("Mission Control", "MC"),
            ..Default::default()
        }),
        Keycode::KC_LAUNCHPAD => Some(LayoutKey {
            tap: Label::with_short("Launchpad", "LP"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_CURSOR_UP => Some(LayoutKey {
            tap: Label::new("Mouse ↑"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_CURSOR_DOWN => Some(LayoutKey {
            tap: Label::new("Mouse ↓"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_CURSOR_LEFT => Some(LayoutKey {
            tap: Label::new("Mouse ←"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_CURSOR_RIGHT => Some(LayoutKey {
            tap: Label::new("Mouse →"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_1 => Some(LayoutKey {
            tap: Label::new("Mouse Btn1"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_2 => Some(LayoutKey {
            tap: Label::new("Mouse Btn2"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_3 => Some(LayoutKey {
            tap: Label::new("Mouse Btn3"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_4 => Some(LayoutKey {
            tap: Label::new("Mouse Btn4"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_5 => Some(LayoutKey {
            tap: Label::new("Mouse Btn5"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_6 => Some(LayoutKey {
            tap: Label::new("Mouse Btn6"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_7 => Some(LayoutKey {
            tap: Label::new("Mouse Btn7"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_8 => Some(LayoutKey {
            tap: Label::new("Mouse Btn8"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_WHEEL_UP => Some(LayoutKey {
            tap: Label::new("Mouse Wh ↑"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_WHEEL_DOWN => Some(LayoutKey {
            tap: Label::new("Mouse Wh ↓"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_WHEEL_LEFT => Some(LayoutKey {
            tap: Label::new("Mouse Wh ←"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_WHEEL_RIGHT => Some(LayoutKey {
            tap: Label::new("Mouse Wh →"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_ACCELERATION_0 => Some(LayoutKey {
            tap: Label::new("Mouse Acc0"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_ACCELERATION_1 => Some(LayoutKey {
            tap: Label::new("Mouse Acc1"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_ACCELERATION_2 => Some(LayoutKey {
            tap: Label::new("Mouse Acc2"),
            ..Default::default()
        }),
        Keycode::KC_LEFT_CTRL => Some(LayoutKey {
            tap: Label::new("Ctrl"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_LEFT_SHIFT => Some(LayoutKey {
            tap: Label::new("Shift"),
            symbol: Some(egui_phosphor::regular::ARROW_FAT_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_LEFT_ALT => Some(LayoutKey {
            tap: Label::new("Alt"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_LEFT_GUI => Some(LayoutKey {
            tap: Label::new("Win"),
            symbol: Some(egui_phosphor::regular::WINDOWS_LOGO.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_RIGHT_CTRL => Some(LayoutKey {
            tap: Label::new("Ctrl"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_RIGHT_SHIFT => Some(LayoutKey {
            tap: Label::new("Shift"),
            symbol: Some(egui_phosphor::regular::ARROW_FAT_UP.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_RIGHT_ALT => Some(LayoutKey {
            tap: Label::new("Alt"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::KC_RIGHT_GUI => Some(LayoutKey {
            tap: Label::new("Win"),
            symbol: Some(egui_phosphor::regular::WINDOWS_LOGO.to_string()),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Swap Hands Toggle", "SwpHT"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_TAP_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Swap Hands Tap Toggle", "SwpTT"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_MOMENTARY_ON => Some(LayoutKey {
            tap: Label::with_short("Swap Hands On", "SwpOn"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_MOMENTARY_OFF => Some(LayoutKey {
            tap: Label::with_short("Swap Hands Off", "SwpOff"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_OFF => Some(LayoutKey {
            tap: Label::with_short("Swap Hands Off", "SwpOff"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_ON => Some(LayoutKey {
            tap: Label::with_short("Swap Hands On", "SwpOn"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_ONE_SHOT => Some(LayoutKey {
            tap: Label::with_short("Swap Hands One Shot", "SwpOS"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_CONTROL_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Swap Ctrl Caps", "SwCtCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_CONTROL_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Unswap Ctrl Caps", "UnCtCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_CONTROL_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Toggle Ctrl Caps", "TgCtCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_CAPS_LOCK_AS_CONTROL_OFF => Some(LayoutKey {
            tap: Label::with_short("Caps as Ctrl Off", "CpCtOf"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_CAPS_LOCK_AS_CONTROL_ON => Some(LayoutKey {
            tap: Label::with_short("Caps as Ctrl On", "CpCtOn"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_LALT_LGUI => Some(LayoutKey {
            tap: Label::with_short("Swap LAlt LGui", "SwAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_LALT_LGUI => Some(LayoutKey {
            tap: Label::with_short("Unswap LAlt LGui", "UnAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_RALT_RGUI => Some(LayoutKey {
            tap: Label::with_short("Swap RAlt RGui", "SwAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_RALT_RGUI => Some(LayoutKey {
            tap: Label::with_short("Unswap RAlt RGui", "UnAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_GUI_ON => Some(LayoutKey {
            tap: Label::with_short("GUI On", "GuiOn"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_GUI_OFF => Some(LayoutKey {
            tap: Label::with_short("GUI Off", "GuiOff"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_GUI => Some(LayoutKey {
            tap: Label::with_short("Toggle GUI", "TgGui"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_GRAVE_ESC => Some(LayoutKey {
            tap: Label::with_short("Swap ` Esc", "Sw`Esc"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_GRAVE_ESC => Some(LayoutKey {
            tap: Label::with_short("Unswap ` Esc", "Un`Esc"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_BACKSLASH_BACKSPACE => Some(LayoutKey {
            tap: Label::with_short("Swap \\ Bksp", "Sw\\Bk"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_BACKSLASH_BACKSPACE => Some(LayoutKey {
            tap: Label::with_short("Unswap \\ Bksp", "Un\\Bk"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_BACKSLASH_BACKSPACE => Some(LayoutKey {
            tap: Label::with_short("Toggle \\ Bksp", "Tg\\Bk"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_NKRO_ON => Some(LayoutKey {
            tap: Label::with_short("NKRO On", "NKROOn"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_NKRO_OFF => Some(LayoutKey {
            tap: Label::with_short("NKRO Off", "NKROOf"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_NKRO => Some(LayoutKey {
            tap: Label::with_short("Toggle NKRO", "NKRO"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_ALT_GUI => Some(LayoutKey {
            tap: Label::with_short("Swap Alt GUI", "SwAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_ALT_GUI => Some(LayoutKey {
            tap: Label::with_short("Unswap Alt GUI", "UnAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_ALT_GUI => Some(LayoutKey {
            tap: Label::with_short("Toggle Alt GUI", "TgAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_LCTL_LGUI => Some(LayoutKey {
            tap: Label::with_short("Swap LCtl LGui", "SwCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_LCTL_LGUI => Some(LayoutKey {
            tap: Label::with_short("Unswap LCtl LGui", "UnCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_RCTL_RGUI => Some(LayoutKey {
            tap: Label::with_short("Swap RCtl RGui", "SwCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_RCTL_RGUI => Some(LayoutKey {
            tap: Label::with_short("Unswap RCtl RGui", "UnCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_CTL_GUI => Some(LayoutKey {
            tap: Label::with_short("Swap Ctl GUI", "SwCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_CTL_GUI => Some(LayoutKey {
            tap: Label::with_short("Unswap Ctl GUI", "UnCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_CTL_GUI => Some(LayoutKey {
            tap: Label::with_short("Toggle Ctl GUI", "TgCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_EE_HANDS_LEFT => Some(LayoutKey {
            tap: Label::with_short("EE Hands Left", "EEHndL"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_EE_HANDS_RIGHT => Some(LayoutKey {
            tap: Label::with_short("EE Hands Right", "EEHndR"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_ESCAPE_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Swap Esc Caps", "SwEsCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_ESCAPE_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Unswap Esc Caps", "UnEsCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_ESCAPE_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Toggle Esc Caps", "TgEsCp"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_ON => Some(LayoutKey {
            tap: Label::with_short("MIDI On", "MDOn"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OFF => Some(LayoutKey {
            tap: Label::with_short("MIDI Off", "MDOff"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("MIDI Toggle", "MDTg"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI C0", "MDC0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_SHARP_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI C#0", "MDC#0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI D0", "MDD0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_SHARP_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI D#0", "MDD#0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_E_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI E0", "MDE0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI F0", "MDF0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_SHARP_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI F#0", "MDF#0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI G0", "MDG0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_SHARP_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI G#0", "MDG#0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI A0", "MDA0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_SHARP_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI A#0", "MDA#0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_B_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI B0", "MDB0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI C1", "MDC1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_SHARP_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI C#1", "MDC#1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI D1", "MDD1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_SHARP_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI D#1", "MDD#1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_E_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI E1", "MDE1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI F1", "MDF1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_SHARP_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI F#1", "MDF#1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI G1", "MDG1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_SHARP_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI G#1", "MDG#1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI A1", "MDA1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_SHARP_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI A#1", "MDA#1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_B_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI B1", "MDB1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI C2", "MDC2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_SHARP_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI C#2", "MDC#2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI D2", "MDD2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_SHARP_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI D#2", "MDD#2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_E_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI E2", "MDE2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI F2", "MDF2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_SHARP_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI F#2", "MDF#2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI G2", "MDG2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_SHARP_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI G#2", "MDG#2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI A2", "MDA2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_SHARP_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI A#2", "MDA#2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_B_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI B2", "MDB2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI C3", "MDC3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_SHARP_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI C#3", "MDC#3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI D3", "MDD3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_SHARP_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI D#3", "MDD#3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_E_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI E3", "MDE3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI F3", "MDF3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_SHARP_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI F#3", "MDF#3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI G3", "MDG3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_SHARP_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI G#3", "MDG#3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI A3", "MDA3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_SHARP_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI A#3", "MDA#3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_B_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI B3", "MDB3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI C4", "MDC4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_SHARP_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI C#4", "MDC#4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI D4", "MDD4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_SHARP_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI D#4", "MDD#4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_E_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI E4", "MDE4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI F4", "MDF4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_SHARP_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI F#4", "MDF#4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI G4", "MDG4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_SHARP_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI G#4", "MDG#4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI A4", "MDA4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_SHARP_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI A#4", "MDA#4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_B_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI B4", "MDB4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI C5", "MDC5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_C_SHARP_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI C#5", "MDC#5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI D5", "MDD5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_D_SHARP_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI D#5", "MDD#5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_E_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI E5", "MDE5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI F5", "MDF5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_F_SHARP_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI F#5", "MDF#5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI G5", "MDG5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_G_SHARP_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI G#5", "MDG#5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI A5", "MDA5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_A_SHARP_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI A#5", "MDA#5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_NOTE_B_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI B5", "MDB5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_N2 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct -2", "MDO-2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_N1 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct -1", "MDO-1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct 0", "MDO0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct 1", "MDO1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct 2", "MDO2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct 3", "MDO3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct 4", "MDO4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct 5", "MDO5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_6 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct 6", "MDO6"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_7 => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct 7", "MDO7"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct Down", "MDO-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct Up", "MDO+"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_N6 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans -6", "MDT-6"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_N5 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans -5", "MDT-5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_N4 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans -4", "MDT-4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_N3 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans -3", "MDT-3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_N2 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans -2", "MDT-2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_N1 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans -1", "MDT-1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans 0", "MDT0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans 1", "MDT1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans 2", "MDT2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans 3", "MDT3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans 4", "MDT4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans 5", "MDT5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_6 => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans 6", "MDT6"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans Down", "MDT-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans Up", "MDT+"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_0 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 0", "MDV0"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 1", "MDV1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 2", "MDV2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 3", "MDV3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 4", "MDV4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 5", "MDV5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_6 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 6", "MDV6"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_7 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 7", "MDV7"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_8 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 8", "MDV8"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_9 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 9", "MDV9"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_10 => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel 10", "MDV10"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel Down", "MDV-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel Up", "MDV+"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_1 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 1", "MDC1"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_2 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 2", "MDC2"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_3 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 3", "MDC3"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_4 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 4", "MDC4"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_5 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 5", "MDC5"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_6 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 6", "MDC6"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_7 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 7", "MDC7"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_8 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 8", "MDC8"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_9 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 9", "MDC9"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_10 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 10", "MDC10"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_11 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 11", "MDC11"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_12 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 12", "MDC12"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_13 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 13", "MDC13"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_14 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 14", "MDC14"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_15 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 15", "MDC15"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_16 => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch 16", "MDC16"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch Down", "MDC-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch Up", "MDC+"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_ALL_NOTES_OFF => Some(LayoutKey {
            tap: Label::with_short("MIDI All Off", "MDAOff"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_SUSTAIN => Some(LayoutKey {
            tap: Label::with_short("MIDI Sustain", "MDSus"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_PORTAMENTO => Some(LayoutKey {
            tap: Label::with_short("MIDI Portamento", "MDPort"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_SOSTENUTO => Some(LayoutKey {
            tap: Label::with_short("MIDI Sostenuto", "MDSost"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_SOFT => Some(LayoutKey {
            tap: Label::with_short("MIDI Soft", "MDSoft"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_LEGATO => Some(LayoutKey {
            tap: Label::with_short("MIDI Legato", "MDLeg"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_MODULATION => Some(LayoutKey {
            tap: Label::with_short("MIDI Modulation", "MDMod"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_MODULATION_SPEED_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Mod Speed -", "MDM-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_MODULATION_SPEED_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Mod Speed +", "MDM+"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_PITCH_BEND_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Pitch -", "MDP-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_PITCH_BEND_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Pitch +", "MDP+"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_ON => Some(LayoutKey {
            tap: Label::with_short("Sequencer On", "SeqOn"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_OFF => Some(LayoutKey {
            tap: Label::with_short("Sequencer Off", "SeqOff"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Sequencer Toggle", "SeqTg"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_TEMPO_DOWN => Some(LayoutKey {
            tap: Label::with_short("Seq Tempo -", "SeqT-"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_TEMPO_UP => Some(LayoutKey {
            tap: Label::with_short("Seq Tempo +", "SeqT+"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_RESOLUTION_DOWN => Some(LayoutKey {
            tap: Label::with_short("Seq Res -", "SeqR-"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_RESOLUTION_UP => Some(LayoutKey {
            tap: Label::with_short("Seq Res +", "SeqR+"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_STEPS_ALL => Some(LayoutKey {
            tap: Label::with_short("Seq All Steps", "SeqAll"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_STEPS_CLEAR => Some(LayoutKey {
            tap: Label::with_short("Seq Clear Steps", "SeqClr"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_0 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 0", "JoyB0"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_1 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 1", "JoyB1"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_2 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 2", "JoyB2"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_3 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 3", "JoyB3"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_4 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 4", "JoyB4"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_5 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 5", "JoyB5"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_6 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 6", "JoyB6"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_7 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 7", "JoyB7"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_8 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 8", "JoyB8"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_9 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 9", "JoyB9"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_10 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 10", "JoyB10"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_11 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 11", "JoyB11"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_12 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 12", "JoyB12"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_13 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 13", "JoyB13"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_14 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 14", "JoyB14"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_15 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 15", "JoyB15"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_16 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 16", "JoyB16"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_17 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 17", "JoyB17"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_18 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 18", "JoyB18"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_19 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 19", "JoyB19"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_20 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 20", "JoyB20"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_21 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 21", "JoyB21"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_22 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 22", "JoyB22"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_23 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 23", "JoyB23"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_24 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 24", "JoyB24"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_25 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 25", "JoyB25"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_26 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 26", "JoyB26"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_27 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 27", "JoyB27"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_28 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 28", "JoyB28"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_29 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 29", "JoyB29"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_30 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 30", "JoyB30"),
            ..Default::default()
        }),
        Keycode::QK_JOYSTICK_BUTTON_31 => Some(LayoutKey {
            tap: Label::with_short("Joy Btn 31", "JoyB31"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_1 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 1", "PB1"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_2 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 2", "PB2"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_3 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 3", "PB3"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_4 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 4", "PB4"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_5 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 5", "PB5"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_6 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 6", "PB6"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_7 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 7", "PB7"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_8 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 8", "PB8"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_9 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 9", "PB9"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_10 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 10", "PB10"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_11 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 11", "PB11"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_12 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 12", "PB12"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_13 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 13", "PB13"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_14 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 14", "PB14"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_15 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 15", "PB15"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_16 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 16", "PB16"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_17 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 17", "PB17"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_18 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 18", "PB18"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_19 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 19", "PB19"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_20 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 20", "PB20"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_21 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 21", "PB21"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_22 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 22", "PB22"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_23 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 23", "PB23"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_24 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 24", "PB24"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_25 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 25", "PB25"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_26 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 26", "PB26"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_27 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 27", "PB27"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_28 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 28", "PB28"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_29 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 29", "PB29"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_30 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 30", "PB30"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_31 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 31", "PB31"),
            ..Default::default()
        }),
        Keycode::QK_PROGRAMMABLE_BUTTON_32 => Some(LayoutKey {
            tap: Label::with_short("Prog Btn 32", "PB32"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_ON => Some(LayoutKey {
            tap: Label::with_short("Audio On", "AudOn"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_OFF => Some(LayoutKey {
            tap: Label::with_short("Audio Off", "AudOff"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Audio Toggle", "AudTg"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Clicky Toggle", "ClkTg"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_ON => Some(LayoutKey {
            tap: Label::with_short("Clicky Enable", "ClkOn"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_OFF => Some(LayoutKey {
            tap: Label::with_short("Clicky Disable", "ClkOff"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_UP => Some(LayoutKey {
            tap: Label::with_short("Clicky Up", "Clk+"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_DOWN => Some(LayoutKey {
            tap: Label::with_short("Clicky Down", "Clk-"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_RESET => Some(LayoutKey {
            tap: Label::with_short("Clicky Reset", "ClkRst"),
            ..Default::default()
        }),
        Keycode::QK_MUSIC_ON => Some(LayoutKey {
            tap: Label::with_short("Music On", "MusicOn"),
            ..Default::default()
        }),
        Keycode::QK_MUSIC_OFF => Some(LayoutKey {
            tap: Label::with_short("Music Off", "MusicOf"),
            ..Default::default()
        }),
        Keycode::QK_MUSIC_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Music Toggle", "MusicTg"),
            ..Default::default()
        }),
        Keycode::QK_MUSIC_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("Music Mode", "MusicMd"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_VOICE_NEXT => Some(LayoutKey {
            tap: Label::with_short("Voice Next", "Voice+"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_VOICE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("Voice Prev", "Voice-"),
            ..Default::default()
        }),
        Keycode::QK_STENO_BOLT => Some(LayoutKey {
            tap: Label::with_short("Steno Bolt", "StBolt"),
            ..Default::default()
        }),
        Keycode::QK_STENO_GEMINI => Some(LayoutKey {
            tap: Label::with_short("Steno Gemini", "StGem"),
            ..Default::default()
        }),
        Keycode::QK_STENO_COMB => Some(LayoutKey {
            tap: Label::with_short("Steno Comb", "StComb"),
            ..Default::default()
        }),
        Keycode::QK_STENO_COMB_MAX => Some(LayoutKey {
            tap: Label::with_short("Steno Comb Max", "StCMax"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_0 => Some(LayoutKey {
            tap: Label::with_short("Macro 0", "M0"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_1 => Some(LayoutKey {
            tap: Label::with_short("Macro 1", "M1"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_2 => Some(LayoutKey {
            tap: Label::with_short("Macro 2", "M2"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_3 => Some(LayoutKey {
            tap: Label::with_short("Macro 3", "M3"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_4 => Some(LayoutKey {
            tap: Label::with_short("Macro 4", "M4"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_5 => Some(LayoutKey {
            tap: Label::with_short("Macro 5", "M5"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_6 => Some(LayoutKey {
            tap: Label::with_short("Macro 6", "M6"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_7 => Some(LayoutKey {
            tap: Label::with_short("Macro 7", "M7"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_8 => Some(LayoutKey {
            tap: Label::with_short("Macro 8", "M8"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_9 => Some(LayoutKey {
            tap: Label::with_short("Macro 9", "M9"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_10 => Some(LayoutKey {
            tap: Label::with_short("Macro 10", "M10"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_11 => Some(LayoutKey {
            tap: Label::with_short("Macro 11", "M11"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_12 => Some(LayoutKey {
            tap: Label::with_short("Macro 12", "M12"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_13 => Some(LayoutKey {
            tap: Label::with_short("Macro 13", "M13"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_14 => Some(LayoutKey {
            tap: Label::with_short("Macro 14", "M14"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_15 => Some(LayoutKey {
            tap: Label::with_short("Macro 15", "M15"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_16 => Some(LayoutKey {
            tap: Label::with_short("Macro 16", "M16"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_17 => Some(LayoutKey {
            tap: Label::with_short("Macro 17", "M17"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_18 => Some(LayoutKey {
            tap: Label::with_short("Macro 18", "M18"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_19 => Some(LayoutKey {
            tap: Label::with_short("Macro 19", "M19"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_20 => Some(LayoutKey {
            tap: Label::with_short("Macro 20", "M20"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_21 => Some(LayoutKey {
            tap: Label::with_short("Macro 21", "M21"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_22 => Some(LayoutKey {
            tap: Label::with_short("Macro 22", "M22"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_23 => Some(LayoutKey {
            tap: Label::with_short("Macro 23", "M23"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_24 => Some(LayoutKey {
            tap: Label::with_short("Macro 24", "M24"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_25 => Some(LayoutKey {
            tap: Label::with_short("Macro 25", "M25"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_26 => Some(LayoutKey {
            tap: Label::with_short("Macro 26", "M26"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_27 => Some(LayoutKey {
            tap: Label::with_short("Macro 27", "M27"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_28 => Some(LayoutKey {
            tap: Label::with_short("Macro 28", "M28"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_29 => Some(LayoutKey {
            tap: Label::with_short("Macro 29", "M29"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_30 => Some(LayoutKey {
            tap: Label::with_short("Macro 30", "M30"),
            ..Default::default()
        }),
        Keycode::QK_MACRO_31 => Some(LayoutKey {
            tap: Label::with_short("Macro 31", "M31"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_ON => Some(LayoutKey {
            tap: Label::new("BL On"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_OFF => Some(LayoutKey {
            tap: Label::with_short("BL Off", "BL Off"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("BL Toggle", "BLTog"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_DOWN => Some(LayoutKey {
            tap: Label::new("BL -"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_UP => Some(LayoutKey {
            tap: Label::new("BL +"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_STEP => Some(LayoutKey {
            tap: Label::new("BL Cycle"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_TOGGLE_BREATHING => Some(LayoutKey {
            tap: Label::new("BR Toggle"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_ON => Some(LayoutKey {
            tap: Label::with_short("LED On", "LEDOn"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_OFF => Some(LayoutKey {
            tap: Label::with_short("LED Off", "LEDOff"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("RGB Toggle", "RGBTog"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_MODE_NEXT => Some(LayoutKey {
            tap: Label::new("RGB Mode +"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::new("RGB Mode -"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_BRIGHTNESS_UP => Some(LayoutKey {
            tap: Label::with_short("LED Bri +", "LED+"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_BRIGHTNESS_DOWN => Some(LayoutKey {
            tap: Label::with_short("LED Bri -", "LED-"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_SPEED_UP => Some(LayoutKey {
            tap: Label::with_short("LED Spd +", "LEDSp+"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_SPEED_DOWN => Some(LayoutKey {
            tap: Label::with_short("LED Spd -", "LEDSp-"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("UG Toggle", "UGTg"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("UG Mode +", "UGM+"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("UG Mode -", "UGM-"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_HUE_UP => Some(LayoutKey {
            tap: Label::new("Hue +"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_HUE_DOWN => Some(LayoutKey {
            tap: Label::new("Hue -"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_SATURATION_UP => Some(LayoutKey {
            tap: Label::new("Sat +"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_SATURATION_DOWN => Some(LayoutKey {
            tap: Label::new("Sat -"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_VALUE_UP => Some(LayoutKey {
            tap: Label::new("Bright +"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_VALUE_DOWN => Some(LayoutKey {
            tap: Label::new("Bright -"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_SPEED_UP => Some(LayoutKey {
            tap: Label::new("Effect Speed+"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_SPEED_DOWN => Some(LayoutKey {
            tap: Label::new("Effect Speed-"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_PLAIN => Some(LayoutKey {
            tap: Label::new("RGB Mode P"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_BREATHE => Some(LayoutKey {
            tap: Label::new("RGB Mode B"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_RAINBOW => Some(LayoutKey {
            tap: Label::new("RGB Mode R"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_SWIRL => Some(LayoutKey {
            tap: Label::new("RGB Mode SW"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_SNAKE => Some(LayoutKey {
            tap: Label::new("RGB Mode SN"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_KNIGHT => Some(LayoutKey {
            tap: Label::new("RGB Mode K"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_XMAS => Some(LayoutKey {
            tap: Label::new("RGB Mode X"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_GRADIENT => Some(LayoutKey {
            tap: Label::new("RGB Mode G"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_RGBTEST => Some(LayoutKey {
            tap: Label::new("RGB Mode Test"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_TWINKLE => Some(LayoutKey {
            tap: Label::new("RGB Mode T"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_ON => Some(LayoutKey {
            tap: Label::with_short("RGB On", "RGBOn"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_OFF => Some(LayoutKey {
            tap: Label::with_short("RGB Off", "RGBOff"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("RGB Toggle", "RGBTg"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("RGB Mode +", "RGBM+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("RGB Mode -", "RGBM-"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_HUE_UP => Some(LayoutKey {
            tap: Label::with_short("RGB Hue +", "RGBH+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_HUE_DOWN => Some(LayoutKey {
            tap: Label::with_short("RGB Hue -", "RGBH-"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_SATURATION_UP => Some(LayoutKey {
            tap: Label::with_short("RGB Sat +", "RGBS+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_SATURATION_DOWN => Some(LayoutKey {
            tap: Label::with_short("RGB Sat -", "RGBS-"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_VALUE_UP => Some(LayoutKey {
            tap: Label::with_short("RGB Val +", "RGBV+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_VALUE_DOWN => Some(LayoutKey {
            tap: Label::with_short("RGB Val -", "RGBV-"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_SPEED_UP => Some(LayoutKey {
            tap: Label::with_short("RGB Spd +", "RGBSp+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_SPEED_DOWN => Some(LayoutKey {
            tap: Label::with_short("RGB Spd -", "RGBSp-"),
            ..Default::default()
        }),
        Keycode::QK_BOOTLOADER => Some(LayoutKey {
            tap: Label::with_short("Bootloader", "Boot"),
            ..Default::default()
        }),
        Keycode::QK_REBOOT => Some(LayoutKey {
            tap: Label::with_short("Reboot", "Reboot"),
            ..Default::default()
        }),
        Keycode::QK_DEBUG_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Debug Toggle", "DbgTg"),
            ..Default::default()
        }),
        Keycode::QK_CLEAR_EEPROM => Some(LayoutKey {
            tap: Label::with_short("Clear EEPROM", "ClrEE"),
            ..Default::default()
        }),
        Keycode::QK_MAKE => Some(LayoutKey {
            tap: Label::with_short("Make", "Make"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_DOWN => Some(LayoutKey {
            tap: Label::with_short("AutoShift -", "AS -"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_UP => Some(LayoutKey {
            tap: Label::with_short("AutoShift +", "AS +"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_REPORT => Some(LayoutKey {
            tap: Label::with_short("AutoShift Rep", "AS R"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_ON => Some(LayoutKey {
            tap: Label::with_short("AutoShift On", "AS On"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_OFF => Some(LayoutKey {
            tap: Label::with_short("AutoShift Off", "ASOff"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("AutoShift Tog", "AS Tg"),
            ..Default::default()
        }),
        Keycode::QK_GRAVE_ESCAPE => Some(LayoutKey {
            tap: Label::new("Esc `"),
            ..Default::default()
        }),
        Keycode::QK_VELOCIKEY_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Velocikey", "VelKey"),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_LEFT_CTRL_PARENTHESIS_OPEN => Some(LayoutKey {
            tap: Label::new("LC ("),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_RIGHT_CTRL_PARENTHESIS_CLOSE => Some(LayoutKey {
            tap: Label::new("RC )"),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_LEFT_SHIFT_PARENTHESIS_OPEN => Some(LayoutKey {
            tap: Label::new("LS ("),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_RIGHT_SHIFT_PARENTHESIS_CLOSE => Some(LayoutKey {
            tap: Label::new("RS )"),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_LEFT_ALT_PARENTHESIS_OPEN => Some(LayoutKey {
            tap: Label::new("LA ("),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_RIGHT_ALT_PARENTHESIS_CLOSE => Some(LayoutKey {
            tap: Label::new("RA )"),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_RIGHT_SHIFT_ENTER => Some(LayoutKey {
            tap: Label::new("SftEnt"),
            ..Default::default()
        }),
        Keycode::QK_OUTPUT_AUTO => Some(LayoutKey {
            tap: Label::with_short("Out Auto", "OutAuto"),
            ..Default::default()
        }),
        Keycode::QK_OUTPUT_USB => Some(LayoutKey {
            tap: Label::with_short("Out USB", "OutUSB"),
            ..Default::default()
        }),
        Keycode::QK_OUTPUT_BLUETOOTH => Some(LayoutKey {
            tap: Label::with_short("Out BT", "OutBT"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("Unicode +", "Uni +"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("Unicode -", "Uni -"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_MACOS => Some(LayoutKey {
            tap: Label::with_short("Unicode macOS", "UniMac"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_LINUX => Some(LayoutKey {
            tap: Label::with_short("Unicode Linux", "UniLnx"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_WINDOWS => Some(LayoutKey {
            tap: Label::with_short("Unicode Win", "UniWin"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_BSD => Some(LayoutKey {
            tap: Label::with_short("Unicode BSD", "UniBSD"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_WINCOMPOSE => Some(LayoutKey {
            tap: Label::with_short("Unicode WinC", "UniWinC"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_EMACS => Some(LayoutKey {
            tap: Label::with_short("Unicode Emacs", "UniEm"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_ON => Some(LayoutKey {
            tap: Label::with_short("Haptic On", "HapOn"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_OFF => Some(LayoutKey {
            tap: Label::with_short("Haptic Off", "HapOff"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Haptic Toggle", "HapTg"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_RESET => Some(LayoutKey {
            tap: Label::with_short("Haptic Reset", "HapRst"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_FEEDBACK_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Haptic FB Tog", "HapFBTg"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_BUZZ_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Haptic Buzz", "HapBuzz"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("Haptic +", "Hap +"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("Haptic -", "Hap -"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_CONTINUOUS_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Haptic Cont", "HapCont"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_CONTINUOUS_UP => Some(LayoutKey {
            tap: Label::with_short("Haptic + ", "HapC+"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_CONTINUOUS_DOWN => Some(LayoutKey {
            tap: Label::with_short("Haptic -", "HapC-"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_DWELL_UP => Some(LayoutKey {
            tap: Label::with_short("Haptic Dwell +", "HapDw+"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_DWELL_DOWN => Some(LayoutKey {
            tap: Label::with_short("Haptic Dwell -", "HapDw-"),
            ..Default::default()
        }),
        Keycode::QK_COMBO_ON => Some(LayoutKey {
            tap: Label::with_short("Combo On", "CombOn"),
            ..Default::default()
        }),
        Keycode::QK_COMBO_OFF => Some(LayoutKey {
            tap: Label::with_short("Combo Off", "CombOff"),
            ..Default::default()
        }),
        Keycode::QK_COMBO_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Combo Toggle", "CombTg"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_RECORD_START_1 => Some(LayoutKey {
            tap: Label::with_short("DM Rec 1", "DMRec1"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_RECORD_START_2 => Some(LayoutKey {
            tap: Label::with_short("DM Rec 2", "DMRec2"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_RECORD_STOP => Some(LayoutKey {
            tap: Label::with_short("DM Stop", "DMStop"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_PLAY_1 => Some(LayoutKey {
            tap: Label::with_short("DM Play 1", "DMPlay1"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_PLAY_2 => Some(LayoutKey {
            tap: Label::with_short("DM Play 2", "DMPlay2"),
            ..Default::default()
        }),
        Keycode::QK_LEADER => Some(LayoutKey {
            tap: Label::with_short("Leader", "Lead"),
            ..Default::default()
        }),
        Keycode::QK_LOCK => Some(LayoutKey {
            tap: Label::with_short("Lock", "Lock"),
            ..Default::default()
        }),
        Keycode::QK_ONE_SHOT_ON => Some(LayoutKey {
            tap: Label::with_short("OneShot On", "1ShotOn"),
            ..Default::default()
        }),
        Keycode::QK_ONE_SHOT_OFF => Some(LayoutKey {
            tap: Label::with_short("OneShot Off", "1ShotOf"),
            ..Default::default()
        }),
        Keycode::QK_ONE_SHOT_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("OneShot Toggle", "1ShotTg"),
            ..Default::default()
        }),
        Keycode::QK_KEY_OVERRIDE_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("KO Toggle", "KOTg"),
            ..Default::default()
        }),
        Keycode::QK_KEY_OVERRIDE_ON => Some(LayoutKey {
            tap: Label::with_short("KO On", "KOOn"),
            ..Default::default()
        }),
        Keycode::QK_KEY_OVERRIDE_OFF => Some(LayoutKey {
            tap: Label::with_short("KO Off", "KOOff"),
            ..Default::default()
        }),
        Keycode::QK_SECURE_LOCK => Some(LayoutKey {
            tap: Label::with_short("Secure Lock", "SecLock"),
            ..Default::default()
        }),
        Keycode::QK_SECURE_UNLOCK => Some(LayoutKey {
            tap: Label::with_short("Secure Unlock", "SecUnlk"),
            ..Default::default()
        }),
        Keycode::QK_SECURE_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Secure Toggle", "SecTg"),
            ..Default::default()
        }),
        Keycode::QK_SECURE_REQUEST => Some(LayoutKey {
            tap: Label::with_short("Secure Request", "SecReq"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_TAPPING_TERM_PRINT => Some(LayoutKey {
            tap: Label::with_short("DT Term", "DTTerm"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_TAPPING_TERM_UP => Some(LayoutKey {
            tap: Label::with_short("DT Term +", "DTTerm+"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_TAPPING_TERM_DOWN => Some(LayoutKey {
            tap: Label::with_short("DT Term -", "DTTerm-"),
            ..Default::default()
        }),
        Keycode::QK_CAPS_WORD_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Caps Word", "CapWord"),
            ..Default::default()
        }),
        Keycode::QK_AUTOCORRECT_ON => Some(LayoutKey {
            tap: Label::with_short("Autocorrect On", "ACOn"),
            ..Default::default()
        }),
        Keycode::QK_AUTOCORRECT_OFF => Some(LayoutKey {
            tap: Label::with_short("Autocorrect Off", "ACOff"),
            ..Default::default()
        }),
        Keycode::QK_AUTOCORRECT_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Autocorrect Tog", "ACTg"),
            ..Default::default()
        }),
        Keycode::QK_TRI_LAYER_LOWER => Some(LayoutKey {
            tap: Label::with_short("Tri Lower", "TriLow"),
            ..Default::default()
        }),
        Keycode::QK_TRI_LAYER_UPPER => Some(LayoutKey {
            tap: Label::with_short("Tri Upper", "TriUp"),
            ..Default::default()
        }),
        Keycode::QK_REPEAT_KEY => Some(LayoutKey {
            tap: Label::with_short("Repeat Key", "RepKey"),
            ..Default::default()
        }),
        Keycode::QK_ALT_REPEAT_KEY => Some(LayoutKey {
            tap: Label::with_short("Alt Repeat", "ARepKey"),
            ..Default::default()
        }),
        Keycode::QK_KB_0 => Some(LayoutKey {
            tap: Label::with_short("KB 0", "KB0"),
            ..Default::default()
        }),
        Keycode::QK_KB_1 => Some(LayoutKey {
            tap: Label::with_short("KB 1", "KB1"),
            ..Default::default()
        }),
        Keycode::QK_KB_2 => Some(LayoutKey {
            tap: Label::with_short("KB 2", "KB2"),
            ..Default::default()
        }),
        Keycode::QK_KB_3 => Some(LayoutKey {
            tap: Label::with_short("KB 3", "KB3"),
            ..Default::default()
        }),
        Keycode::QK_KB_4 => Some(LayoutKey {
            tap: Label::with_short("KB 4", "KB4"),
            ..Default::default()
        }),
        Keycode::QK_KB_5 => Some(LayoutKey {
            tap: Label::with_short("KB 5", "KB5"),
            ..Default::default()
        }),
        Keycode::QK_KB_6 => Some(LayoutKey {
            tap: Label::with_short("KB 6", "KB6"),
            ..Default::default()
        }),
        Keycode::QK_KB_7 => Some(LayoutKey {
            tap: Label::with_short("KB 7", "KB7"),
            ..Default::default()
        }),
        Keycode::QK_KB_8 => Some(LayoutKey {
            tap: Label::with_short("KB 8", "KB8"),
            ..Default::default()
        }),
        Keycode::QK_KB_9 => Some(LayoutKey {
            tap: Label::with_short("KB 9", "KB9"),
            ..Default::default()
        }),
        Keycode::QK_KB_10 => Some(LayoutKey {
            tap: Label::with_short("KB 10", "KB10"),
            ..Default::default()
        }),
        Keycode::QK_KB_11 => Some(LayoutKey {
            tap: Label::with_short("KB 11", "KB11"),
            ..Default::default()
        }),
        Keycode::QK_KB_12 => Some(LayoutKey {
            tap: Label::with_short("KB 12", "KB12"),
            ..Default::default()
        }),
        Keycode::QK_KB_13 => Some(LayoutKey {
            tap: Label::with_short("KB 13", "KB13"),
            ..Default::default()
        }),
        Keycode::QK_KB_14 => Some(LayoutKey {
            tap: Label::with_short("KB 14", "KB14"),
            ..Default::default()
        }),
        Keycode::QK_KB_15 => Some(LayoutKey {
            tap: Label::with_short("KB 15", "KB15"),
            ..Default::default()
        }),
        Keycode::QK_KB_16 => Some(LayoutKey {
            tap: Label::with_short("KB 16", "KB16"),
            ..Default::default()
        }),
        Keycode::QK_KB_17 => Some(LayoutKey {
            tap: Label::with_short("KB 17", "KB17"),
            ..Default::default()
        }),
        Keycode::QK_KB_18 => Some(LayoutKey {
            tap: Label::with_short("KB 18", "KB18"),
            ..Default::default()
        }),
        Keycode::QK_KB_19 => Some(LayoutKey {
            tap: Label::with_short("KB 19", "KB19"),
            ..Default::default()
        }),
        Keycode::QK_KB_20 => Some(LayoutKey {
            tap: Label::with_short("KB 20", "KB20"),
            ..Default::default()
        }),
        Keycode::QK_KB_21 => Some(LayoutKey {
            tap: Label::with_short("KB 21", "KB21"),
            ..Default::default()
        }),
        Keycode::QK_KB_22 => Some(LayoutKey {
            tap: Label::with_short("KB 22", "KB22"),
            ..Default::default()
        }),
        Keycode::QK_KB_23 => Some(LayoutKey {
            tap: Label::with_short("KB 23", "KB23"),
            ..Default::default()
        }),
        Keycode::QK_KB_24 => Some(LayoutKey {
            tap: Label::with_short("KB 24", "KB24"),
            ..Default::default()
        }),
        Keycode::QK_KB_25 => Some(LayoutKey {
            tap: Label::with_short("KB 25", "KB25"),
            ..Default::default()
        }),
        Keycode::QK_KB_26 => Some(LayoutKey {
            tap: Label::with_short("KB 26", "KB26"),
            ..Default::default()
        }),
        Keycode::QK_KB_27 => Some(LayoutKey {
            tap: Label::with_short("KB 27", "KB27"),
            ..Default::default()
        }),
        Keycode::QK_KB_28 => Some(LayoutKey {
            tap: Label::with_short("KB 28", "KB28"),
            ..Default::default()
        }),
        Keycode::QK_KB_29 => Some(LayoutKey {
            tap: Label::with_short("KB 29", "KB29"),
            ..Default::default()
        }),
        Keycode::QK_KB_30 => Some(LayoutKey {
            tap: Label::with_short("KB 30", "KB30"),
            ..Default::default()
        }),
        Keycode::QK_KB_31 => Some(LayoutKey {
            tap: Label::with_short("KB 31", "KB31"),
            ..Default::default()
        }),
        Keycode::QK_USER_0 => Some(LayoutKey {
            tap: Label::with_short("User 0", "Usr0"),
            ..Default::default()
        }),
        Keycode::QK_USER_1 => Some(LayoutKey {
            tap: Label::with_short("User 1", "Usr1"),
            ..Default::default()
        }),
        Keycode::QK_USER_2 => Some(LayoutKey {
            tap: Label::with_short("User 2", "Usr2"),
            ..Default::default()
        }),
        Keycode::QK_USER_3 => Some(LayoutKey {
            tap: Label::with_short("User 3", "Usr3"),
            ..Default::default()
        }),
        Keycode::QK_USER_4 => Some(LayoutKey {
            tap: Label::with_short("User 4", "Usr4"),
            ..Default::default()
        }),
        Keycode::QK_USER_5 => Some(LayoutKey {
            tap: Label::with_short("User 5", "Usr5"),
            ..Default::default()
        }),
        Keycode::QK_USER_6 => Some(LayoutKey {
            tap: Label::with_short("User 6", "Usr6"),
            ..Default::default()
        }),
        Keycode::QK_USER_7 => Some(LayoutKey {
            tap: Label::with_short("User 7", "Usr7"),
            ..Default::default()
        }),
        Keycode::QK_USER_8 => Some(LayoutKey {
            tap: Label::with_short("User 8", "Usr8"),
            ..Default::default()
        }),
        Keycode::QK_USER_9 => Some(LayoutKey {
            tap: Label::with_short("User 9", "Usr9"),
            ..Default::default()
        }),
        Keycode::QK_USER_10 => Some(LayoutKey {
            tap: Label::with_short("User 10", "Usr10"),
            ..Default::default()
        }),
        Keycode::QK_USER_11 => Some(LayoutKey {
            tap: Label::with_short("User 11", "Usr11"),
            ..Default::default()
        }),
        Keycode::QK_USER_12 => Some(LayoutKey {
            tap: Label::with_short("User 12", "Usr12"),
            ..Default::default()
        }),
        Keycode::QK_USER_13 => Some(LayoutKey {
            tap: Label::with_short("User 13", "Usr13"),
            ..Default::default()
        }),
        Keycode::QK_USER_14 => Some(LayoutKey {
            tap: Label::with_short("User 14", "Usr14"),
            ..Default::default()
        }),
        Keycode::QK_USER_15 => Some(LayoutKey {
            tap: Label::with_short("User 15", "Usr15"),
            ..Default::default()
        }),
        Keycode::QK_USER_16 => Some(LayoutKey {
            tap: Label::with_short("User 16", "Usr16"),
            ..Default::default()
        }),
        Keycode::QK_USER_17 => Some(LayoutKey {
            tap: Label::with_short("User 17", "Usr17"),
            ..Default::default()
        }),
        Keycode::QK_USER_18 => Some(LayoutKey {
            tap: Label::with_short("User 18", "Usr18"),
            ..Default::default()
        }),
        Keycode::QK_USER_19 => Some(LayoutKey {
            tap: Label::with_short("User 19", "Usr19"),
            ..Default::default()
        }),
        Keycode::QK_USER_20 => Some(LayoutKey {
            tap: Label::with_short("User 20", "Usr20"),
            ..Default::default()
        }),
        Keycode::QK_USER_21 => Some(LayoutKey {
            tap: Label::with_short("User 21", "Usr21"),
            ..Default::default()
        }),
        Keycode::QK_USER_22 => Some(LayoutKey {
            tap: Label::with_short("User 22", "Usr22"),
            ..Default::default()
        }),
        Keycode::QK_USER_23 => Some(LayoutKey {
            tap: Label::with_short("User 23", "Usr23"),
            ..Default::default()
        }),
        Keycode::QK_USER_24 => Some(LayoutKey {
            tap: Label::with_short("User 24", "Usr24"),
            ..Default::default()
        }),
        Keycode::QK_USER_25 => Some(LayoutKey {
            tap: Label::with_short("User 25", "Usr25"),
            ..Default::default()
        }),
        Keycode::QK_USER_26 => Some(LayoutKey {
            tap: Label::with_short("User 26", "Usr26"),
            ..Default::default()
        }),
        Keycode::QK_USER_27 => Some(LayoutKey {
            tap: Label::with_short("User 27", "Usr27"),
            ..Default::default()
        }),
        Keycode::QK_USER_28 => Some(LayoutKey {
            tap: Label::with_short("User 28", "Usr28"),
            ..Default::default()
        }),
        Keycode::QK_USER_29 => Some(LayoutKey {
            tap: Label::with_short("User 29", "Usr29"),
            ..Default::default()
        }),
        Keycode::QK_USER_30 => Some(LayoutKey {
            tap: Label::with_short("User 30", "Usr30"),
            ..Default::default()
        }),
        Keycode::QK_USER_31 => Some(LayoutKey {
            tap: Label::with_short("User 31", "Usr31"),
            ..Default::default()
        }),
    }
}
