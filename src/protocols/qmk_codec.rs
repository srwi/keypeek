//! Codec translating between QMK keycodes and domain [`KeySpec`].

use crate::hid_labels::Modifiers;
use crate::key_spec::{
    BacklightAction, CustomBinding, CustomKind, HidKey, KeySpec, LayerActivation, LightingAction,
    MouseAction, MouseButton, PowerAction, RgbAction,
};
use crate::protocols::DeviceError;
use qmk_via_api::keycodes::Keycode;
use qmk_via_api::ranges::*;
use qmk_via_api::{QmkKeycode, QmkLayerOp};

/// Decodes a 16-bit QMK keycode into a normalized domain [`KeySpec`].
pub fn qmk_to_keyspec(code: u16) -> KeySpec {
    if code == Keycode::KC_TRANSPARENT as u16 {
        return KeySpec::Transparent;
    }
    if code == Keycode::KC_NO as u16 {
        return KeySpec::None;
    }

    match QmkKeycode::from_u16(code) {
        QmkKeycode::ModCombo { mods, keycode } => KeySpec::KeyPress {
            key: HidKey::keyboard(keycode as u16),
            modifiers: from_qmk_mask(mods),
        },
        QmkKeycode::ModTap { mods, keycode } => KeySpec::ModTap {
            hold: from_qmk_mask(mods),
            tap: HidKey::keyboard(keycode as u16),
            tap_modifiers: Modifiers::default(),
        },
        QmkKeycode::LayerTap { layer, keycode } => KeySpec::LayerTap {
            layer,
            tap: HidKey::keyboard(keycode as u16),
            tap_modifiers: Modifiers::default(),
        },
        QmkKeycode::LayerMod { layer, mods } => KeySpec::Layer {
            layer,
            activation: LayerActivation::LayerMod(from_qmk_mask(mods)),
        },
        QmkKeycode::OneShotMod(mods) => KeySpec::StickyKey {
            key: None,
            modifiers: from_qmk_mask(mods),
        },
        QmkKeycode::LayerOp { op, layer } => {
            let activation = match op {
                QmkLayerOp::Momentary | QmkLayerOp::TapToggle => LayerActivation::Momentary,
                QmkLayerOp::Toggle => LayerActivation::Toggle,
                QmkLayerOp::To => LayerActivation::To,
                QmkLayerOp::OneShot => LayerActivation::Sticky,
                QmkLayerOp::Default => LayerActivation::Default,
            };
            KeySpec::Layer { layer, activation }
        }
        QmkKeycode::TapDance(n) => KeySpec::Custom(CustomBinding {
            kind: CustomKind::TapDance,
            id: n as u32,
            name: None,
            param1: None,
            param2: None,
        }),
        QmkKeycode::Macro(n) => KeySpec::Custom(CustomBinding {
            kind: CustomKind::Macro,
            id: n as u32,
            name: None,
            param1: None,
            param2: None,
        }),
        QmkKeycode::CustomKb(n) => KeySpec::Custom(CustomBinding {
            kind: CustomKind::Keyboard,
            id: n as u32,
            name: None,
            param1: None,
            param2: None,
        }),
        QmkKeycode::CustomUser(n) => KeySpec::Custom(CustomBinding {
            kind: CustomKind::User,
            id: n as u32,
            name: None,
            param1: None,
            param2: None,
        }),
        _ => {
            if let Ok(kc) = Keycode::try_from(code) {
                if let Some(spec) = qmk_special_keycode_to_spec(kc) {
                    return spec;
                }
            }

            // Standard USB HID keyboard keycodes
            if code <= 0x00A4 || (0x00E0..=0x00E7).contains(&code) {
                return KeySpec::KeyPress {
                    key: HidKey::keyboard(code),
                    modifiers: Modifiers::default(),
                };
            }

            // Fallback for quantum or custom keycodes
            KeySpec::Custom(CustomBinding {
                kind: CustomKind::Raw,
                id: code as u32,
                name: None,
                param1: None,
                param2: None,
            })
        }
    }
}

