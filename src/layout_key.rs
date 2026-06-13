/// Symbols for the four modifier keys. macOS uses native glyphs (⌃ ⇧ ⌥ ⌘);
/// Windows/Linux keep ⇧ for Shift but use shrinkable text names for the rest.
pub mod modifier_symbols {
    /// Full/short display names for a modifier (same glyph for both on glyph modifiers).
    pub struct ModName {
        pub full: &'static str,
        pub short: &'static str,
    }

    #[cfg(target_os = "macos")]
    pub const MOD_CTRL: ModName = ModName {
        full: egui_phosphor::regular::CONTROL,
        short: egui_phosphor::regular::CONTROL,
    };
    #[cfg(not(target_os = "macos"))]
    pub const MOD_CTRL: ModName = ModName {
        full: "Ctrl",
        short: "Ctl",
    };

    pub const MOD_SHIFT: ModName = ModName {
        full: egui_phosphor::regular::ARROW_FAT_UP,
        short: egui_phosphor::regular::ARROW_FAT_UP,
    };

    #[cfg(target_os = "macos")]
    pub const MOD_ALT: ModName = ModName {
        full: egui_phosphor::regular::OPTION,
        short: egui_phosphor::regular::OPTION,
    };
    #[cfg(not(target_os = "macos"))]
    pub const MOD_ALT: ModName = ModName {
        full: "Alt",
        short: "Alt",
    };

    #[cfg(target_os = "macos")]
    pub const MOD_GUI: ModName = ModName {
        full: egui_phosphor::regular::COMMAND,
        short: egui_phosphor::regular::COMMAND,
    };
    #[cfg(target_os = "windows")]
    pub const MOD_GUI: ModName = ModName {
        full: "Win",
        short: "Win",
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub const MOD_GUI: ModName = ModName {
        full: "Super",
        short: "Sup",
    };

    /// Chord separator: macOS packs glyphs tightly (⌃⇧⌥⌘); elsewhere "+" separates text names.
    #[cfg(target_os = "macos")]
    const MOD_SEP: &str = "";
    #[cfg(not(target_os = "macos"))]
    const MOD_SEP: &str = "+";

    /// True when `s` is a single Private-Use-Area glyph rather than a text name.
    fn is_glyph(s: &str) -> bool {
        let mut chars = s.chars();
        matches!(chars.next(), Some(c) if ('\u{E000}'..='\u{F8FF}').contains(&c))
            && chars.next().is_none()
    }

    /// Build a standalone modifier key: glyph modifiers go in `symbol`, text names in `tap`.
    pub fn modifier_key(m: &ModName) -> super::LayoutKey {
        if is_glyph(m.full) {
            super::LayoutKey {
                symbol: Some(m.full.to_string()),
                kind: super::KeycodeKind::Modifier,
                ..Default::default()
            }
        } else {
            super::LayoutKey {
                tap: super::Label::with_short(m.full, m.short),
                kind: super::KeycodeKind::Modifier,
                ..Default::default()
            }
        }
    }

    /// Combined label for a set of held modifiers (e.g. "Ctrl+⇧"), with a short form to shrink.
    pub fn glyphs(ctrl: bool, shift: bool, alt: bool, gui: bool) -> super::Label {
        let mut full: Vec<&str> = Vec::new();
        let mut short: Vec<&str> = Vec::new();
        let mut push = |m: &ModName| {
            full.push(m.full);
            short.push(m.short);
        };
        if ctrl {
            push(&MOD_CTRL);
        }
        if shift {
            push(&MOD_SHIFT);
        }
        if alt {
            push(&MOD_ALT);
        }
        if gui {
            push(&MOD_GUI);
        }
        super::Label::with_short(full.join(MOD_SEP), short.join(MOD_SEP))
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum KeycodeKind {
    #[default]
    Basic,
    Modifier,
    Special,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Label {
    /// Full label text (e.g., "Enter", "Shift")
    pub full: String,

    /// Optional shorter version (e.g., "Ent", "Shft")
    pub short: Option<String>,
}

impl Label {
    pub fn new(full: impl Into<String>) -> Self {
        Label {
            full: full.into(),
            short: None,
        }
    }

    pub fn with_short(full: impl Into<String>, short: impl Into<String>) -> Self {
        Label {
            full: full.into(),
            short: Some(short.into()),
        }
    }

    /// Returns a copy with `prefix` prepended to both the full and short forms,
    /// e.g. `Label{full:"Ctrl",short:"Ctl"}.prefixed("MT: ")` -> "MT: Ctrl"/"MT: Ctl".
    pub fn prefixed(&self, prefix: &str) -> Label {
        Label {
            full: format!("{prefix}{}", self.full),
            short: self.short.as_ref().map(|s| format!("{prefix}{s}")),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.full.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LayoutKey {
    /// Primary key action label (e.g., "A", "Enter", "L1")
    pub tap: Label,

    /// Secondary "function" legend describing the key's modifier, target layer,
    /// or behavior (e.g. "⎈" for MT, "L2" for LT, "MO"/"OSM"/"Toggle"). Rendered
    /// in a small strip along the bottom edge of the key.
    pub function: Option<Label>,

    /// Shifted character shown above `tap` (e.g. "!" for KC_1).
    pub shifted: Option<String>,

    /// Symbol/icon for the key (using Phosphor icon font)
    pub symbol: Option<String>,

    /// Visual classification for coloring
    pub kind: KeycodeKind,

    /// Layer this key activates (for MO, LT, TO, etc.) - used for coloring
    pub layer_ref: Option<u8>,
}

impl Default for LayoutKey {
    fn default() -> Self {
        LayoutKey {
            tap: Label::default(),
            function: None,
            shifted: None,
            symbol: None,
            kind: KeycodeKind::Basic,
            layer_ref: None,
        }
    }
}
