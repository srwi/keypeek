//! Candidate ZMK keycodes and behaviors for picker grids.

use super::picker::{Candidate, CandidateGroup};
use crate::key_spec::LayerInfo;
use std::sync::OnceLock;
use zmk_studio_api::{Behavior, HidUsage, Keycode};

pub use zmk_studio_api::BehaviorRole as ZmkBehaviorKind;

/// Returns a sample Behavior instance for a behavior role.
pub fn sample_behavior(role: ZmkBehaviorKind) -> Behavior {
    role.sample_behavior()
}

/// Returns candidate groups for ZMK layer behaviors.
pub fn layer_groups(
    kinds: &[ZmkBehaviorKind],
    layer_infos: &[LayerInfo],
    layer_names: &[String],
    tap: HidUsage,
) -> Vec<CandidateGroup> {
    kinds
        .iter()
        .map(|kind| CandidateGroup {
            name: kind.label(),
            candidates: layer_infos
                .iter()
                .map(|info| {
                    let behavior = match kind {
                        ZmkBehaviorKind::MomentaryLayer => {
                            Behavior::MomentaryLayer { layer_id: info.id }
                        }
                        ZmkBehaviorKind::ToggleLayer => Behavior::ToggleLayer { layer_id: info.id },
                        ZmkBehaviorKind::ToLayer => Behavior::ToLayer { layer_id: info.id },
                        ZmkBehaviorKind::StickyLayer => Behavior::StickyLayer { layer_id: info.id },
                        ZmkBehaviorKind::LayerTap => Behavior::LayerTap {
                            layer_id: info.id,
                            tap,
                        },
                        _ => unreachable!("layer page kinds only"),
                    };
                    behavior_candidate(&behavior, layer_names)
                })
                .collect(),
        })
        .collect()
}

/// Returns candidate keys for a ZMK command behavior.
pub fn command_candidates(kind: ZmkBehaviorKind, backlight_level: u8) -> Vec<Candidate> {
    let mut behaviors = kind.standard_candidates();
    for behavior in &mut behaviors {
        if let Behavior::Backlight(zmk_studio_api::BacklightCommand::Set(ref mut level)) = behavior
        {
            *level = backlight_level;
        }
    }
    behaviors
        .into_iter()
        .map(|behavior| behavior_candidate(&behavior, &[]))
        .collect()
}

/// Creates a candidate definition for a ZMK behavior.
pub fn behavior_candidate(behavior: &Behavior, layer_names: &[String]) -> Candidate {
    let mut candidate = Candidate::from_action(
        crate::protocols::zmk_codec::zmk_to_keyspec(behavior),
        layer_names,
    );
    match behavior {
        Behavior::KeyPress(usage) | Behavior::KeyToggle(usage) | Behavior::StickyKey(usage) => {
            use std::fmt::Write;
            let mut hex8 = String::with_capacity(9);
            let _ = write!(&mut hex8, "{:08x}", usage.to_hid_usage());
            let mut hex4 = String::with_capacity(5);
            let _ = write!(&mut hex4, "{:04x}", usage.id());
            candidate = candidate.with_search_token(hex8).with_search_token(hex4);
            if let Ok(kc) = Keycode::try_from(usage.to_hid_usage()) {
                candidate = candidate
                    .with_search_token(kc.as_ref())
                    .with_search_token(kc.to_name());
            }
        }
        _ => {}
    }
    candidate
}

pub fn categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(build_categories)
}

fn build_categories() -> Vec<CandidateGroup> {
    vec![
        CandidateGroup {
            name: "Keyboard",
            candidates: Keycode::all_keyboard()
                .iter()
                .map(|&k| keycode_candidate(k.to_hid_usage()))
                .collect(),
        },
        CandidateGroup {
            name: "Consumer",
            candidates: Keycode::all_consumer()
                .iter()
                .map(|&k| keycode_candidate(k.to_hid_usage()))
                .collect(),
        },
    ]
}

/// Creates a candidate definition for a ZMK HID usage code.
pub fn keycode_candidate(encoded: u32) -> Candidate {
    use std::fmt::Write;
    let usage = HidUsage::from_encoded(encoded);
    let action = crate::protocols::zmk_codec::zmk_to_keyspec(&Behavior::KeyPress(usage));
    let mut candidate = Candidate::from_action(action, &[]);
    if candidate.key.symbol.is_none() && candidate.key.tap.is_empty() {
        candidate.key.symbol = Some(format!("0x{:02X}", usage.id()));
    }
    let mut hex8 = String::with_capacity(9);
    let _ = write!(&mut hex8, "{:08x}", encoded);
    let mut hex4 = String::with_capacity(5);
    let _ = write!(&mut hex4, "{:04x}", usage.id());
    candidate = candidate.with_search_token(hex8).with_search_token(hex4);
    if let Ok(kc) = Keycode::try_from(encoded) {
        candidate = candidate
            .with_search_token(kc.as_ref())
            .with_search_token(kc.to_name());
    }
    candidate
}