/// Encodes a normalized domain [`KeySpec`] into a 16-bit QMK keycode.
pub fn keyspec_to_qmk(spec: &KeySpec) -> Result<u16, DeviceError> {
    match spec {
        KeySpec::Transparent => Ok(Keycode::KC_TRANSPARENT as u16),
        KeySpec::None => Ok(Keycode::KC_NO as u16),

        KeySpec::KeyPress { key, modifiers } => {
            if modifiers.is_empty() {
                if key.page == 0x07 {
                    return Ok(key.id);
                }
                if key.page == 0x0C {
                    return consumer_hid_to_qmk(key.id);
                }
                if key.page == 0x01 {
                    return system_hid_to_qmk(key.id);
                }
                Ok(key.id)
            } else {
                if key.page != 0x07 {
                    return Err(DeviceError::Unsupported(format!(
                        "Modified key with HID page 0x{:02X} not supported in QMK",
                        key.page
                    )));
                }
                QmkKeycode::encode_mod_combo(to_qmk_mask(*modifiers), key.id as u8).ok_or_else(
                    || {
                        DeviceError::Unsupported(format!(
                            "Cannot encode QMK mod combo for key 0x{:02X}",
                            key.id
                        ))
                    },
                )
            }
        }

        KeySpec::ModTap {
            hold,
            tap,
            tap_modifiers,
        } => {
            if !tap_modifiers.is_empty() {
                return Err(DeviceError::Unsupported(
                    "QMK does not support modifiers on mod tap key".to_string(),
                ));
            }
            QmkKeycode::encode_mod_tap(to_qmk_mask(*hold), tap.id as u8).ok_or_else(|| {
                DeviceError::Unsupported(format!(
                    "Cannot encode QMK mod tap for key 0x{:02X}",
                    tap.id
                ))
            })
        }

        KeySpec::LayerTap {
            layer,
            tap,
            tap_modifiers,
        } => {
            if !tap_modifiers.is_empty() {
                return Err(DeviceError::Unsupported(
                    "QMK does not support modifiers on layer tap key".to_string(),
                ));
            }
            QmkKeycode::encode_layer_tap(*layer, tap.id as u8).ok_or_else(|| {
                DeviceError::Unsupported(format!("Cannot encode QMK layer tap for layer {}", layer))
            })
        }

        KeySpec::Layer { layer, activation } => match activation {
            LayerActivation::Momentary => QmkLayerOp::Momentary
                .encode(*layer)
                .ok_or_else(|| DeviceError::Unsupported(format!("Cannot encode MO({})", layer))),
            LayerActivation::Toggle => QmkLayerOp::Toggle
                .encode(*layer)
                .ok_or_else(|| DeviceError::Unsupported(format!("Cannot encode TG({})", layer))),
            LayerActivation::To => QmkLayerOp::To
                .encode(*layer)
                .ok_or_else(|| DeviceError::Unsupported(format!("Cannot encode TO({})", layer))),
            LayerActivation::Sticky => QmkLayerOp::OneShot
                .encode(*layer)
                .ok_or_else(|| DeviceError::Unsupported(format!("Cannot encode OSL({})", layer))),
            LayerActivation::Default => QmkLayerOp::Default
                .encode(*layer)
                .ok_or_else(|| DeviceError::Unsupported(format!("Cannot encode DF({})", layer))),
            LayerActivation::LayerMod(mods) => {
                QmkKeycode::encode_layer_mod(*layer, to_qmk_mask(*mods))
                    .ok_or_else(|| DeviceError::Unsupported(format!("Cannot encode LM({})", layer)))
            }
        },

        KeySpec::StickyKey {
            key: None,
            modifiers,
        } => QmkKeycode::encode_one_shot_mod(to_qmk_mask(*modifiers))
            .ok_or_else(|| DeviceError::Unsupported("Cannot encode QMK OSM".to_string())),

        KeySpec::GraveEscape => Ok(Keycode::QK_GRAVE_ESCAPE as u16),
        KeySpec::CapsWord => Ok(Keycode::QK_CAPS_WORD_TOGGLE as u16),
        KeySpec::KeyRepeat => Ok(Keycode::QK_REPEAT_KEY as u16),

        KeySpec::Power(PowerAction::Bootloader) => Ok(Keycode::QK_BOOTLOADER as u16),
        KeySpec::Power(PowerAction::Reset) => Ok(Keycode::QK_REBOOT as u16),
        KeySpec::Power(PowerAction::Other(0xEE)) => Ok(Keycode::QK_CLEAR_EEPROM as u16),

        KeySpec::Mouse(action) => match action {
            MouseAction::Move { x: 0, y: -1 } => Ok(Keycode::QK_MOUSE_CURSOR_UP as u16),
            MouseAction::Move { x: 0, y: 1 } => Ok(Keycode::QK_MOUSE_CURSOR_DOWN as u16),
            MouseAction::Move { x: -1, y: 0 } => Ok(Keycode::QK_MOUSE_CURSOR_LEFT as u16),
            MouseAction::Move { x: 1, y: 0 } => Ok(Keycode::QK_MOUSE_CURSOR_RIGHT as u16),
            MouseAction::Press(MouseButton::Left) => Ok(Keycode::QK_MOUSE_BUTTON_1 as u16),
            MouseAction::Press(MouseButton::Right) => Ok(Keycode::QK_MOUSE_BUTTON_2 as u16),
            MouseAction::Press(MouseButton::Middle) => Ok(Keycode::QK_MOUSE_BUTTON_3 as u16),
            MouseAction::Press(MouseButton::Button4) => Ok(Keycode::QK_MOUSE_BUTTON_4 as u16),
            MouseAction::Press(MouseButton::Button5) => Ok(Keycode::QK_MOUSE_BUTTON_5 as u16),
            MouseAction::Press(MouseButton::Other(6)) => Ok(Keycode::QK_MOUSE_BUTTON_6 as u16),
            MouseAction::Press(MouseButton::Other(7)) => Ok(Keycode::QK_MOUSE_BUTTON_7 as u16),
            MouseAction::Press(MouseButton::Other(8)) => Ok(Keycode::QK_MOUSE_BUTTON_8 as u16),
            MouseAction::Scroll { x: 0, y: 1 } => Ok(Keycode::QK_MOUSE_WHEEL_UP as u16),
            MouseAction::Scroll { x: 0, y: -1 } => Ok(Keycode::QK_MOUSE_WHEEL_DOWN as u16),
            MouseAction::Scroll { x: -1, y: 0 } => Ok(Keycode::QK_MOUSE_WHEEL_LEFT as u16),
            MouseAction::Scroll { x: 1, y: 0 } => Ok(Keycode::QK_MOUSE_WHEEL_RIGHT as u16),
            MouseAction::Acceleration(0) => Ok(Keycode::QK_MOUSE_ACCELERATION_0 as u16),
            MouseAction::Acceleration(1) => Ok(Keycode::QK_MOUSE_ACCELERATION_1 as u16),
            MouseAction::Acceleration(2) => Ok(Keycode::QK_MOUSE_ACCELERATION_2 as u16),
            _ => Err(DeviceError::Unsupported(
                "Mouse action not supported in QMK".to_string(),
            )),
        },

        KeySpec::Lighting(lighting) => match lighting {
            LightingAction::Backlight(BacklightAction::On) => Ok(Keycode::QK_BACKLIGHT_ON as u16),
            LightingAction::Backlight(BacklightAction::Off) => Ok(Keycode::QK_BACKLIGHT_OFF as u16),
            LightingAction::Backlight(BacklightAction::Toggle) => {
                Ok(Keycode::QK_BACKLIGHT_TOGGLE as u16)
            }
            LightingAction::Backlight(BacklightAction::Inc) => Ok(Keycode::QK_BACKLIGHT_UP as u16),
            LightingAction::Backlight(BacklightAction::Dec) => {
                Ok(Keycode::QK_BACKLIGHT_DOWN as u16)
            }
            LightingAction::Backlight(BacklightAction::Cycle) => {
                Ok(Keycode::QK_BACKLIGHT_STEP as u16)
            }
            LightingAction::Rgb(RgbAction::Toggle) => Ok(Keycode::QK_UNDERGLOW_TOGGLE as u16),
            LightingAction::Rgb(RgbAction::EffectInc) => Ok(Keycode::QK_UNDERGLOW_MODE_NEXT as u16),
            LightingAction::Rgb(RgbAction::EffectDec) => {
                Ok(Keycode::QK_UNDERGLOW_MODE_PREVIOUS as u16)
            }
            LightingAction::Rgb(RgbAction::HueInc) => Ok(Keycode::QK_UNDERGLOW_HUE_UP as u16),
            LightingAction::Rgb(RgbAction::HueDec) => Ok(Keycode::QK_UNDERGLOW_HUE_DOWN as u16),
            LightingAction::Rgb(RgbAction::SatInc) => {
                Ok(Keycode::QK_UNDERGLOW_SATURATION_UP as u16)
            }
            LightingAction::Rgb(RgbAction::SatDec) => {
                Ok(Keycode::QK_UNDERGLOW_SATURATION_DOWN as u16)
            }
            LightingAction::Rgb(RgbAction::BrightInc) => Ok(Keycode::QK_UNDERGLOW_VALUE_UP as u16),
            LightingAction::Rgb(RgbAction::BrightDec) => {
                Ok(Keycode::QK_UNDERGLOW_VALUE_DOWN as u16)
            }
            LightingAction::Rgb(RgbAction::SpeedInc) => Ok(Keycode::QK_UNDERGLOW_SPEED_UP as u16),
            LightingAction::Rgb(RgbAction::SpeedDec) => Ok(Keycode::QK_UNDERGLOW_SPEED_DOWN as u16),
            _ => Err(DeviceError::Unsupported(
                "Lighting action not supported in QMK".to_string(),
            )),
        },

        KeySpec::Custom(binding) => match binding.kind {
            CustomKind::TapDance => Ok(QK_TAP_DANCE.start + binding.id as u16),
            CustomKind::Macro => Ok(QK_MACRO.start + binding.id as u16),
            CustomKind::Keyboard => Ok(QK_KB.start + binding.id as u16),
            CustomKind::User => Ok(QK_USER.start + binding.id as u16),
            CustomKind::Raw => Ok(binding.id as u16),
        },

        _ => Err(DeviceError::Unsupported(
            "Action not supported on QMK keyboards".to_string(),
        )),
    }
}

