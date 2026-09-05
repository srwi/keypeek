use crate::layout_key::{BorderStyle, KeycodeKind, Label, LayoutKey};

/// Pure layer-switch key: the target layer is the centered label and `border`
/// hints how the layer activates; there are no legend strips.
pub fn layer_switch_key(layer_id: u8, label: Label, border: BorderStyle) -> LayoutKey {
    LayoutKey {
        tap: label,
        kind: KeycodeKind::Modifier,
        layer_ref: Some(layer_id),
        border,
        ..Default::default()
    }
}

/// Layer-tap key: tapping produces `tap_key`, holding activates `layer_id`.
pub fn layer_tap_key(layer_id: u8, tap_key: LayoutKey, behavior: Option<Label>) -> LayoutKey {
    LayoutKey {
        tap: tap_key.tap,
        behavior,
        argument: tap_key.argument,
        shifted: tap_key.shifted,
        ralt: tap_key.ralt,
        ralt_shifted: tap_key.ralt_shifted,
        symbol: tap_key.symbol,
        kind: KeycodeKind::Modifier,
        layer_ref: Some(layer_id),
        border: BorderStyle::None,
        ..Default::default()
    }
}

/// Mod-tap / Hold-tap key: tapping produces `tap_key`, holding applies `hold_label`.
pub fn mod_tap_key(
    tap_key: LayoutKey,
    hold_label: Label,
    hold_mod_mask: Option<u16>,
    behavior: Option<Label>,
) -> LayoutKey {
    LayoutKey {
        tap: tap_key.tap,
        behavior,
        argument: Some(hold_label),
        shifted: tap_key.shifted,
        ralt: tap_key.ralt,
        ralt_shifted: tap_key.ralt_shifted,
        mod_mask: hold_mod_mask,
        symbol: tap_key.symbol,
        kind: KeycodeKind::Basic,
        layer_ref: None,
        border: BorderStyle::None,
    }
}

/// One-shot modifier key: shows modifier glyphs as `tap` with an OSM behavior badge.
pub fn one_shot_mod_key(
    mod_label: Label,
    mod_mask: Option<u16>,
    behavior: Option<Label>,
) -> LayoutKey {
    LayoutKey {
        tap: mod_label,
        behavior,
        mod_mask,
        kind: KeycodeKind::Modifier,
        ..Default::default()
    }
}

/// Normalized modifier flags across firmwares (QMK and ZMK).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub gui: bool,
    pub right_alt: bool,
}

impl Modifiers {
    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.shift && !self.alt && !self.gui
    }

    pub fn label(&self) -> Label {
        crate::layout_key::modifier_symbols::glyphs(self.ctrl, self.shift, self.alt, self.gui)
    }

    pub fn from_zmk_mask(mods: u8) -> Self {
        Self {
            ctrl: mods & (zmk_studio_api::MOD_LCTL | zmk_studio_api::MOD_RCTL) != 0,
            shift: mods & (zmk_studio_api::MOD_LSFT | zmk_studio_api::MOD_RSFT) != 0,
            alt: mods & (zmk_studio_api::MOD_LALT | zmk_studio_api::MOD_RALT) != 0,
            gui: mods & (zmk_studio_api::MOD_LGUI | zmk_studio_api::MOD_RGUI) != 0,
            right_alt: mods & zmk_studio_api::MOD_RALT != 0,
        }
    }
}

impl From<qmk_via_api::QmkModMask> for Modifiers {
    fn from(mods: qmk_via_api::QmkModMask) -> Self {
        Self {
            ctrl: mods.has_ctrl(),
            shift: mods.has_shift(),
            alt: mods.has_alt(),
            gui: mods.has_gui(),
            right_alt: mods.has_alt() && mods.is_right(),
        }
    }
}

/// Resolves a key pressed with modifiers.
///
/// If the modifiers are text-producing (Shift, RAlt / Level 3, Shift+RAlt,
/// or macOS Option) on Keyboard page 0x07, this resolves to the resulting flat
/// character without badges. Otherwise, it renders the base key with modifier glyphs.
pub fn mod_combo_key(
    page: u16,
    usage_id: u16,
    mods: Modifiers,
    base_key: Option<LayoutKey>,
) -> LayoutKey {
    let base_key = base_key.or_else(|| crate::hid_labels::hid_usage_to_layout_key(page, usage_id));

    if page == 0x07 {
        let alt_is_level3 = cfg!(target_os = "macos") || mods.right_alt;
        let text_modifier = if !mods.ctrl && !mods.gui {
            if mods.shift && !mods.alt {
                Some(crate::os_layout::Modifier::Shift)
            } else if mods.alt && !mods.shift && alt_is_level3 {
                Some(crate::os_layout::Modifier::RAlt)
            } else if mods.shift && mods.alt && alt_is_level3 {
                Some(crate::os_layout::Modifier::ShiftRAlt)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(m) = text_modifier {
            if let Some(text) = crate::os_layout::resolve(usage_id, m) {
                return LayoutKey {
                    tap: Label::new(text),
                    ..Default::default()
                };
            }

            // Fallback when active OS layout resolution is unavailable:
            let fallback = match m {
                crate::os_layout::Modifier::Shift => {
                    base_key.as_ref().and_then(|k| k.shifted.clone())
                }
                crate::os_layout::Modifier::RAlt => base_key.as_ref().and_then(|k| k.ralt.clone()),
                crate::os_layout::Modifier::ShiftRAlt => {
                    base_key.as_ref().and_then(|k| k.ralt_shifted.clone())
                }
                _ => None,
            };

            if let Some(text) = fallback {
                return LayoutKey {
                    tap: Label::new(text),
                    ..Default::default()
                };
            }
        }
    }

    let (tap, symbol) = match base_key {
        Some(k) => (k.tap, k.symbol),
        None => (Label::new(format!("0x{:02X}", usage_id)), None),
    };

    LayoutKey {
        tap,
        argument: Some(mods.label()),
        symbol,
        kind: KeycodeKind::Modifier,
        ..Default::default()
    }
}
