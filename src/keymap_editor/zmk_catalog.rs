//! Candidate ZMK keycodes for the editor's picker grids, split into the
//! keyboard (usage page 0x07) and consumer (page 0x0C) pages. The candidate
//! lists and their button text are computed once and cached, since enumerating
//! every encoded usage is expensive to do per frame. Also builds the
//! behavior candidates for the parameterless and command keys, which are
//! applied directly on click.

use crate::key_action::{KeyAction, LayerInfo};
use crate::zmk_keycode_labels::behavior_to_layout_key;
use std::sync::OnceLock;
use zmk_studio_api::{Behavior, HidUsage, Keycode};
pub const HID_USAGE_KEYBOARD: u16 = 0x07;
pub const HID_USAGE_CONSUMER: u16 = 0x0C;

use super::picker::{Candidate, CandidateGroup};
use super::zmk_editor::ZmkBehaviorKind;

/// The parameterless behaviors, applied directly on click. The grid replaces
/// the old per-kind panes that held nothing but an Apply button.
pub fn special_candidates() -> &'static [Candidate] {
    static SPECIALS: OnceLock<Vec<Candidate>> = OnceLock::new();
    SPECIALS.get_or_init(|| {
        let behaviors = [
            Behavior::Transparent,
            Behavior::None,
            Behavior::CapsWord,
            Behavior::KeyRepeat,
            Behavior::GraveEscape,
            Behavior::StudioUnlock,
            Behavior::Reset,
            Behavior::Bootloader,
            Behavior::SoftOff,
        ];
        behaviors
            .into_iter()
            .enumerate()
            .map(|(i, behavior)| behavior_candidate(i as u32, &behavior, &[]))
            .collect()
    })
}

/// Group order for the layer page: `layer_groups` builds one group per kind
/// in this order, and the editor maps a clicked group index back to its kind.
pub(super) const LAYER_KINDS: [ZmkBehaviorKind; 5] = [
    ZmkBehaviorKind::MomentaryLayer,
    ZmkBehaviorKind::ToggleLayer,
    ZmkBehaviorKind::ToLayer,
    ZmkBehaviorKind::StickyLayer,
    ZmkBehaviorKind::LayerTap,
];

/// The layer page's groups: one candidate per layer for each layer behavior,
/// rendered like the overlay paints the binding (layer name as the legend, the
/// layer's own color). The Layer-Tap group's tap side comes from `tap`.
pub fn layer_groups(
    layer_infos: &[LayerInfo],
    layer_names: &[String],
    tap: HidUsage,
) -> Vec<CandidateGroup> {
    LAYER_KINDS
        .iter()
        .map(|kind| CandidateGroup {
            name: kind.label(),
            candidates: layer_infos
                .iter()
                .enumerate()
                .map(|(i, info)| {
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
                    behavior_candidate(i as u32, &behavior, layer_names)
                })
                .collect(),
        })
        .collect()
}