fn qmk_special_keycode_to_spec(kc: Keycode) -> Option<KeySpec> {
    match kc {
        Keycode::QK_GRAVE_ESCAPE => Some(KeySpec::GraveEscape),
        Keycode::QK_CAPS_WORD_TOGGLE => Some(KeySpec::CapsWord),
        Keycode::QK_REPEAT_KEY => Some(KeySpec::KeyRepeat),
        Keycode::QK_BOOTLOADER => Some(KeySpec::Power(PowerAction::Bootloader)),
        Keycode::QK_REBOOT => Some(KeySpec::Power(PowerAction::Reset)),
        Keycode::QK_CLEAR_EEPROM => Some(KeySpec::Power(PowerAction::Other(0xEE))),

        // Mouse keys
        Keycode::QK_MOUSE_CURSOR_UP => Some(KeySpec::Mouse(MouseAction::Move { x: 0, y: -1 })),
        Keycode::QK_MOUSE_CURSOR_DOWN => Some(KeySpec::Mouse(MouseAction::Move { x: 0, y: 1 })),
        Keycode::QK_MOUSE_CURSOR_LEFT => Some(KeySpec::Mouse(MouseAction::Move { x: -1, y: 0 })),
        Keycode::QK_MOUSE_CURSOR_RIGHT => Some(KeySpec::Mouse(MouseAction::Move { x: 1, y: 0 })),
        Keycode::QK_MOUSE_BUTTON_1 => Some(KeySpec::Mouse(MouseAction::Press(MouseButton::Left))),
        Keycode::QK_MOUSE_BUTTON_2 => Some(KeySpec::Mouse(MouseAction::Press(MouseButton::Right))),
        Keycode::QK_MOUSE_BUTTON_3 => Some(KeySpec::Mouse(MouseAction::Press(MouseButton::Middle))),
        Keycode::QK_MOUSE_BUTTON_4 => {
            Some(KeySpec::Mouse(MouseAction::Press(MouseButton::Button4)))
        }
        Keycode::QK_MOUSE_BUTTON_5 => {
            Some(KeySpec::Mouse(MouseAction::Press(MouseButton::Button5)))
        }
        Keycode::QK_MOUSE_BUTTON_6 => {
            Some(KeySpec::Mouse(MouseAction::Press(MouseButton::Other(6))))
        }
        Keycode::QK_MOUSE_BUTTON_7 => {
            Some(KeySpec::Mouse(MouseAction::Press(MouseButton::Other(7))))
        }
        Keycode::QK_MOUSE_BUTTON_8 => {
            Some(KeySpec::Mouse(MouseAction::Press(MouseButton::Other(8))))
        }
        Keycode::QK_MOUSE_WHEEL_UP => Some(KeySpec::Mouse(MouseAction::Scroll { x: 0, y: 1 })),
        Keycode::QK_MOUSE_WHEEL_DOWN => Some(KeySpec::Mouse(MouseAction::Scroll { x: 0, y: -1 })),
        Keycode::QK_MOUSE_WHEEL_LEFT => Some(KeySpec::Mouse(MouseAction::Scroll { x: -1, y: 0 })),
        Keycode::QK_MOUSE_WHEEL_RIGHT => Some(KeySpec::Mouse(MouseAction::Scroll { x: 1, y: 0 })),
        Keycode::QK_MOUSE_ACCELERATION_0 => Some(KeySpec::Mouse(MouseAction::Acceleration(0))),
        Keycode::QK_MOUSE_ACCELERATION_1 => Some(KeySpec::Mouse(MouseAction::Acceleration(1))),
        Keycode::QK_MOUSE_ACCELERATION_2 => Some(KeySpec::Mouse(MouseAction::Acceleration(2))),

        // System keys
        Keycode::KC_SYSTEM_POWER => Some(KeySpec::KeyPress {
            key: HidKey::system(0x81),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_SYSTEM_SLEEP => Some(KeySpec::KeyPress {
            key: HidKey::system(0x82),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_SYSTEM_WAKE => Some(KeySpec::KeyPress {
            key: HidKey::system(0x83),
            modifiers: Modifiers::default(),
        }),

        // Consumer / Media keys
        Keycode::KC_AUDIO_MUTE => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xE2),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_AUDIO_VOL_UP => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xE9),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_AUDIO_VOL_DOWN => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xEA),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MEDIA_NEXT_TRACK => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xB5),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MEDIA_PREV_TRACK => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xB6),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MEDIA_STOP => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xB7),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MEDIA_PLAY_PAUSE => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xCD),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MEDIA_FAST_FORWARD => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xB3),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MEDIA_REWIND => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xB4),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MEDIA_EJECT => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0xB8),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MAIL => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x18A),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_CALCULATOR => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x192),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_MY_COMPUTER => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x194),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_WWW_SEARCH => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x221),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_WWW_HOME => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x223),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_WWW_BACK => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x224),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_WWW_FORWARD => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x225),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_WWW_STOP => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x226),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_WWW_REFRESH => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x227),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_WWW_FAVORITES => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x22A),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_BRIGHTNESS_UP => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x6F),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_BRIGHTNESS_DOWN => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x70),
            modifiers: Modifiers::default(),
        }),
        Keycode::KC_CONTROL_PANEL => Some(KeySpec::KeyPress {
            key: HidKey::consumer(0x19F),
            modifiers: Modifiers::default(),
        }),

        // Backlight
        Keycode::QK_BACKLIGHT_ON => Some(KeySpec::Lighting(LightingAction::Backlight(
            BacklightAction::On,
        ))),
        Keycode::QK_BACKLIGHT_OFF => Some(KeySpec::Lighting(LightingAction::Backlight(
            BacklightAction::Off,
        ))),
        Keycode::QK_BACKLIGHT_TOGGLE => Some(KeySpec::Lighting(LightingAction::Backlight(
            BacklightAction::Toggle,
        ))),
        Keycode::QK_BACKLIGHT_UP => Some(KeySpec::Lighting(LightingAction::Backlight(
            BacklightAction::Inc,
        ))),
        Keycode::QK_BACKLIGHT_DOWN => Some(KeySpec::Lighting(LightingAction::Backlight(
            BacklightAction::Dec,
        ))),
        Keycode::QK_BACKLIGHT_STEP => Some(KeySpec::Lighting(LightingAction::Backlight(
            BacklightAction::Cycle,
        ))),

        // Underglow
        Keycode::QK_UNDERGLOW_TOGGLE => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::Toggle)))
        }
        Keycode::QK_UNDERGLOW_MODE_NEXT => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::EffectInc)))
        }
        Keycode::QK_UNDERGLOW_MODE_PREVIOUS => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::EffectDec)))
        }
        Keycode::QK_UNDERGLOW_HUE_UP => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::HueInc)))
        }
        Keycode::QK_UNDERGLOW_HUE_DOWN => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::HueDec)))
        }
        Keycode::QK_UNDERGLOW_SATURATION_UP => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::SatInc)))
        }
        Keycode::QK_UNDERGLOW_SATURATION_DOWN => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::SatDec)))
        }
        Keycode::QK_UNDERGLOW_VALUE_UP => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::BrightInc)))
        }
        Keycode::QK_UNDERGLOW_VALUE_DOWN => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::BrightDec)))
        }
        Keycode::QK_UNDERGLOW_SPEED_UP => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::SpeedInc)))
        }
        Keycode::QK_UNDERGLOW_SPEED_DOWN => {
            Some(KeySpec::Lighting(LightingAction::Rgb(RgbAction::SpeedDec)))
        }

        _ => None,
    }
}

