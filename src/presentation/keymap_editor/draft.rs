//! Domain draft model for the unified keymap editor.
//!
//! Stores interactive in-progress key configuration state, and converts directly
//! to and from [`KeySpec`].

use crate::hid_labels::Modifiers;
use crate::key_spec::{BacklightAction, HidKey, KeySpec, LayerActivation, LightingAction};
use super::profile::EditorProfile;

/// Unified sidebar sections for key categories.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum EditorSection {
    // Keys
    #[default]
    Keyboard,
    KeyToggle,
    Special,

    // Parameterized Combos & Layers
    Combo,
    ModTap,
    Layers,
    LayerMod,
    OneShot,

    // Hardware & Connectivity
    Bluetooth,
    Output,
    System,
    BootPower,

    // Lighting & Audio
    Backlight,
    Rgb,
    RgbMatrix,
    Audio,

    // Pointing
    Mouse,

    // Extensions & Raw
    Custom,
    RawHex,
}

impl EditorSection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Keyboard => "Key Press",
            Self::KeyToggle => "Key Toggle",
            Self::Special => "Special",
            Self::Combo => "Modifier Key",
            Self::ModTap => "Mod-Tap",
            Self::Layers => "Layers",
            Self::LayerMod => "Layer Mod",
            Self::OneShot => "Sticky Key",
            Self::Bluetooth => "Bluetooth",
            Self::Output => "Output Selection",
            Self::System => "System",
            Self::BootPower => "Boot & Power",
            Self::Backlight => "Backlight",
            Self::Rgb => "Underglow",
            Self::RgbMatrix => "RGB Matrix",
            Self::Audio => "Audio",
            Self::Mouse => "Mouse",
            Self::Custom => "Custom",
            Self::RawHex => "Any Keycode",
        }
    }
}

/// Backlight command parameters for staged backlight adjustment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BacklightDraft {
    pub value: u8,
    pub staged: bool,
}

/// In-progress editable parameter state for the key editor.
#[derive(Clone, Debug, Default)]
pub struct KeyDraft {
    pub section: EditorSection,
    /// Base key for Combo, ModTap, LayerTap, OneShot, KeyToggle.
    pub tap_key: Option<HidKey>,
    /// Modifier mask for Combo, OneShot, or LayerMod (bits 0..3 Left, 4..7 Right).
    pub modifiers: u8,
    /// Modifier mask for the tap key on ModTap or LayerTap.
    pub tap_modifiers: u8,
    /// Held modifier mask for ModTap.
    pub hold_mods: u8,
    /// Target layer ID for LayerTap or direct layer selection.
    pub target_layer: Option<usize>,
    /// Activation mode for layer actions.
    pub layer_activation: Option<LayerActivation>,
    /// Indicates whether the active layer configuration is Layer-Tap.
    pub is_layer_tap: bool,
    /// Backlight level parameter.
    pub backlight: BacklightDraft,
    /// Raw hex entry string.
    pub hex: String,
}

