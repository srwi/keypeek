use crate::layout_key::{behavior_names, BorderStyle, Label, LayoutKey};
use qmk_via_api::{QmkKeycode, QmkLayerOp};

pub fn get_layer_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    // Layer-switch keys are shown by their border alone (Solid = persists, Dashed =
    // sticky/one-shot, None = momentary) and carry no legend strip. Tap dance keeps a
    // name strip and default border.
    let (behavior, tap, layer_ref, border) = match QmkKeycode::from_u16(keycode_bytes) {
        QmkKeycode::LayerOp { op, layer } => {
            let (border, layer_ref) = match op {
                QmkLayerOp::To | QmkLayerOp::Toggle => (BorderStyle::Solid, Some(layer)),
                QmkLayerOp::Momentary | QmkLayerOp::TapToggle => (BorderStyle::None, Some(layer)),
                QmkLayerOp::OneShot => (BorderStyle::Dashed, Some(layer)),
                QmkLayerOp::Default => (BorderStyle::Solid, None),
            };
            (None, layer_label(layer), layer_ref, border)
        }
        QmkKeycode::TapDance(n) => (
            Some(behavior_names::TAP_DANCE.label()),
            Label::new(n.to_string()),
            None,
            BorderStyle::None,
        ),
        QmkKeycode::Macro(n) => (
            None,
            numbered_label("Macro", "M", n as u16),
            None,
            BorderStyle::None,
        ),
        QmkKeycode::CustomKb(n) => (
            None,
            numbered_label("KB", "KB", n as u16),
            None,
            BorderStyle::None,
        ),
        QmkKeycode::CustomUser(n) => (
            None,
            numbered_label("User", "Usr", n),
            None,
            BorderStyle::None,
        ),
        _ => return None,
    };

    Some(LayoutKey {
        tap,
        behavior,
        layer_ref,
        border,
        ..Default::default()
    })
}

fn layer_label(layer: u8) -> Label {
    Label::new(format!("L{layer}"))
}

fn numbered_label(full_prefix: &str, short_prefix: &str, index: u16) -> Label {
    Label::with_short(
        format!("{full_prefix} {index}"),
        format!("{short_prefix}{index}"),
    )
}
