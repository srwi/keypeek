use crate::layout_key::{behavior_names, BorderStyle, Label, LayoutKey};
use crate::qmk_keycode_labels::constants::*;

pub fn get_layer_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    // Layer-switch keys are shown by their border alone (Solid = persists, Dashed =
    // sticky/one-shot, None = momentary) and carry no legend strip. Tap dance keeps a
    // name strip and default border.
    let (behavior, tap, layer_ref, border) = match keycode_bytes {
        b if QK_TO.contains(&b) => {
            let l = (b - QK_TO.start) as u8;
            (None, layer_label(l), Some(l), BorderStyle::Solid)
        }
        b if QK_MOMENTARY.contains(&b) => {
            let l = (b - QK_MOMENTARY.start) as u8;
            (None, layer_label(l), Some(l), BorderStyle::None)
        }
        b if QK_TOGGLE_LAYER.contains(&b) => {
            let l = (b - QK_TOGGLE_LAYER.start) as u8;
            (None, layer_label(l), Some(l), BorderStyle::Solid)
        }
        b if QK_ONE_SHOT_LAYER.contains(&b) => {
            let l = (b - QK_ONE_SHOT_LAYER.start) as u8;
            (None, layer_label(l), Some(l), BorderStyle::Dashed)
        }
        b if QK_LAYER_TAP_TOGGLE.contains(&b) => {
            let l = (b - QK_LAYER_TAP_TOGGLE.start) as u8;
            (None, layer_label(l), Some(l), BorderStyle::None)
        }
        b if QK_DEF_LAYER.contains(&b) => {
            let l = (b - QK_DEF_LAYER.start) as u8;
            (None, layer_label(l), None, BorderStyle::Solid)
        }
        b if QK_TAP_DANCE.contains(&b) => {
            let n = b - QK_TAP_DANCE.start;
            (
                Some(behavior_names::TAP_DANCE.label()),
                Label::new(n.to_string()),
                None,
                BorderStyle::None,
            )
        }
        b if QK_MACRO.contains(&b) => (
            None,
            numbered_label("Macro", "M", b - QK_MACRO.start),
            None,
            BorderStyle::None,
        ),
        b if QK_KB.contains(&b) => (
            None,
            numbered_label("KB", "KB", b - QK_KB.start),
            None,
            BorderStyle::None,
        ),
        b if QK_USER.contains(&b) => (
            None,
            numbered_label("User", "Usr", b - QK_USER.start),
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
