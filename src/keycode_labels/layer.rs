use crate::keycode_labels::constants::*;
use crate::layout_key::{Label, LayoutKey};

pub fn get_layer_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    let (tap_label, layer_ref) = match keycode_bytes {
        b if QK_TO.contains(&b) => {
            let l = (b - QK_TO.start) as u8;
            (format!("TO({})", l), Some(l))
        }
        b if QK_MOMENTARY.contains(&b) => {
            let l = (b - QK_MOMENTARY.start) as u8;
            (format!("MO({})", l), Some(l))
        }
        b if QK_TOGGLE_LAYER.contains(&b) => {
            let l = (b - QK_TOGGLE_LAYER.start) as u8;
            (format!("TG({})", l), Some(l))
        }
        b if QK_ONE_SHOT_LAYER.contains(&b) => {
            let l = (b - QK_ONE_SHOT_LAYER.start) as u8;
            (format!("OSL({})", l), Some(l))
        }
        b if QK_LAYER_TAP_TOGGLE.contains(&b) => {
            let l = (b - QK_LAYER_TAP_TOGGLE.start) as u8;
            (format!("TT({})", l), Some(l))
        }
        b if QK_DEF_LAYER.contains(&b) => {
            let l = (b - QK_DEF_LAYER.start) as u8;
            (format!("DF({})", l), None)
        }
        b if QK_KB.contains(&b) => {
            let n = b - QK_KB.start;
            (format!("CUSTOM({})", n), None)
        }
        b if QK_MACRO.contains(&b) => {
            let n = b - QK_MACRO.start;
            (format!("MACRO({})", n), None)
        }
        _ => return None,
    };

    Some(LayoutKey {
        tap: Label::new(tap_label),
        layer_ref,
        ..Default::default()
    })
}
