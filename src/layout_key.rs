/// Symbols for the four modifier keys. macOS uses native glyphs (⌃ ⇧ ⌥ ⌘);
/// Windows/Linux keep ⇧ for Shift but use shrinkable text names for the rest.
pub mod modifier_symbols {
    /// Full/short display names for a modifier (same glyph for both on glyph modifiers).
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
    /// `mod_mask` is `HELD_MOD_SHIFT`/`HELD_MOD_RALT`, or 0. See their doc comments.
    pub fn modifier_key(m: &ModName, mod_mask: u16) -> super::LayoutKey {
        super::LayoutKey {
            tap: super::Label::with_short(m.name, m.short),
            symbol: is_glyph(m.full).then(|| m.full.to_string()),
            kind: super::KeycodeKind::Modifier,
            mod_mask: (mod_mask != 0).then_some(mod_mask),
            ..Default::default()
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

/// Behavior display names for the top strip, as `(full, short)` pairs. Shared by
/// the ZMK and QMK label producers so both render the same wording.
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

    // Only behaviors that get a top-strip legend live here; pure layer-switch
    // behaviors are shown by their border alone and need no entry.
    behavior_name!(MOD_TAP, "Mod-Tap", "MT");
    behavior_name!(ONE_SHOT_MOD, "One-Shot Mod", "OSM");
    behavior_name!(STICKY_KEY, "Sticky Key", "SK");
    behavior_name!(KEY_TOGGLE, "Key Toggle", "KT");
    behavior_name!(TAP_DANCE, "Tap Dance", "TD");
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum KeycodeKind {
    #[default]
    Basic,
    Modifier,
    Special,
}

/// Outline style hinting *how* a layer activates: persistent changes get a solid
/// outline, sticky/one-shot get a striped one, momentary keeps the default border.
/// Behaviors that activate a layer the same way deliberately share a style.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum BorderStyle {
    /// Default thin border: plain keys, non-layer behaviors, and momentary/while-held
    /// layer keys (momentary / layer-tap / layer-mod / layer-tap-toggle).
    #[default]
    None,
    /// Solid, medium-width outline: the layer change persists after release
    /// (toggle / to-layer / default-layer).
    Solid,
    /// Striped outline: the layer stays active for one keypress, then reverts
    /// (one-shot / sticky layer).
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

/// `mod_mask` bit for a key that acts as a live Shift source while held (a
/// standalone Shift key, or the hold side of a Shift-carrying Mod-Tap/
/// One-Shot-Mod/Layer-Mod). The Single-legend live preview uses it to detect
/// "Shift is currently held", independent of the keyboard's protocol.
pub const HELD_MOD_SHIFT: u16 = 0x01;
/// Same as `HELD_MOD_SHIFT`, but for RAlt (the layout's Level-3 shift, on
/// layouts that define one).
pub const HELD_MOD_RALT: u16 = 0x02;

/// `mod_mask` contribution of a *plain* Alt key (`KC_LALT` / ZMK `LEFT_ALT`).
/// On macOS both left and right Alt are Option, a genuine text-producing
/// Level-3 shift (⌥G → ©), so plain Alt triggers the live RAlt preview too.
/// On Windows/Linux plain Alt is purely a shortcut modifier and contributes
/// nothing.
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

    /// Shifted character shown above `tap` (e.g. "!" for KC_1), and in
    /// Single-legend mode, what's shown instead of `tap` while Shift is held.
    pub shifted: Option<String>,

    /// RAlt-shifted character (e.g. "[" for a German RAlt+8), same role
    /// as `shifted` but for RAlt. Only ever set alongside `shifted` (both come
    /// from the same OS-layout resolution pass over a symbol/digit key).
    pub ralt: Option<String>,

    /// Character produced under Shift+RAlt (e.g. "ˇ" for a German
    /// RAlt+Shift+9), same role as `ralt` but with Shift additionally held.
    /// Only set alongside `ralt`, from the same resolution pass.
    pub ralt_shifted: Option<String>,

    /// `HELD_MOD_SHIFT`/`HELD_MOD_RALT` bits (OR'd) this key contributes
    /// while physically held. `None` for keys that are not a modifier
    /// source. Read by the Single-legend live preview, ignored otherwise.
    pub mod_mask: Option<u16>,

    /// Symbol/icon for the key (using Phosphor icon font)
    pub symbol: Option<String>,

    /// Visual classification for coloring
    pub kind: KeycodeKind,

    /// Layer this key activates (for MO, LT, TO, etc.) - used for coloring
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