fn consumer_hid_to_qmk(id: u16) -> Result<u16, DeviceError> {
    match id {
        0xE2 => Ok(Keycode::KC_AUDIO_MUTE as u16),
        0xE9 => Ok(Keycode::KC_AUDIO_VOL_UP as u16),
        0xEA => Ok(Keycode::KC_AUDIO_VOL_DOWN as u16),
        0xB5 => Ok(Keycode::KC_MEDIA_NEXT_TRACK as u16),
        0xB6 => Ok(Keycode::KC_MEDIA_PREV_TRACK as u16),
        0xB7 => Ok(Keycode::KC_MEDIA_STOP as u16),
        0xCD => Ok(Keycode::KC_MEDIA_PLAY_PAUSE as u16),
        0xB3 => Ok(Keycode::KC_MEDIA_FAST_FORWARD as u16),
        0xB4 => Ok(Keycode::KC_MEDIA_REWIND as u16),
        0xB8 => Ok(Keycode::KC_MEDIA_EJECT as u16),
        0x18A => Ok(Keycode::KC_MAIL as u16),
        0x192 => Ok(Keycode::KC_CALCULATOR as u16),
        0x194 => Ok(Keycode::KC_MY_COMPUTER as u16),
        0x221 => Ok(Keycode::KC_WWW_SEARCH as u16),
        0x223 => Ok(Keycode::KC_WWW_HOME as u16),
        0x224 => Ok(Keycode::KC_WWW_BACK as u16),
        0x225 => Ok(Keycode::KC_WWW_FORWARD as u16),
        0x226 => Ok(Keycode::KC_WWW_STOP as u16),
        0x227 => Ok(Keycode::KC_WWW_REFRESH as u16),
        0x22A => Ok(Keycode::KC_WWW_FAVORITES as u16),
        0x6F => Ok(Keycode::KC_BRIGHTNESS_UP as u16),
        0x70 => Ok(Keycode::KC_BRIGHTNESS_DOWN as u16),
        0x19F => Ok(Keycode::KC_CONTROL_PANEL as u16),
        _ => Err(DeviceError::Unsupported(format!(
            "Consumer HID usage 0x{:04X} not supported in QMK",
            id
        ))),
    }
}

