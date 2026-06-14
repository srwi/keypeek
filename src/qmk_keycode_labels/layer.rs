use crate::layout_key::{behavior_names, Label, LayoutKey};
use crate::qmk_keycode_labels::constants::*;

pub fn get_layer_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    let (behavior, center, layer_ref) = match keycode_bytes {
        b if QK_TO.contains(&b) => {
            let l = (b - QK_TO.start) as u8;
            (behavior_names::TO_LAYER.label(), format!("L{}", l), Some(l))
        }
        b if QK_MOMENTARY.contains(&b) => {
            let l = (b - QK_MOMENTARY.start) as u8;
            (
                behavior_names::MOMENTARY.label(),
                format!("L{}", l),
                Some(l),
            )
        }
        b if QK_TOGGLE_LAYER.contains(&b) => {
            let l = (b - QK_TOGGLE_LAYER.start) as u8;
            (behavior_names::TOGGLE.label(), format!("L{}", l), Some(l))
        }
        b if QK_ONE_SHOT_LAYER.contains(&b) => {
            let l = (b - QK_ONE_SHOT_LAYER.start) as u8;
            (
                behavior_names::ONE_SHOT_LAYER.label(),
                format!("L{}", l),
                Some(l),
            )
        }
        b if QK_LAYER_TAP_TOGGLE.contains(&b) => {
            let l = (b - QK_LAYER_TAP_TOGGLE.start) as u8;
            (
                behavior_names::LAYER_TAP_TOGGLE.label(),
                format!("L{}", l),
                Some(l),
            )
        }
        b if QK_DEF_LAYER.contains(&b) => {
            let l = (b - QK_DEF_LAYER.start) as u8;
            (
                behavior_names::DEFAULT_LAYER.label(),
                format!("L{}", l),
                None,
            )
        }
        b if QK_TAP_DANCE.contains(&b) => {
            let n = b - QK_TAP_DANCE.start;
            (behavior_names::TAP_DANCE.label(), n.to_string(), None)
        }
        b if QK_KB.contains(&b) => {
            let n = b - QK_KB.start;
            (behavior_names::CUSTOM.label(), n.to_string(), None)
        }
        b if QK_MACRO.contains(&b) => {
            let n = b - QK_MACRO.start;
            (behavior_names::MACRO.label(), n.to_string(), None)
        }
        _ => return None,
    };

    Some(LayoutKey {
        tap: Label::new(center),
        behavior: Some(behavior),
        layer_ref,
        ..Default::default()
    })
}
