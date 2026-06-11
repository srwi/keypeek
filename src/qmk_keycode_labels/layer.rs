use crate::layout_key::{Label, LayoutKey};
use crate::qmk_keycode_labels::constants::*;

pub fn get_layer_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    // Layer/behavior keys put the target layer in `tap` and the behavior
    // abbreviation in `function`, mirroring the tap/function split used for MT/LT.
    let (tap_label, function_label, layer_ref) = match keycode_bytes {
        b if QK_TO.contains(&b) => {
            let l = (b - QK_TO.start) as u8;
            (format!("L{}", l), Some("TO"), Some(l))
        }
        b if QK_MOMENTARY.contains(&b) => {
            let l = (b - QK_MOMENTARY.start) as u8;
            (format!("L{}", l), Some("MO"), Some(l))
        }
        b if QK_TOGGLE_LAYER.contains(&b) => {
            let l = (b - QK_TOGGLE_LAYER.start) as u8;
            (format!("L{}", l), Some("TG"), Some(l))
        }
        b if QK_ONE_SHOT_LAYER.contains(&b) => {
            let l = (b - QK_ONE_SHOT_LAYER.start) as u8;
            (format!("L{}", l), Some("OSL"), Some(l))
        }
        b if QK_LAYER_TAP_TOGGLE.contains(&b) => {
            let l = (b - QK_LAYER_TAP_TOGGLE.start) as u8;
            (format!("L{}", l), Some("TT"), Some(l))
        }
        b if QK_DEF_LAYER.contains(&b) => {
            let l = (b - QK_DEF_LAYER.start) as u8;
            (format!("L{}", l), Some("DF"), None)
        }
        b if QK_TAP_DANCE.contains(&b) => {
            let n = b - QK_TAP_DANCE.start;
            (format!("TD({})", n), None, None)
        }
        b if QK_KB.contains(&b) => {
            let n = b - QK_KB.start;
            (format!("CUSTOM({})", n), None, None)
        }
        b if QK_MACRO.contains(&b) => {
            let n = b - QK_MACRO.start;
            (format!("MACRO({})", n), None, None)
        }
        _ => return None,
    };

    Some(LayoutKey {
        tap: Label::new(tap_label),
        function: function_label.map(Label::new),
        layer_ref,
        ..Default::default()
    })
}