fn system_hid_to_qmk(id: u16) -> Result<u16, DeviceError> {
    match id {
        0x81 => Ok(Keycode::KC_SYSTEM_POWER as u16),
        0x82 => Ok(Keycode::KC_SYSTEM_SLEEP as u16),
        0x83 => Ok(Keycode::KC_SYSTEM_WAKE as u16),
        _ => Err(DeviceError::Unsupported(format!(
            "System HID usage 0x{:04X} not supported in QMK",
            id
        ))),
    }
}

/// Translates a [`Modifiers`] domain struct into a QMK [`qmk_via_api::QmkModMask`].
pub fn to_qmk_mask(mods: Modifiers) -> qmk_via_api::QmkModMask {
    let mut bits = 0;
    if mods.ctrl {
        bits |= qmk_via_api::QmkModMask::LCTL;
    }
    if mods.shift {
        bits |= qmk_via_api::QmkModMask::LSFT;
    }
    if mods.alt {
        bits |= qmk_via_api::QmkModMask::LALT;
        if mods.right_alt {
            bits |= qmk_via_api::QmkModMask::RIGHT_HAND;
        }
    }
    if mods.gui {
        bits |= qmk_via_api::QmkModMask::LGUI;
    }
    qmk_via_api::QmkModMask::from_bits(bits)
}