/// Candidates for one command behavior kind: every option is rendered as a key
/// (its overlay legend comes from the behavior resolver) and applied directly
/// on click. `backlight_level` feeds the parametric Backlight `Set` command.
pub fn command_candidates(kind: ZmkBehaviorKind, backlight_level: u32) -> Vec<Candidate> {
    use ZmkBehaviorKind::*;
    let behaviors: Vec<Behavior> = match kind {
        Bluetooth => {
            // Fixed commands, then Select/Disconnect expanded per profile —
            // the profile is part of the binding, so each is its own key.
            let mut list: Vec<Behavior> = (0..=2)
                .map(|command| Behavior::Bluetooth { command, value: 0 })
                .collect();
            list.extend((0..=9).map(|value| Behavior::Bluetooth { command: 3, value }));
            list.push(Behavior::Bluetooth {
                command: 4,
                value: 0,
            });
            list.extend((0..=9).map(|value| Behavior::Bluetooth { command: 5, value }));
            list
        }
        OutputSelection => (0..=3)
            .map(|value| Behavior::OutputSelection { value })
            .collect(),
        Backlight => {
            let mut list: Vec<Behavior> = (0..=5)
                .map(|command| Behavior::Backlight { command, value: 0 })
                .collect();
            list.push(Behavior::Backlight {
                command: 6,
                value: backlight_level,
            });
            list
        }
        Underglow => (0..=14)
            .map(|command| Behavior::Underglow { command, value: 0 })
            .collect(),
        MouseKeyPress => [1, 2, 4, 8, 16]
            .into_iter()
            .map(|value| Behavior::MouseKeyPress { value })
            .collect(),
        // ZMK pointing values: `(x << 16) | (y & 0xFFFF)`.
        MouseMove => [0x0001_0000, 0xFFFF_FFFF, 0x0000_0001, 0x0000_FFFF]
            .into_iter()
            .map(|value| Behavior::MouseMove { value })
            .collect(),
        MouseScroll => [0x0000_0001, 0x0000_FFFF, 0x0001_0000, 0xFFFF_FFFF]
            .into_iter()
            .map(|value| Behavior::MouseScroll { value })
            .collect(),
        _ => Vec::new(),
    };
    behaviors
        .into_iter()
        .enumerate()
        .map(|(i, behavior)| behavior_candidate(i as u32, &behavior, &[]))
        .collect()
}

/// The candidate for a ZMK behavior: its fully resolved `LayoutKey`, exactly as
/// the overlay paints the binding. `Transparent` has no key of its own — it
/// falls through — so it renders as a ghosted empty slot. `layer_names`
/// resolves layer references for the legends.
fn behavior_candidate(code: u32, behavior: &Behavior, layer_names: &[String]) -> Candidate {
    let key = behavior_to_layout_key(behavior, layer_names).unwrap_or_default();
    Candidate {
        code,
        key,
        transparent: *behavior == Behavior::Transparent,
        behavior: Some(behavior.clone()),
    }
}

pub fn categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(build_categories)
}

fn build_categories() -> Vec<CandidateGroup> {
    let mut keyboard = Vec::new();
    let mut consumer = Vec::new();
    // Scan every encoded usage below the consumer range; keep only known
    // keyboard-page (0x07) and consumer-page (0x0C) keycodes.
    for encoded in 0..=((HID_USAGE_CONSUMER as u32) << 16 | 0x3FF) {
        let Some(_keycode) = Keycode::from_hid_usage(encoded) else {
            continue;
        };
        let page = HidUsage::from_encoded(encoded).page();
        let candidate = keycode_candidate(encoded);
        match page {
            HID_USAGE_KEYBOARD => keyboard.push(candidate),
            HID_USAGE_CONSUMER => consumer.push(candidate),
            _ => {}
        }
    }
    vec![
        CandidateGroup {
            name: "Keyboard",
            candidates: keyboard,
        },
        CandidateGroup {
            name: "Consumer",
            candidates: consumer,
        },
    ]
}

/// The candidate for a ZMK keycode: the fully resolved `LayoutKey` from the
/// key-press behavior, falling back to a hex label for unknown usages.
pub fn keycode_candidate(encoded: u32) -> Candidate {
    let usage = HidUsage::from_encoded(encoded);
    let action = KeyAction::Zmk(Behavior::KeyPress(usage));
    let key = match action.resolve_label(&[]) {
        Some(key) if !key.tap.full.is_empty() => key,
        Some(mut key) => {
            if key.symbol.is_none() && key.tap.is_empty() {
                key.symbol = Some(format!("0x{:02X}", usage.id()));
            }
            key
        }
        None => crate::layout_key::LayoutKey {
            tap: crate::layout_key::Label::new(format!("0x{:02X}", usage.id())),
            ..Default::default()
        },
    };
    Candidate::new(encoded, key)
}
