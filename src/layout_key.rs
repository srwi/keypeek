/// Symbols used to render the four modifier keys across all protocols.
///
/// Each modifier has a `full` and a `short` display name. On macOS all four are
/// native Phosphor glyphs (⌃ ⇧ ⌥ ⌘), where `full` and `short` are the same
/// glyph. On Windows/Linux, Shift keeps the ⇧ glyph (it is universally
/// understood), while Ctrl, Alt and GUI use text names that can shrink when
/// space is tight ("Ctrl"→"Ctl", "Super"→"Sup").
pub mod modifier_symbols {
    /// `full`/`short` display names for one modifier. For glyph modifiers both
    /// fields hold the same icon glyph.
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

    /// Separator for the combined chord string from `glyphs()`. macOS packs the
    /// glyphs tightly (⌃⇧⌥⌘, the native convention); elsewhere a "+" keeps the
    /// text labels from running together ("Ctrl+Alt" rather than "CtrlAlt").
    #[cfg(target_os = "macos")]
    const MOD_SEP: &str = "";
    #[cfg(not(target_os = "macos"))]
    const MOD_SEP: &str = "+";

    /// True when `s` is a single Phosphor icon glyph (Unicode Private Use Area)
    /// rather than a plain text name like "Ctrl".
    fn is_glyph(s: &str) -> bool {
        let mut chars = s.chars();
        matches!(chars.next(), Some(c) if ('\u{E000}'..='\u{F8FF}').contains(&c))
            && chars.next().is_none()
    }

    /// Build a standalone modifier key. Glyph modifiers (macOS, plus Shift on
    /// every platform) go in `symbol`; text names ("Ctrl"/"Alt"/"Win"/"Super" on
    /// Windows/Linux) go in `tap` (with their short form) and leave `symbol` empty.
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

    /// Combined label for a set of held modifiers, used in the small function
    /// legend of modifier-tap / modified keys (e.g. "Ctrl+⇧" for LCTL(LSFT(..))).
    /// Carries both a full and a short joined form so it can shrink to fit.
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

/// Display names for the tap/hold and layer behaviors, shown in the top strip of
/// a key. Each entry is `(full, short)`; the short form is used by the auto-fit
/// logic when the key is too small for the full word. Shared by the ZMK and QMK
/// label producers so both protocols render the same wording.
pub mod behavior_names {
    use super::Label;

    /// `(full, short)` behavior label pair.
    pub struct BehaviorName {
        pub full: &'static str,
        pub short: &'static str,
    }

    impl BehaviorName {
        /// Build a `Label` carrying both the full and short forms.
        pub fn label(&self) -> Label {
            Label::with_short(self.full, self.short)
        }
    }

    macro_rules! behavior_name {
        ($name:ident, $full:expr, $short:expr) => {
            pub const $name: BehaviorName = BehaviorName {
                full: $full,
                short: $short,
            };
        };
    }

    behavior_name!(MOD_TAP, "Mod-Tap", "MT");
    behavior_name!(LAYER_TAP, "Layer-Tap", "LT");
    behavior_name!(LAYER_MOD, "Layer-Mod", "LM");
    behavior_name!(MOMENTARY, "Momentary", "MO");
    behavior_name!(TOGGLE, "Toggle", "TG");
    behavior_name!(TO_LAYER, "To Layer", "TO");
    behavior_name!(DEFAULT_LAYER, "Default", "DF");
    behavior_name!(ONE_SHOT_LAYER, "One-Shot Layer", "OSL");
    behavior_name!(ONE_SHOT_MOD, "One-Shot Mod", "OSM");
    behavior_name!(LAYER_TAP_TOGGLE, "Layer Tap-Toggle", "TT");
    behavior_name!(STICKY_LAYER, "Sticky Layer", "SL");
    behavior_name!(STICKY_KEY, "Sticky Key", "SK");
    behavior_name!(KEY_TOGGLE, "Key Toggle", "KT");
    behavior_name!(TAP_DANCE, "Tap Dance", "TD");
    behavior_name!(MACRO, "Macro", "MACRO");
    behavior_name!(CUSTOM, "Custom", "CUSTOM");
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

    pub fn is_empty(&self) -> bool {
        self.full.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LayoutKey {
    /// Primary key action label (e.g., "A", "Enter", "L1")
    pub tap: Label,

    /// Behavior name shown in a small strip along the top edge of the key
    /// (e.g. "Mod-Tap", "Momentary", "Layer-Tap"). `None` for plain keys.
    pub behavior: Option<Label>,

    /// The behavior's argument, shown in a small strip along the bottom edge of
    /// the key (e.g. the held modifier "Ctrl" for Mod-Tap, the target layer "L2" for
    /// Layer-Tap). `None` when the behavior takes no separate argument.
    pub argument: Option<Label>,

    /// Shifted legend for keys that produce a different character when shift is
    /// held (e.g. "!" for KC_1). Rendered as a second line above `tap`.
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
            behavior: None,
            argument: None,
            shifted: None,
            symbol: None,
            kind: KeycodeKind::Basic,
            layer_ref: None,
        }
    }
}