/// Translates a QMK [`qmk_via_api::QmkModMask`] into a [`Modifiers`] domain struct.
pub fn from_qmk_mask(mods: qmk_via_api::QmkModMask) -> Modifiers {
    Modifiers {
        ctrl: mods.has_ctrl(),
        shift: mods.has_shift(),
        alt: mods.has_alt(),
        gui: mods.has_gui(),
        right_alt: mods.has_alt() && mods.is_right(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qmk_round_trips() {
        let test_codes = [
            Keycode::KC_A as u16,
            Keycode::KC_ENTER as u16,
            Keycode::KC_SPACE as u16,
            Keycode::KC_TRANSPARENT as u16,
            Keycode::KC_NO as u16,
            Keycode::KC_AUDIO_MUTE as u16,
            Keycode::QK_GRAVE_ESCAPE as u16,
            Keycode::QK_BOOTLOADER as u16,
            Keycode::QK_UNDERGLOW_TOGGLE as u16,
            Keycode::QK_MOUSE_BUTTON_1 as u16,
        ];

        for code in test_codes {
            let spec = qmk_to_keyspec(code);
            let round_trip = keyspec_to_qmk(&spec).expect("should encode");
            assert_eq!(code, round_trip, "code 0x{:04X} did not round trip", code);
        }
    }

    #[test]
    fn test_qmk_mod_combo_round_trip() {
        let code = QmkKeycode::encode_mod_combo(
            qmk_via_api::QmkModMask::from_bits(qmk_via_api::QmkModMask::LCTL),
            0x06,
        )
        .unwrap();
        let spec = qmk_to_keyspec(code);
        let round_trip = keyspec_to_qmk(&spec).expect("should encode");
        assert_eq!(code, round_trip);
    }

    #[test]
    fn test_qmk_layer_tap_round_trip() {
        let code = QmkKeycode::encode_layer_tap(2, 0x2C).unwrap();
        let spec = qmk_to_keyspec(code);
        let round_trip = keyspec_to_qmk(&spec).expect("should encode");
        assert_eq!(code, round_trip);
    }

    #[test]
    fn test_qmk_display_fidelity_parity() {
        let test_codes = [
            Keycode::KC_TRANSPARENT as u16,
            Keycode::KC_NO as u16,
            Keycode::KC_A as u16,
            Keycode::KC_1 as u16,
            Keycode::KC_ENTER as u16,
            Keycode::KC_SPACE as u16,
            Keycode::KC_ESCAPE as u16,
            // Mod combos
            QmkKeycode::encode_mod_combo(
                qmk_via_api::QmkModMask::from_bits(qmk_via_api::QmkModMask::LCTL),
                0x04,
            )
            .unwrap(),
            QmkKeycode::encode_mod_combo(
                qmk_via_api::QmkModMask::from_bits(
                    qmk_via_api::QmkModMask::LSFT | qmk_via_api::QmkModMask::LCTL,
                ),
                0x05,
            )
            .unwrap(),
            // Mod-tap
            QmkKeycode::encode_mod_tap(
                qmk_via_api::QmkModMask::from_bits(qmk_via_api::QmkModMask::LCTL),
                0x28, // Enter
            )
            .unwrap(),
            // Layer-tap
            QmkKeycode::encode_layer_tap(1, 0x2C).unwrap(),
            // Layer ops
            QmkLayerOp::Momentary.encode(1).unwrap(),
            QmkLayerOp::Toggle.encode(2).unwrap(),
            QmkLayerOp::To.encode(0).unwrap(),
            QmkLayerOp::OneShot.encode(1).unwrap(),
            QmkLayerOp::Default.encode(0).unwrap(),
            QmkKeycode::encode_layer_mod(
                2,
                qmk_via_api::QmkModMask::from_bits(qmk_via_api::QmkModMask::LALT),
            )
            .unwrap(),
            // One-shot mod
            QmkKeycode::encode_one_shot_mod(qmk_via_api::QmkModMask::from_bits(
                qmk_via_api::QmkModMask::LSFT,
            ))
            .unwrap(),
            // Media
            Keycode::KC_AUDIO_MUTE as u16,
            Keycode::KC_AUDIO_VOL_UP as u16,
            Keycode::KC_MEDIA_PLAY_PAUSE as u16,
            // Mouse
            Keycode::QK_MOUSE_BUTTON_1 as u16,
            // Lighting
            Keycode::QK_BACKLIGHT_TOGGLE as u16,
            Keycode::QK_UNDERGLOW_TOGGLE as u16,
            // Custom / Vendor
            qmk_via_api::ranges::QK_TAP_DANCE.start + 2,
            qmk_via_api::ranges::QK_MACRO.start + 5,
            qmk_via_api::ranges::QK_KB.start + 1,
            qmk_via_api::ranges::QK_USER.start + 3,
            // Unknown hex fallback
            0x5FFF,
        ];

        let empty_layer_names: Vec<String> = vec![];
        for code in test_codes {
            let legacy_layout = crate::qmk_keycode_labels::qmk_to_layout_key(code);
            let spec = qmk_to_keyspec(code);
            let spec_layout = spec.resolve_label(&empty_layer_names);
            if code == Keycode::QK_UNDERGLOW_TOGGLE as u16 {
                // Legacy QMK used "UG Toggle", KeySpec unifies to "RGB Toggle" across protocols
                assert_eq!(spec_layout.unwrap().tap.full, "RGB Toggle");
                continue;
            }
            assert_eq!(
                legacy_layout, spec_layout,
                "Parity mismatch for QMK code 0x{:04X}",
                code
            );
        }
    }
}