impl KeyDraft {
    /// Decodes an existing [`KeySpec`] into draft state.
    pub fn from_spec(spec: &KeySpec) -> Self {
        let mut draft = Self::default();

        match spec {
            KeySpec::KeyPress { key, modifiers } => {
                draft.section = EditorSection::Keyboard;
                draft.modifiers = u8_from_modifiers(*modifiers);
                draft.tap_key = Some(*key);
            }
            KeySpec::KeyToggle { key, modifiers } => {
                draft.section = EditorSection::KeyToggle;
                draft.tap_key = Some(*key);
                draft.modifiers = u8_from_modifiers(*modifiers);
            }
            KeySpec::ModTap {
                hold,
                tap,
                tap_modifiers,
            } => {
                draft.section = EditorSection::ModTap;
                draft.hold_mods = u8_from_modifiers(*hold);
                draft.tap_key = Some(*tap);
                draft.tap_modifiers = u8_from_modifiers(*tap_modifiers);
            }
            KeySpec::LayerTap {
                layer,
                tap,
                tap_modifiers,
            } => {
                draft.section = EditorSection::Layers;
                draft.target_layer = Some(*layer as usize);
                draft.tap_key = Some(*tap);
                draft.tap_modifiers = u8_from_modifiers(*tap_modifiers);
                draft.is_layer_tap = true;
                draft.layer_activation = None;
            }
            KeySpec::Layer { layer, activation } => {
                if let LayerActivation::LayerMod(mods) = activation {
                    draft.section = EditorSection::LayerMod;
                    draft.target_layer = Some(*layer as usize);
                    draft.modifiers = u8_from_modifiers(*mods);
                } else {
                    draft.section = EditorSection::Layers;
                    draft.target_layer = Some(*layer as usize);
                    draft.layer_activation = Some(*activation);
                    draft.is_layer_tap = false;
                }
            }
            KeySpec::StickyKey { key, modifiers } => {
                draft.section = EditorSection::OneShot;
                draft.tap_key = *key;
                draft.modifiers = u8_from_modifiers(*modifiers);
            }
            KeySpec::Bluetooth(_) => {
                draft.section = EditorSection::Bluetooth;
            }
            KeySpec::Output(_) => {
                draft.section = EditorSection::Output;
            }
            KeySpec::Power(_) => {
                draft.section = EditorSection::BootPower;
            }
            KeySpec::Lighting(LightingAction::Backlight(bl)) => {
                draft.section = EditorSection::Backlight;
                if let BacklightAction::Set(val) = bl {
                    draft.backlight.value = *val;
                    draft.backlight.staged = true;
                }
            }
            KeySpec::Lighting(LightingAction::Rgb(_)) => {
                draft.section = EditorSection::Rgb;
            }
            KeySpec::Lighting(LightingAction::RgbMatrix(_)) => {
                draft.section = EditorSection::RgbMatrix;
            }
            KeySpec::Audio(_) => {
                draft.section = EditorSection::Audio;
            }
            KeySpec::Mouse(_) => {
                draft.section = EditorSection::Mouse;
            }
            KeySpec::CapsWord
            | KeySpec::KeyRepeat
            | KeySpec::GraveEscape
            | KeySpec::Transparent
            | KeySpec::None => {
                draft.section = EditorSection::Special;
            }
            KeySpec::Custom(_) => {
                draft.section = EditorSection::Custom;
            }
        }

        draft
    }

    /// Initializes draft for a section, preserving active parameters if relevant.
    pub fn for_section(section: EditorSection, current_spec: Option<&KeySpec>) -> Self {
        if let Some(spec) = current_spec {
            let draft = Self::from_spec(spec);
            if draft.section == section {
                return draft;
            }
            if section == EditorSection::Layers {
                return Self {
                    section,
                    tap_key: draft.tap_key,
                    is_layer_tap: false,
                    ..Default::default()
                };
            }
            // Preserve tap_key if navigating to another parameterized section
            if let Some(tap) = draft.tap_key {
                return Self {
                    section,
                    tap_key: Some(tap),
                    modifiers: draft.modifiers,
                    ..Default::default()
                };
            }
        }

        Self {
            section,
            ..Default::default()
        }
    }

    /// Returns the staged [`KeySpec`] if all required parameters are valid.
    ///
    /// Raw-hex staging additionally requires the profile's raw-keycode
    /// parsing; use [`KeyDraft::staged_for`] when an editor profile is available.
    pub fn staged(&self) -> Option<KeySpec> {
        match self.section {
            EditorSection::RawHex => None,
            _ => self.staged_section(),
        }
    }

    /// Like [`KeyDraft::staged`], resolving raw hex entry through the
    /// active editor profile (see [`EditorProfile::parse_raw_keycode`]).
    pub fn staged_for(&self, profile: &dyn EditorProfile) -> Option<KeySpec> {
        match self.section {
            EditorSection::RawHex => profile.parse_raw_keycode(&self.hex),
            _ => self.staged_section(),
        }
    }

