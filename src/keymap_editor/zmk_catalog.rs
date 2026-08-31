//! Candidate ZMK keycodes and behaviors for picker grids.

use crate::key_action::{KeyAction, LayerInfo};
use crate::layout_key::{Label, LayoutKey};
use crate::zmk_keycode_labels::behavior_to_layout_key;
use std::sync::OnceLock;
use zmk_studio_api::{Behavior, HidUsage, Keycode};
pub const HID_USAGE_KEYBOARD: u16 = 0x07;
pub const HID_USAGE_CONSUMER: u16 = 0x0C;
const MAX_USAGE_ID: u32 = 0x3FF;

/// Backlight command identifier for setting brightness level.
pub const BACKLIGHT_SET_COMMAND: u32 = 6;

use super::picker::{Candidate, CandidateGroup};

pub use zmk_studio_api::BehaviorRole as ZmkBehaviorKind;

/// Returns a sample Behavior instance for a behavior role.
pub fn sample_behavior(role: ZmkBehaviorKind) -> Behavior {
    use zmk_studio_api::BehaviorRole::*;
    let sample_usage = HidUsage::from(Keycode::A);
    let sample_mod_usage = HidUsage::from(Keycode::LEFT_SHIFT);
    match role {
        KeyPress => Behavior::KeyPress(sample_usage),
        KeyToggle => Behavior::KeyToggle(sample_usage),
        StickyKey => Behavior::StickyKey(sample_usage),
        MomentaryLayer => Behavior::MomentaryLayer { layer_id: 0 },
        ToggleLayer => Behavior::ToggleLayer { layer_id: 0 },
        ToLayer => Behavior::ToLayer { layer_id: 0 },
        StickyLayer => Behavior::StickyLayer { layer_id: 0 },
        LayerTap => Behavior::LayerTap {
            layer_id: 0,
            tap: sample_usage,
        },
        ModTap => Behavior::ModTap {
            hold: sample_mod_usage,
            tap: sample_usage,
        },
        Transparent => Behavior::Transparent,
        None => Behavior::None,
        CapsWord => Behavior::CapsWord,
        KeyRepeat => Behavior::KeyRepeat,
        GraveEscape => Behavior::GraveEscape,
        StudioUnlock => Behavior::StudioUnlock,
        Reset => Behavior::Reset,
        Bootloader => Behavior::Bootloader,
        SoftOff => Behavior::SoftOff,
        Bluetooth => Behavior::Bluetooth {
            command: 0,
            value: 0,
        },
        ExternalPower => Behavior::ExternalPower { value: 0 },
        OutputSelection => Behavior::OutputSelection { value: 0 },
        Backlight => Behavior::Backlight {
            command: 0,
            value: 0,
        },
        Underglow => Behavior::Underglow {
            command: 0,
            value: 0,
        },
        MouseKeyPress => Behavior::MouseKeyPress { value: 1 },
        MouseMove => Behavior::MouseMove { value: 0 },
        MouseScroll => Behavior::MouseScroll { value: 0 },
    }
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
pub fn command_candidates(kind: ZmkBehaviorKind, backlight_level: u32) -> Vec<Candidate> {
    use ZmkBehaviorKind::*;
    let behaviors: Vec<Behavior> = match kind {
        Bluetooth => {
            // Profile select and disconnect commands (profiles 0 to 9).
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
                command: BACKLIGHT_SET_COMMAND,
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
        // Pointing coordinates: `(x << 16) | (y & 0xFFFF)`.
        MouseMove => [0x0001_0000, 0xFFFF_0000, 0x0000_0001, 0x0000_FFFF]
            .into_iter()
            .map(|value| Behavior::MouseMove { value })
            .collect(),
        MouseScroll => [0x0000_0001, 0x0000_FFFF, 0x0001_0000, 0xFFFF_0000]
            .into_iter()
            .map(|value| Behavior::MouseScroll { value })
            .collect(),
        _ => Vec::new(),
    };
    behaviors
        .into_iter()
        .map(|behavior| behavior_candidate(&behavior, &[]))
        .collect()
}

/// Creates a candidate definition for a ZMK behavior.
pub fn behavior_candidate(behavior: &Behavior, layer_names: &[String]) -> Candidate {
    let key = match behavior {
        Behavior::Transparent => LayoutKey {
            tap: Label::with_short("Transparent", egui_phosphor::regular::CARET_DOWN),
            ..Default::default()
        },
        Behavior::None => LayoutKey {
            tap: Label::new("None"),
            ..Default::default()
        },
        _ => behavior_to_layout_key(behavior, layer_names).unwrap_or_default(),
    };
    Candidate {
        binding: KeyAction::Zmk(behavior.clone()),
        key,
        transparent: *behavior == Behavior::Transparent,
    }
}

pub fn categories() -> &'static [CandidateGroup] {
    static CATEGORIES: OnceLock<Vec<CandidateGroup>> = OnceLock::new();
    CATEGORIES.get_or_init(build_categories)
}

fn build_categories() -> Vec<CandidateGroup> {
    let mut keyboard = Vec::new();
    let mut consumer = Vec::new();
    for &page in &[HID_USAGE_KEYBOARD, HID_USAGE_CONSUMER] {
        for id in 0..=MAX_USAGE_ID {
            let encoded = (page as u32) << 16 | id;
            if Keycode::from_hid_usage(encoded).is_none() {
                continue;
            }
            let candidate = keycode_candidate(encoded);
            match page {
                HID_USAGE_KEYBOARD => keyboard.push(candidate),
                HID_USAGE_CONSUMER => consumer.push(candidate),
                _ => {}
            }
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

/// Creates a candidate definition for a ZMK HID usage code.
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
    Candidate::new(action, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaviors_map_to_correct_roles() {
        assert_eq!(
            Behavior::Backlight {
                command: 0,
                value: 0
            }
            .role(),
            Some(ZmkBehaviorKind::Backlight)
        );
        assert_eq!(
            Behavior::Underglow {
                command: 0,
                value: 0
            }
            .role(),
            Some(ZmkBehaviorKind::Underglow)
        );
        assert_eq!(
            Behavior::Transparent.role(),
            Some(ZmkBehaviorKind::Transparent)
        );
    }
}
