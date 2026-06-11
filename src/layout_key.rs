/// Symbols used to render the four modifier keys across all protocols.
///
/// Three are Phosphor glyphs (guaranteed present by the loaded Phosphor fonts);
/// `MOD_SYMBOL_CTRL` is a raw Unicode codepoint (⎈ U+2388) because Phosphor has no
/// control symbol.
pub mod modifier_symbols {
    pub const MOD_SYMBOL_CTRL: &str = "\u{2388}";
    pub const MOD_SYMBOL_SHIFT: &str = egui_phosphor::regular::ARROW_FAT_UP;
    pub const MOD_SYMBOL_ALT: &str = egui_phosphor::regular::OPTION;
    pub const MOD_SYMBOL_GUI: &str = egui_phosphor::fill::DIAMOND;
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

    /// Hold action label for hold-tap keys (e.g., "Shift" for MT(LSFT, KC_A))
    pub hold: Option<Label>,

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
            hold: None,
            symbol: None,
            kind: KeycodeKind::Basic,
            layer_ref: None,
        }
    }
}
