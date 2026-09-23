/// Symbols for the four modifier keys. macOS uses native glyphs;
/// Windows/Linux keep arrow glyph for Shift but use text names for the rest.
pub mod modifier_symbols {
    /// Full and short display names for a modifier.
    pub struct ModName {
        pub full: &'static str,
        pub short: &'static str,
        pub name: &'static str,
    }

    #[cfg(target_os = "macos")]
    pub const MOD_CTRL: ModName = ModName {
        full: egui_phosphor::regular::CONTROL,
        short: egui_phosphor::regular::CONTROL,
        name: "Control",
    };
    #[cfg(not(target_os = "macos"))]
    pub const MOD_CTRL: ModName = ModName {
        full: "Ctrl",
        short: "Ctl",
        name: "Control",
    };

    pub const MOD_SHIFT: ModName = ModName {
        full: egui_phosphor::regular::ARROW_FAT_UP,
        short: egui_phosphor::regular::ARROW_FAT_UP,
        name: "Shift",
    };

    #[cfg(target_os = "macos")]
    pub const MOD_ALT: ModName = ModName {
        full: egui_phosphor::regular::OPTION,
        short: egui_phosphor::regular::OPTION,
        name: "Option",
    };
    #[cfg(not(target_os = "macos"))]
    pub const MOD_ALT: ModName = ModName {
        full: "Alt",
        short: "Alt",
        name: "Alt",
    };

    #[cfg(target_os = "macos")]
    pub const MOD_GUI: ModName = ModName {
        full: egui_phosphor::regular::COMMAND,
        short: egui_phosphor::regular::COMMAND,
        name: "Command",
    };
    #[cfg(target_os = "windows")]
    pub const MOD_GUI: ModName = ModName {
        full: "Win",
        short: "Win",
        name: "Windows",
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub const MOD_GUI: ModName = ModName {
        full: "Super",
        short: "Sup",
        name: "Super",
    };

    /// Chord separator: macOS packs glyphs together; other platforms use "+" between names.
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

    /// Builds a modifier key definition.
    pub fn modifier_key(m: &ModName, mod_mask: u16) -> super::LayoutKey {
        let is_sym = is_glyph(m.full);
        super::LayoutKey {
            tap: if is_sym {
                super::Label::new(m.name)
            } else {
                super::Label::with_short(m.name, m.short)
            },
            symbol: is_sym.then(|| m.full.to_string()),
            kind: super::KeycodeKind::Modifier,
            mod_mask: (mod_mask != 0).then_some(mod_mask),
            ..Default::default()
        }
    }

    /// Combined label for a set of held modifiers, with a short form.
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

/// Behavior display names for the top strip as (full, short) pairs.
pub mod behavior_names {
    use super::Label;

    pub struct BehaviorName {
        pub full: &'static str,
        pub short: &'static str,
    }

    impl BehaviorName {
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
    behavior_name!(ONE_SHOT_MOD, "One-Shot Mod", "OSM");
    behavior_name!(STICKY_KEY, "Sticky Key", "SK");
    behavior_name!(KEY_TOGGLE, "Key Toggle", "KT");
    behavior_name!(TAP_DANCE, "Tap Dance", "TD");
    behavior_name!(MACRO, "Macro", "M");
    behavior_name!(CUSTOM_KB, "Keyboard", "KB");
    behavior_name!(CUSTOM_USER, "User", "Usr");
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum KeycodeKind {
    #[default]
    Basic,
    Modifier,
    Special,
}

/// Outline style that shows how a layer activates.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum BorderStyle {
    /// Default border for regular keys and momentary layer keys.
    #[default]
    None,
    /// Solid outline for persistent layer changes (toggle, to-layer, default-layer).
    Solid,
    /// Dashed outline for temporary layer changes (one-shot or sticky layer).
    Dashed,
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

/// Bit flag for keys that activate Shift while held.
pub const HELD_MOD_SHIFT: u16 = 0x01;
/// Bit flag for keys that activate RAlt while held.
pub const HELD_MOD_RALT: u16 = 0x02;

/// `mod_mask` contribution of a plain Alt key (`KC_LALT` / ZMK `LEFT_ALT`).
/// On macOS both Alt keys act as Option (level-3 shift), so plain Alt triggers
/// the live RAlt preview. On other platforms plain Alt contributes nothing.
#[cfg(target_os = "macos")]
pub const PLAIN_ALT_MOD_MASK: u16 = HELD_MOD_RALT;
#[cfg(not(target_os = "macos"))]
pub const PLAIN_ALT_MOD_MASK: u16 = 0;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LayoutKey {
    /// Primary key action label (e.g., "A", "Enter", "L1")
    pub tap: Label,

    /// Behavior name shown in the top strip (e.g. "Mod-Tap"). `None` for plain keys.
    pub behavior: Option<Label>,

    /// Behavior argument shown in the bottom strip (e.g. "Ctrl" for Mod-Tap, "L2"
    /// for Layer-Tap). `None` when there is no argument.
    pub argument: Option<Label>,

    /// Shifted character shown above `tap`, or in place of `tap` while Shift is held.
    pub shifted: Option<String>,

    /// RAlt-shifted character, shown while RAlt is held.
    pub ralt: Option<String>,

    /// Character produced when both Shift and RAlt are held.
    pub ralt_shifted: Option<String>,

    /// Modifier mask bit flags contributed while key is held. None for non-modifier keys.
    pub mod_mask: Option<u16>,

    /// Symbol/icon for the key (using Phosphor icon font)
    pub symbol: Option<String>,

    /// Visual classification for coloring
    pub kind: KeycodeKind,

    /// Layer this key activates (used for coloring).
    pub layer_ref: Option<u8>,

    /// Outline style hinting how this key activates a layer. `None` for plain keys.
    pub border: BorderStyle,
}

impl LayoutKey {
    /// Full long name for tooltips and descriptions.
    pub fn tooltip_text(&self) -> Option<String> {
        if self.tap.is_empty() {
            return None;
        }

        let full_text = match (&self.behavior, &self.argument) {
            (Some(behavior), Some(arg)) => {
                format!("{}: {} ({})", behavior.full, self.tap.full, arg.full)
            }
            (Some(behavior), None) => {
                format!("{}: {}", behavior.full, self.tap.full)
            }
            (None, Some(arg)) => {
                format!("{} ({})", self.tap.full, arg.full)
            }
            (None, None) => self.tap.full.clone(),
        };

        Some(full_text)
    }
}

impl Default for LayoutKey {
    fn default() -> Self {
        LayoutKey {
            tap: Label::default(),
            behavior: None,
            argument: None,
            shifted: None,
            ralt: None,
            ralt_shifted: None,
            mod_mask: None,
            symbol: None,
            kind: KeycodeKind::Basic,
            layer_ref: None,
            border: BorderStyle::None,
        }
    }
}
