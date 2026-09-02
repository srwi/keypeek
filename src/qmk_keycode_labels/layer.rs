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
            Some(behavior_names::MACRO.label()),
            Label::new(n.to_string()),
            None,
            BorderStyle::None,
        ),
        QmkKeycode::CustomKb(n) => (
            Some(behavior_names::CUSTOM_KB.label()),
            Label::new(n.to_string()),
            None,
            BorderStyle::None,
        ),
        QmkKeycode::CustomUser(n) => (
            Some(behavior_names::CUSTOM_USER.label()),
            Label::new(n.to_string()),
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

#[cfg(test)]
mod tests {
    use crate::qmk_keycode_labels::resolve_qmk_key;
    use crate::qmk_keycode_labels::KeyResolution;
    use qmk_via_api::ranges::{QK_KB, QK_MACRO, QK_TAP_DANCE, QK_USER};

    #[test]
    fn custom_keys_render_top_strip_behavior_and_numeric_tap() {
        let cases = [
            (QK_TAP_DANCE.start + 5, "TD", "5"),
            (QK_MACRO.start + 3, "M", "3"),
            (QK_KB.start + 7, "KB", "7"),
            (QK_USER.start + 12, "Usr", "12"),
        ];

        for (code, expected_behavior_short, expected_tap) in cases {
            let resolution = resolve_qmk_key(code);
            match resolution {
                KeyResolution::Key(key) => {
                    assert_eq!(key.tap.full, expected_tap, "tap label for 0x{code:04X}");
                    assert_eq!(
                        key.behavior.as_ref().and_then(|b| b.short.as_deref()),
                        Some(expected_behavior_short),
                        "behavior short for 0x{code:04X}"
                    );
                }
                _ => panic!("0x{code:04X} should resolve to Key"),
            }
        }
    }
}