    fn staged_section(&self) -> Option<KeySpec> {
        match self.section {
            EditorSection::Keyboard => {
                let tap = self.tap_key?;
                Some(KeySpec::KeyPress {
                    key: tap,
                    modifiers: modifiers_from_u8(self.modifiers),
                })
            }
            EditorSection::Combo => {
                let tap = self.tap_key?;
                if self.modifiers == 0 {
                    return None;
                }
                Some(KeySpec::KeyPress {
                    key: tap,
                    modifiers: modifiers_from_u8(self.modifiers),
                })
            }
            EditorSection::KeyToggle => {
                let tap = self.tap_key?;
                Some(KeySpec::KeyToggle {
                    key: tap,
                    modifiers: modifiers_from_u8(self.modifiers),
                })
            }
            EditorSection::ModTap => {
                let tap = self.tap_key?;
                if self.hold_mods == 0 {
                    return None;
                }
                Some(KeySpec::ModTap {
                    hold: modifiers_from_u8(self.hold_mods),
                    tap,
                    tap_modifiers: modifiers_from_u8(self.tap_modifiers),
                })
            }
            EditorSection::Layers => {
                if self.is_layer_tap {
                    let layer = self.target_layer? as u8;
                    let tap = self.tap_key?;
                    Some(KeySpec::LayerTap {
                        layer,
                        tap,
                        tap_modifiers: modifiers_from_u8(self.tap_modifiers),
                    })
                } else if let Some(layer) = self.target_layer {
                    let activation = self.layer_activation.unwrap_or(LayerActivation::Momentary);
                    Some(KeySpec::Layer {
                        layer: layer as u8,
                        activation,
                    })
                } else {
                    None
                }
            }
            EditorSection::LayerMod => {
                let layer = self.target_layer? as u8;
                if self.modifiers == 0 {
                    return None;
                }
                Some(KeySpec::Layer {
                    layer,
                    activation: LayerActivation::LayerMod(modifiers_from_u8(self.modifiers)),
                })
            }
            EditorSection::OneShot => {
                if self.modifiers == 0 && self.tap_key.is_none() {
                    return None;
                }
                Some(KeySpec::StickyKey {
                    key: self.tap_key,
                    modifiers: modifiers_from_u8(self.modifiers),
                })
            }
            _ => None,
        }
    }

    /// Checks if the draft configuration is complete and ready to commit.
    pub fn is_valid(&self) -> bool {
        match self.section {
            EditorSection::Combo
            | EditorSection::ModTap
            | EditorSection::OneShot
            | EditorSection::KeyToggle
            | EditorSection::LayerMod => self.staged().is_some(),
            EditorSection::RawHex => u16::from_str_radix(&self.hex, 16).is_ok(),
            EditorSection::Layers => {
                if self.is_layer_tap {
                    self.target_layer.is_some() && self.tap_key.is_some()
                } else {
                    self.target_layer.is_some()
                }
            }
            _ => true,
        }
    }

    /// Unmodified keypress representation of the staged base/tap key, useful for picker selection matching.
    pub fn tap_key_spec(&self) -> Option<KeySpec> {
        self.tap_key.map(|key| KeySpec::KeyPress {
            key,
            modifiers: Modifiers::default(),
        })
    }
}

/// Converts a `Modifiers` struct to an 8-bit mask (bits 0..3 Left, 4..7 Right).
pub fn u8_from_modifiers(mods: Modifiers) -> u8 {
    mods.to_hid_mask()
}

