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
