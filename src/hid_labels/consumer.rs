use crate::layout_key::{Label, LayoutKey};

/// Resolves a USB HID Consumer Page (0x0C) usage ID into a LayoutKey.
pub fn hid_consumer_key(usage_id: u16) -> Option<LayoutKey> {
    match usage_id {

        0x30 => Some(LayoutKey {
            tap: Label::new("Power"),
            ..Default::default()
        }),

        0x31 => Some(LayoutKey {
            tap: Label::new("Reset"),
            ..Default::default()
        }),

        0x32 => Some(LayoutKey {
            tap: Label::new("Sleep"),
            ..Default::default()
        }),

        0x34 => Some(LayoutKey {
            tap: Label::with_short("Sleep Mode", "Slp"),
            ..Default::default()
        }),

        0x40 => Some(LayoutKey {
            tap: Label::new("Menu"),
            ..Default::default()
        }),

        0x41 => Some(LayoutKey {
            tap: Label::with_short("Menu Select", "MSel"),
            ..Default::default()
        }),

        0x42 => Some(LayoutKey {
            tap: Label::with_short("Menu Up", "MUp"),
            ..Default::default()
        }),

        0x43 => Some(LayoutKey {
            tap: Label::with_short("Menu Down", "MDn"),
            ..Default::default()
        }),

        0x44 => Some(LayoutKey {
            tap: Label::with_short("Menu Left", "MLft"),
            ..Default::default()
        }),

        0x45 => Some(LayoutKey {
            tap: Label::with_short("Menu Right", "MRgt"),
            ..Default::default()
        }),

        0x46 => Some(LayoutKey {
            tap: Label::with_short("Menu Escape", "MEsc"),
            ..Default::default()
        }),

        0x47 => Some(LayoutKey {
            tap: Label::with_short("Menu Increase", "M+"),
            ..Default::default()
        }),

        0x48 => Some(LayoutKey {
            tap: Label::with_short("Menu Decrease", "M-"),
            ..Default::default()
        }),

        0x60 => Some(LayoutKey {
            tap: Label::with_short("Data on Screen", "OSD"),
            ..Default::default()
        }),

        0x61 => Some(LayoutKey {
            tap: Label::with_short("Subtitles", "Sub"),
            ..Default::default()
        }),

        0x65 => Some(LayoutKey {
            tap: Label::with_short("Snapshot", "Snap"),
            ..Default::default()
        }),

        0x67 => Some(LayoutKey {
            tap: Label::new("PIP"),
            ..Default::default()
        }),

        0x69 => Some(LayoutKey {
            tap: Label::new("Red"),
            ..Default::default()
        }),

        0x6A => Some(LayoutKey {
            tap: Label::new("Green"),
            ..Default::default()
        }),

        0x6B => Some(LayoutKey {
            tap: Label::new("Blue"),
            ..Default::default()
        }),

        0x6C => Some(LayoutKey {
            tap: Label::new("Yellow"),
            ..Default::default()
        }),

        0x6D => Some(LayoutKey {
            tap: Label::with_short("Aspect", "Asp"),
            ..Default::default()
        }),

        0x6F => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SUN.to_string()),
            ..Default::default()
        }),

        0x70 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SUN_DIM.to_string()),
            ..Default::default()
        }),

        0x72 => Some(LayoutKey {
            tap: Label::with_short("Backlight Toggle", "BLTog"),
            ..Default::default()
        }),

        0x73 => Some(LayoutKey {
            tap: Label::with_short("Brightness Min", "BriMin"),
            ..Default::default()
        }),

        0x74 => Some(LayoutKey {
            tap: Label::with_short("Brightness Max", "BriMax"),
            ..Default::default()
        }),

        0x75 => Some(LayoutKey {
            tap: Label::with_short("Brightness Auto", "BriAuto"),
            ..Default::default()
        }),

        0x82 => Some(LayoutKey {
            tap: Label::with_short("Mode Step", "Step"),
            ..Default::default()
        }),

        0x83 => Some(LayoutKey {
            tap: Label::with_short("Last Channel", "Last"),
            ..Default::default()
        }),

        0x88 => Some(LayoutKey {
            tap: Label::new("Computer"),
            ..Default::default()
        }),

        0x89 => Some(LayoutKey {
            tap: Label::new("TV"),
            ..Default::default()
        }),

        0x8A => Some(LayoutKey {
            tap: Label::new("WWW"),
            ..Default::default()
        }),

        0x8B => Some(LayoutKey {
            tap: Label::new("DVD"),
            ..Default::default()
        }),

        0x8C => Some(LayoutKey {
            tap: Label::new("Phone"),
            ..Default::default()
        }),

        0x8D => Some(LayoutKey {
            tap: Label::new("Guide"),
            ..Default::default()
        }),

        0x8E => Some(LayoutKey {
            tap: Label::with_short("Video Phone", "VidPh"),
            ..Default::default()
        }),

        0x8F => Some(LayoutKey {
            tap: Label::new("Games"),
            ..Default::default()
        }),

        0x90 => Some(LayoutKey {
            tap: Label::new("Messages"),
            ..Default::default()
        }),

        0x91 => Some(LayoutKey {
            tap: Label::new("CD"),
            ..Default::default()
        }),

        0x92 => Some(LayoutKey {
            tap: Label::new("VCR"),
            ..Default::default()
        }),

        0x93 => Some(LayoutKey {
            tap: Label::new("Tuner"),
            ..Default::default()
        }),

        0x94 => Some(LayoutKey {
            tap: Label::new("Quit"),
            ..Default::default()
        }),

        0x95 => Some(LayoutKey {
            tap: Label::new("Help"),
            ..Default::default()
        }),

        0x96 => Some(LayoutKey {
            tap: Label::new("Tape"),
            ..Default::default()
        }),

        0x97 => Some(LayoutKey {
            tap: Label::new("Cable"),
            ..Default::default()
        }),

        0x98 => Some(LayoutKey {
            tap: Label::with_short("Satellite", "Sat"),
            ..Default::default()
        }),

        0x9A => Some(LayoutKey {
            tap: Label::with_short("Media Home", "Home"),
            ..Default::default()
        }),

        0x9C => Some(LayoutKey {
            tap: Label::with_short("Channel +", "Ch+"),
            ..Default::default()
        }),

        0x9D => Some(LayoutKey {
            tap: Label::with_short("Channel -", "Ch-"),
            ..Default::default()
        }),

        0xA0 => Some(LayoutKey {
            tap: Label::with_short("VCR Plus", "VCR+"),
            ..Default::default()
        }),

        0xB0 => Some(LayoutKey {
            tap: Label::new("Play"),
            ..Default::default()
        }),

        0xB1 => Some(LayoutKey {
            tap: Label::new("Pause"),
            ..Default::default()
        }),

        0xB2 => Some(LayoutKey {
            tap: Label::with_short("Record", "Rec"),
            ..Default::default()
        }),

        0xB3 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::FAST_FORWARD.to_string()),
            ..Default::default()
        }),

        0xB4 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::REWIND.to_string()),
            ..Default::default()
        }),

        0xB5 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_FORWARD.to_string()),
            ..Default::default()
        }),

        0xB6 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SKIP_BACK.to_string()),
            ..Default::default()
        }),

        0xB7 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::STOP.to_string()),
            ..Default::default()
        }),

        0xB8 => Some(LayoutKey {
            tap: Label::with_short("Eject", "Ejct"),
            ..Default::default()
        }),

        0xB9 => Some(LayoutKey {
            tap: Label::with_short("Shuffle", "Shfl"),
            symbol: Some(egui_phosphor::regular::SHUFFLE.to_string()),
            ..Default::default()
        }),

        0xBC => Some(LayoutKey {
            tap: Label::with_short("Repeat", "Rpt"),
            symbol: Some(egui_phosphor::regular::REPEAT.to_string()),
            ..Default::default()
        }),

        0xBF => Some(LayoutKey {
            tap: Label::new("Slow"),
            ..Default::default()
        }),

        0xCC => Some(LayoutKey {
            tap: Label::with_short("Stop/Eject", "StEj"),
            ..Default::default()
        }),

        0xCD => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::PLAY_PAUSE.to_string()),
            ..Default::default()
        }),

        0xCF => Some(LayoutKey {
            tap: Label::with_short("Voice Command", "Voice"),
            symbol: Some(egui_phosphor::regular::MICROPHONE.to_string()),
            ..Default::default()
        }),

        0xE2 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_X.to_string()),
            ..Default::default()
        }),

        0xE5 => Some(LayoutKey {
            tap: Label::with_short("Bass Boost", "Bass"),
            ..Default::default()
        }),

        0xE9 => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
            ..Default::default()
        }),

        0xEA => Some(LayoutKey {
            symbol: Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
            ..Default::default()
        }),

        0xF5 => Some(LayoutKey {
            tap: Label::new("Slow"),
            ..Default::default()
        }),

        0x173 => Some(LayoutKey {
            tap: Label::with_short("Alt Audio Inc", "Aud+"),
            ..Default::default()
        }),

        0x183 => Some(LayoutKey {
            tap: Label::new("CCC"),
            ..Default::default()
        }),

        0x184 => Some(LayoutKey {
            tap: Label::new("Word"),
            ..Default::default()
        }),

        0x185 => Some(LayoutKey {
            tap: Label::with_short("Text Editor", "Edit"),
            ..Default::default()
        }),

        0x186 => Some(LayoutKey {
            tap: Label::with_short("Spreadsheet", "Sheet"),
            ..Default::default()
        }),

        0x187 => Some(LayoutKey {
            tap: Label::with_short("Graphics Editor", "Gfx"),
            ..Default::default()
        }),

        0x188 => Some(LayoutKey {
            tap: Label::with_short("Presentation", "Present"),
            ..Default::default()
        }),

        0x189 => Some(LayoutKey {
            tap: Label::new("DB"),
            ..Default::default()
        }),

        0x18A => Some(LayoutKey {
            tap: Label::new("Mail"),
            ..Default::default()
        }),

        0x18B => Some(LayoutKey {
            tap: Label::new("News"),
            ..Default::default()
        }),

        0x18C => Some(LayoutKey {
            tap: Label::with_short("Voicemail", "VMail"),
            ..Default::default()
        }),

        0x18D => Some(LayoutKey {
            tap: Label::new("Contacts"),
            ..Default::default()
        }),

        0x18E => Some(LayoutKey {
            tap: Label::with_short("Calendar", "Cal"),
            ..Default::default()
        }),

        0x18F => Some(LayoutKey {
            tap: Label::with_short("Task Manager", "TaskMgr"),
            ..Default::default()
        }),

        0x190 => Some(LayoutKey {
            tap: Label::with_short("Journal", "Jrnl"),
            ..Default::default()
        }),

        0x191 => Some(LayoutKey {
            tap: Label::with_short("Finance", "Fin"),
            ..Default::default()
        }),

        0x192 => Some(LayoutKey {
            tap: Label::new("Calc"),
            ..Default::default()
        }),

        0x193 => Some(LayoutKey {
            tap: Label::with_short("AV Capture Playback", "AVCap"),
            ..Default::default()
        }),

        0x194 => Some(LayoutKey {
            tap: Label::new("My Comp"),
            ..Default::default()
        }),

        0x196 => Some(LayoutKey {
            tap: Label::new("WWW"),
            ..Default::default()
        }),

        0x199 => Some(LayoutKey {
            tap: Label::new("Chat"),
            ..Default::default()
        }),

        0x19C => Some(LayoutKey {
            tap: Label::with_short("Log Off", "LogOff"),
            ..Default::default()
        }),

        0x19E => Some(LayoutKey {
            tap: Label::with_short("Screen Saver", "ScrSv"),
            ..Default::default()
        }),

        0x19F => Some(LayoutKey {
            tap: Label::with_short("Control Panel", "Ctrl P"),
            ..Default::default()
        }),

        0x1A2 => Some(LayoutKey {
            tap: Label::with_short("Select Task", "SelTk"),
            ..Default::default()
        }),

        0x1A3 => Some(LayoutKey {
            tap: Label::with_short("Next Task", "NextTk"),
            ..Default::default()
        }),

        0x1A4 => Some(LayoutKey {
            tap: Label::with_short("Previous Task", "PrevTk"),
            ..Default::default()
        }),

        0x1A6 => Some(LayoutKey {
            tap: Label::new("Help"),
            ..Default::default()
        }),

        0x1A7 => Some(LayoutKey {
            tap: Label::new("Docs"),
            ..Default::default()
        }),

        0x1AB => Some(LayoutKey {
            tap: Label::with_short("Spellcheck", "Spell"),
            ..Default::default()
        }),

        0x1AE => Some(LayoutKey {
            tap: Label::with_short("Keyboard Layout", "KbdLy"),
            ..Default::default()
        }),

        0x1B1 => Some(LayoutKey {
            tap: Label::with_short("Screen Saver", "ScrSv"),
            ..Default::default()
        }),

        0x1B4 => Some(LayoutKey {
            tap: Label::new("Files"),
            ..Default::default()
        }),

        0x1B6 => Some(LayoutKey {
            tap: Label::new("Images"),
            ..Default::default()
        }),

        0x1B7 => Some(LayoutKey {
            tap: Label::new("Audio"),
            ..Default::default()
        }),

        0x1B8 => Some(LayoutKey {
            tap: Label::new("Movies"),
            ..Default::default()
        }),

        0x1BC => Some(LayoutKey {
            tap: Label::with_short("Instant Messaging", "IM"),
            ..Default::default()
        }),

        0x1BD => Some(LayoutKey {
            tap: Label::with_short("OEM Features", "OEM"),
            ..Default::default()
        }),

        0x201 => Some(LayoutKey {
            tap: Label::new("New"),
            ..Default::default()
        }),

        0x202 => Some(LayoutKey {
            tap: Label::new("Open"),
            ..Default::default()
        }),

        0x203 => Some(LayoutKey {
            tap: Label::new("Close"),
            ..Default::default()
        }),

        0x204 => Some(LayoutKey {
            tap: Label::new("Exit"),
            ..Default::default()
        }),

        0x207 => Some(LayoutKey {
            tap: Label::new("Save"),
            ..Default::default()
        }),

        0x208 => Some(LayoutKey {
            tap: Label::new("Print"),
            ..Default::default()
        }),

        0x209 => Some(LayoutKey {
            tap: Label::with_short("Properties", "Props"),
            ..Default::default()
        }),

        0x21A => Some(LayoutKey {
            tap: Label::new("Undo"),
            ..Default::default()
        }),

        0x21B => Some(LayoutKey {
            tap: Label::new("Copy"),
            ..Default::default()
        }),

        0x21C => Some(LayoutKey {
            tap: Label::new("Cut"),
            ..Default::default()
        }),

        0x21D => Some(LayoutKey {
            tap: Label::new("Paste"),
            ..Default::default()
        }),

        0x21F => Some(LayoutKey {
            tap: Label::new("Find"),
            ..Default::default()
        }),

        0x221 => Some(LayoutKey {
            tap: Label::new("Search"),
            ..Default::default()
        }),

        0x222 => Some(LayoutKey {
            tap: Label::with_short("Go To", "GoTo"),
            ..Default::default()
        }),

        0x223 => Some(LayoutKey {
            tap: Label::new("Home"),
            ..Default::default()
        }),

        0x224 => Some(LayoutKey {
            tap: Label::new("Back"),
            ..Default::default()
        }),

        0x225 => Some(LayoutKey {
            tap: Label::new("Forward"),
            ..Default::default()
        }),

        0x226 => Some(LayoutKey {
            tap: Label::new("Stop"),
            ..Default::default()
        }),

        0x227 => Some(LayoutKey {
            tap: Label::new("Refresh"),
            ..Default::default()
        }),

        0x22A => Some(LayoutKey {
            tap: Label::new("Favorites"),
            ..Default::default()
        }),

        0x22D => Some(LayoutKey {
            tap: Label::with_short("Zoom In", "Z+"),
            ..Default::default()
        }),

        0x22E => Some(LayoutKey {
            tap: Label::with_short("Zoom Out", "Z-"),
            ..Default::default()
        }),

        0x22F => Some(LayoutKey {
            tap: Label::new("Zoom"),
            ..Default::default()
        }),

        0x232 => Some(LayoutKey {
            tap: Label::with_short("View Toggle", "View"),
            ..Default::default()
        }),

        0x233 => Some(LayoutKey {
            tap: Label::with_short("Scroll Up", "ScrUp"),
            ..Default::default()
        }),

        0x234 => Some(LayoutKey {
            tap: Label::with_short("Scroll Down", "ScrDn"),
            ..Default::default()
        }),

        0x23D => Some(LayoutKey {
            tap: Label::new("Edit"),
            ..Default::default()
        }),

        0x25F => Some(LayoutKey {
            tap: Label::new("Cancel"),
            ..Default::default()
        }),

        0x269 => Some(LayoutKey {
            tap: Label::with_short("Insert", "Ins"),
            ..Default::default()
        }),

        0x26A => Some(LayoutKey {
            tap: Label::with_short("Delete", "Del"),
            ..Default::default()
        }),

        0x279 => Some(LayoutKey {
            tap: Label::new("Redo"),
            ..Default::default()
        }),

        0x289 => Some(LayoutKey {
            tap: Label::new("Reply"),
            ..Default::default()
        }),

        0x28B => Some(LayoutKey {
            tap: Label::with_short("Forward Mail", "Fwd"),
            ..Default::default()
        }),

        0x28C => Some(LayoutKey {
            tap: Label::new("Send"),
            ..Default::default()
        }),

        0x29D => Some(LayoutKey {
            tap: Label::new("Globe"),
            symbol: Some(egui_phosphor::regular::GLOBE.to_string()),
            ..Default::default()
        }),

        0x29F => Some(LayoutKey {
            tap: Label::with_short("Show All Windows", "AllWin"),
            ..Default::default()
        }),

        0x2A2 => Some(LayoutKey {
            tap: Label::with_short("Show All Apps", "AllApp"),
            ..Default::default()
        }),

        0x2C7 => Some(LayoutKey {
            tap: Label::with_short("KB Assist Prev", "KBIALft"),
            ..Default::default()
        }),

        0x2C8 => Some(LayoutKey {
            tap: Label::with_short("KB Assist Next", "KBIARgt"),
            ..Default::default()
        }),

        0x2C9 => Some(LayoutKey {
            tap: Label::with_short("KB Assist Prev Group", "KBIAGLft"),
            ..Default::default()
        }),

        0x2CA => Some(LayoutKey {
            tap: Label::with_short("KB Assist Next Group", "KBIAGRgt"),
            ..Default::default()
        }),

        0x2CB => Some(LayoutKey {
            tap: Label::with_short("KB Assist Accept", "KBIAOK"),
            ..Default::default()
        }),

        0x2CC => Some(LayoutKey {
            tap: Label::with_short("KB Assist Cancel", "KBIAX"),
            ..Default::default()
        }),
        _ => None,
    }
}