/// Converts an 8-bit modifier mask to a `Modifiers` struct.
pub fn modifiers_from_u8(mask: u8) -> Modifiers {
    Modifiers::from_hid_mask(mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_draft_starts_empty() {
        let draft = KeyDraft::default();
        assert_eq!(draft.section, EditorSection::Keyboard);
        assert_eq!(draft.tap_key, None);
        assert_eq!(draft.modifiers, 0);
        assert_eq!(draft.hold_mods, 0);
        assert!(draft.is_valid());
    }

    #[test]
    fn mod_tap_requires_both_hold_and_tap() {
        let mut draft = KeyDraft {
            section: EditorSection::ModTap,
            tap_key: None,
            hold_mods: 0,
            ..Default::default()
        };
        assert_eq!(draft.staged(), None);
        assert!(!draft.is_valid());

        // Set hold only
        draft.hold_mods = 0x02; // Shift
        assert_eq!(draft.staged(), None);
        assert!(!draft.is_valid());

        // Set tap key
        draft.tap_key = Some(HidKey::keyboard(0x04)); // A
        assert!(draft.staged().is_some());
        assert!(draft.is_valid());

        let staged = draft.staged().unwrap();
        assert_eq!(
            staged,
            KeySpec::ModTap {
                hold: Modifiers {
                    shift: true,
                    ..Default::default()
                },
                tap: HidKey::keyboard(0x04),
                tap_modifiers: Modifiers::default(),
            }
        );
    }

    #[test]
    fn layer_tap_requires_both_layer_and_tap() {
        let mut draft = KeyDraft {
            section: EditorSection::Layers,
            tap_key: Some(HidKey::keyboard(0x28)), // Enter
            target_layer: None,
            is_layer_tap: true,
            ..Default::default()
        };
        assert_eq!(draft.staged(), None);
        assert!(!draft.is_valid());

        draft.target_layer = Some(2);
        assert_eq!(
            draft.staged(),
            Some(KeySpec::LayerTap {
                layer: 2,
                tap: HidKey::keyboard(0x28),
                tap_modifiers: Modifiers::default(),
            })
        );
        assert!(draft.is_valid());
    }

    #[test]
    fn layer_direct_activation_does_not_require_tap() {
        let draft = KeyDraft {
            section: EditorSection::Layers,
            target_layer: Some(1),
            layer_activation: Some(LayerActivation::Momentary),
            is_layer_tap: false,
            ..Default::default()
        };
        assert!(draft.is_valid());
        assert_eq!(
            draft.staged(),
            Some(KeySpec::Layer {
                layer: 1,
                activation: LayerActivation::Momentary,
            })
        );
    }

    #[test]
    fn combo_requires_modifiers_and_tap() {
        let mut draft = KeyDraft {
            section: EditorSection::Combo,
            tap_key: Some(HidKey::keyboard(0x04)),
            modifiers: 0,
            ..Default::default()
        };
        assert_eq!(draft.staged(), None);
        assert!(!draft.is_valid());

        draft.modifiers = 0x01 | 0x02; // Ctrl + Shift
        let staged = draft.staged();
        assert!(staged.is_some());
        assert!(draft.is_valid());
        assert_eq!(
            staged.unwrap(),
            KeySpec::KeyPress {
                key: HidKey::keyboard(0x04),
                modifiers: Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn one_shot_stages_sticky_key() {
        let draft = KeyDraft {
            section: EditorSection::OneShot,
            tap_key: None,
            modifiers: 0x01, // Ctrl
            ..Default::default()
        };
        assert!(draft.is_valid());
        assert_eq!(
            draft.staged(),
            Some(KeySpec::StickyKey {
                key: None,
                modifiers: Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            })
        );
    }

    #[test]
    fn round_trip_from_spec() {
        let spec = KeySpec::ModTap {
            hold: Modifiers {
                ctrl: true,
                ..Default::default()
            },
            tap: HidKey::keyboard(0x29),
            tap_modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
        };
        let draft = KeyDraft::from_spec(&spec);
        assert_eq!(draft.section, EditorSection::ModTap);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x29)));
        assert_eq!(draft.hold_mods, 0x01);
        assert_eq!(draft.tap_modifiers, 0x02);
        assert_eq!(draft.staged(), Some(spec));
    }

    #[test]
    fn round_trip_layer_tap() {
        let spec = KeySpec::LayerTap {
            layer: 3,
            tap: HidKey::keyboard(0x28),
            tap_modifiers: Modifiers {
                ctrl: true,
                ..Default::default()
            },
        };
        let draft = KeyDraft::from_spec(&spec);
        assert_eq!(draft.section, EditorSection::Layers);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x28)));
        assert_eq!(draft.tap_modifiers, 0x01);
        assert_eq!(draft.target_layer, Some(3));
        assert_eq!(draft.staged(), Some(spec));
    }

    #[test]
    fn round_trip_layer_mod() {
        let spec = KeySpec::Layer {
            layer: 2,
            activation: LayerActivation::LayerMod(Modifiers {
                shift: true,
                ..Default::default()
            }),
        };
        let draft = KeyDraft::from_spec(&spec);
        assert_eq!(draft.section, EditorSection::LayerMod);
        assert_eq!(draft.target_layer, Some(2));
        assert_eq!(draft.modifiers, 0x02);
        assert_eq!(draft.staged(), Some(spec));
    }

    #[test]
    fn round_trip_key_toggle() {
        let spec = KeySpec::KeyToggle {
            key: HidKey::keyboard(0x39),
            modifiers: Modifiers::default(),
        }; // CapsLock
        let draft = KeyDraft::from_spec(&spec);
        assert_eq!(draft.section, EditorSection::KeyToggle);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x39)));
        assert_eq!(draft.modifiers, 0);
        assert_eq!(draft.staged(), Some(spec));
    }

    #[test]
    fn round_trip_modified_keypress() {
        let spec = KeySpec::KeyPress {
            key: HidKey::keyboard(0x06), // C
            modifiers: Modifiers {
                ctrl: true,
                gui: true,
                ..Default::default()
            },
        };
        let draft = KeyDraft::from_spec(&spec);
        assert_eq!(draft.section, EditorSection::Keyboard);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x06)));
        assert_eq!(draft.modifiers, 0x01 | 0x08);
        assert_eq!(draft.staged(), Some(spec));
    }

    #[test]
    fn round_trip_layer_activation() {
        let spec = KeySpec::Layer {
            layer: 2,
            activation: LayerActivation::Toggle,
        };
        let draft = KeyDraft::from_spec(&spec);
        assert_eq!(draft.section, EditorSection::Layers);
        assert_eq!(draft.target_layer, Some(2));
        assert_eq!(draft.layer_activation, Some(LayerActivation::Toggle));
        assert_eq!(draft.staged(), Some(spec));
    }

    #[test]
    fn round_trip_raw_hex() {
        let draft = KeyDraft {
            section: EditorSection::RawHex,
            hex: "0004".into(),
            ..Default::default()
        };
        assert!(draft.is_valid());
        // Staging raw hex requires an active profile that supports raw keycode parsing;
        // without one nothing stages.
        assert_eq!(draft.staged(), None);
        assert_eq!(
            draft.staged_for(&crate::firmware::qmk::QmkEditorProfile),
            Some(KeySpec::KeyPress {
                key: HidKey::keyboard(0x04),
                modifiers: Default::default(),
            })
        );
        assert_eq!(
            draft.staged_for(&crate::firmware::zmk::ZmkEditorProfile),
            None
        );
    }

    #[test]
    fn for_section_preserves_tap_key_across_sections() {
        let current = KeySpec::KeyPress {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers {
                ctrl: true,
                ..Default::default()
            },
        };
        let draft = KeyDraft::for_section(EditorSection::ModTap, Some(&current));
        assert_eq!(draft.section, EditorSection::ModTap);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x04)));
    }

    #[test]
    fn keyboard_draft_stages_with_and_without_modifiers() {
        let mut draft = KeyDraft {
            section: EditorSection::Keyboard,
            tap_key: Some(HidKey::keyboard(0x04)),
            modifiers: 0,
            ..Default::default()
        };
        assert_eq!(
            draft.staged(),
            Some(KeySpec::KeyPress {
                key: HidKey::keyboard(0x04),
                modifiers: Modifiers::default(),
            })
        );

        draft.modifiers = 0x02; // Shift
        assert_eq!(
            draft.staged(),
            Some(KeySpec::KeyPress {
                key: HidKey::keyboard(0x04),
                modifiers: Modifiers {
                    shift: true,
                    ..Default::default()
                },
            })
        );
    }

    #[test]
    fn for_section_preserves_modifiers_between_keyboard_and_combo() {
        let current = KeySpec::KeyPress {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
        };
        let draft = KeyDraft::for_section(EditorSection::Keyboard, Some(&current));
        assert_eq!(draft.section, EditorSection::Keyboard);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x04)));
        assert_eq!(draft.modifiers, 0x01 | 0x02);
    }

    #[test]
    fn from_spec_routes_audio_and_rgb_matrix() {
        let audio_spec = KeySpec::Audio(crate::key_spec::AudioAction::On);
        let draft = KeyDraft::from_spec(&audio_spec);
        assert_eq!(draft.section, EditorSection::Audio);

        let rgb_matrix_spec = KeySpec::Lighting(crate::key_spec::LightingAction::RgbMatrix(
            crate::key_spec::RgbMatrixAction::Toggle,
        ));
        let draft = KeyDraft::from_spec(&rgb_matrix_spec);
        assert_eq!(draft.section, EditorSection::RgbMatrix);
    }

    #[test]
    fn round_trip_key_toggle_with_modifiers() {
        let spec = KeySpec::KeyToggle {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
        };
        let draft = KeyDraft::from_spec(&spec);
        assert_eq!(draft.section, EditorSection::KeyToggle);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x04)));
        assert_eq!(draft.modifiers, 0x01 | 0x02);
        assert_eq!(draft.staged(), Some(spec));
    }

    #[test]
    fn round_trip_sticky_key_with_key_and_modifiers() {
        let spec = KeySpec::StickyKey {
            key: Some(HidKey::keyboard(0x04)),
            modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
        };
        let draft = KeyDraft::from_spec(&spec);
        assert_eq!(draft.section, EditorSection::OneShot);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x04)));
        assert_eq!(draft.modifiers, 0x02);
        assert_eq!(draft.staged(), Some(spec));
    }

    #[test]
    fn editor_section_labels_are_firmware_neutral() {
        assert_eq!(EditorSection::Keyboard.label(), "Key Press");
        assert_eq!(EditorSection::Combo.label(), "Modifier Key");
        assert_eq!(EditorSection::OneShot.label(), "Sticky Key");
        assert_eq!(EditorSection::Output.label(), "Output Selection");
        assert_eq!(EditorSection::Rgb.label(), "Underglow");
    }

    #[test]
    fn draft_mod_tap_compound_hold_modifiers() {
        let mut draft = KeyDraft {
            section: EditorSection::ModTap,
            tap_key: Some(HidKey::keyboard(0x04)), // A
            hold_mods: 0,
            ..Default::default()
        };
        // Toggle LCtrl (0x01) and LShift (0x02)
        draft.hold_mods ^= 0x01;
        draft.hold_mods ^= 0x02;
        assert_eq!(draft.hold_mods, 0x03);

        let staged = draft.staged().expect("should be valid");
        assert_eq!(
            staged,
            KeySpec::ModTap {
                hold: Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Default::default()
                },
                tap: HidKey::keyboard(0x04),
                tap_modifiers: Modifiers::default(),
            }
        );
    }

    #[test]
    fn draft_section_routing_is_firmware_agnostic() {
        let modified_key = KeySpec::KeyPress {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers {
                ctrl: true,
                ..Default::default()
            },
        };
        // Modified keys route to Keyboard (which has the modifier grid).
        let draft = KeyDraft::from_spec(&modified_key);
        assert_eq!(draft.section, EditorSection::Keyboard);
        assert_eq!(draft.modifiers, 0x01);
        assert_eq!(draft.tap_key, Some(HidKey::keyboard(0x04)));

        let media_key = KeySpec::KeyPress {
            key: HidKey::consumer(0xE2), // Mute
            modifiers: Modifiers::default(),
        };
        // Media keys are ordinary key presses: they route to Key Press, where
        // the Media group is available alongside the keyboard keys.
        let draft = KeyDraft::from_spec(&media_key);
        assert_eq!(draft.section, EditorSection::Keyboard);
        assert_eq!(draft.tap_key, Some(HidKey::consumer(0xE2)));
    }

    #[test]
    fn draft_round_trips_all_8_modifier_bits() {
        let mods = Modifiers {
            ctrl: true,
            shift: true,
            alt: true,
            gui: true,
            right_ctrl: true,
            right_shift: true,
            right_alt: true,
            right_gui: true,
        };
        let mask = u8_from_modifiers(mods);
        assert_eq!(mask, 0xFF);
        let round_trip = modifiers_from_u8(mask);
        assert_eq!(mods, round_trip);
    }
}
