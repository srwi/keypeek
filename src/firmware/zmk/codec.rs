//! Codec translating between ZMK Studio [`Behavior`] and domain [`KeySpec`].

use crate::hid_labels::Modifiers;
use crate::key_spec::{
    BacklightAction, BluetoothAction, CustomBinding, CustomKind, CustomParam, HidKey, KeySpec,
    LayerActivation, LightingAction, MouseAction, MouseButton, OutputTarget, PowerAction,
    RgbAction,
};
use crate::protocols::DeviceError;
use zmk_studio_api::{
    BacklightCommand, Behavior, BehaviorParam, BluetoothCommand, ExternalPowerCommand, HidUsage,
    MouseButton as ZmkMouseButton, OutputSelection, UnderglowCommand,
};

/// Decodes a ZMK Studio [`Behavior`] into a normalized domain [`KeySpec`].
pub fn zmk_to_keyspec(behavior: &Behavior) -> KeySpec {
    match behavior {
        Behavior::Transparent => KeySpec::Transparent,
        Behavior::None => KeySpec::None,
        Behavior::KeyPress(usage) => KeySpec::KeyPress {
            key: HidKey::new(usage.page(), usage.id()),
            modifiers: from_zmk_mask(usage.modifiers()),
        },
        Behavior::KeyToggle(usage) => KeySpec::KeyToggle {
            key: HidKey::new(usage.page(), usage.id()),
            modifiers: from_zmk_mask(usage.modifiers()),
        },
        Behavior::MomentaryLayer { layer_id } => KeySpec::Layer {
            layer: *layer_id as u8,
            activation: LayerActivation::Momentary,
        },
        Behavior::ToggleLayer { layer_id } => KeySpec::Layer {
            layer: *layer_id as u8,
            activation: LayerActivation::Toggle,
        },
        Behavior::ToLayer { layer_id } => KeySpec::Layer {
            layer: *layer_id as u8,
            activation: LayerActivation::To,
        },
        Behavior::StickyLayer { layer_id } => KeySpec::Layer {
            layer: *layer_id as u8,
            activation: LayerActivation::Sticky,
        },
        Behavior::LayerTap { layer_id, tap } => KeySpec::LayerTap {
            layer: *layer_id as u8,
            tap: HidKey::new(tap.page(), tap.id()),
            tap_modifiers: from_zmk_mask(tap.modifiers()),
        },
        Behavior::ModTap { hold, tap } => {
            let mut hold_mods = from_zmk_mask(hold.modifiers());
            if hold.modifier_mask() != 0 {
                hold_mods = from_zmk_mask(hold.modifier_mask());
            }
            KeySpec::ModTap {
                hold: hold_mods,
                tap: HidKey::new(tap.page(), tap.id()),
                tap_modifiers: from_zmk_mask(tap.modifiers()),
            }
        }
        Behavior::StickyKey(usage) => {
            let mut mods = from_zmk_mask(usage.modifiers());
            let key = if usage.id() == 0 {
                None
            } else if (0xE0..=0xE7).contains(&usage.id()) {
                match usage.id() {
                    0xE0 => mods.ctrl = true,
                    0xE1 => mods.shift = true,
                    0xE2 => mods.alt = true,
                    0xE3 => mods.gui = true,
                    0xE4 => mods.right_ctrl = true,
                    0xE5 => mods.right_shift = true,
                    0xE6 => mods.right_alt = true,
                    0xE7 => mods.right_gui = true,
                    _ => {}
                }
                None
            } else {
                Some(HidKey::new(usage.page(), usage.id()))
            };
            KeySpec::StickyKey {
                key,
                modifiers: mods,
            }
        }
        Behavior::CapsWord => KeySpec::CapsWord,
        Behavior::KeyRepeat => KeySpec::KeyRepeat,
        Behavior::Reset => KeySpec::Power(PowerAction::Reset),
        Behavior::Bootloader => KeySpec::Power(PowerAction::Bootloader),
        Behavior::SoftOff => KeySpec::Power(PowerAction::SoftOff),
        Behavior::StudioUnlock => KeySpec::Power(PowerAction::UnlockKeymap),
        Behavior::GraveEscape => KeySpec::GraveEscape,
        Behavior::Bluetooth(cmd) => KeySpec::Bluetooth(bluetooth_to_domain(cmd)),
        Behavior::OutputSelection(out) => KeySpec::Output(output_to_domain(out)),
        Behavior::ExternalPower(ep) => KeySpec::Power(external_power_to_domain(ep)),
        Behavior::Backlight(bl) => {
            KeySpec::Lighting(LightingAction::Backlight(backlight_to_domain(bl)))
        }
        Behavior::Underglow(ug) => KeySpec::Lighting(LightingAction::Rgb(underglow_to_domain(ug))),
        Behavior::MouseKeyPress(btn) => {
            KeySpec::Mouse(MouseAction::Press(mouse_btn_to_domain(btn)))
        }
        Behavior::MouseMove { x, y } => KeySpec::Mouse(MouseAction::Move { x: *x, y: *y }),
        Behavior::MouseScroll { x, y } => KeySpec::Mouse(MouseAction::Scroll { x: *x, y: *y }),
        Behavior::Custom {
            behavior_id,
            display_name,
            param1,
            param2,
        } => KeySpec::Custom(CustomBinding {
            kind: CustomKind::User,
            id: *behavior_id,
            name: Some(display_name.clone()),
            param1: param_to_domain(param1),
            param2: param_to_domain(param2),
        }),
        Behavior::Unknown {
            behavior_id,
            param1,
            param2,
        } => KeySpec::Custom(CustomBinding {
            kind: CustomKind::Raw,
            id: *behavior_id as u32,
            name: None,
            param1: (*param1 != 0).then_some(CustomParam::Number(*param1)),
            param2: (*param2 != 0).then_some(CustomParam::Number(*param2)),
        }),
    }
}

/// Encodes a normalized domain [`KeySpec`] into a ZMK Studio [`Behavior`].
pub fn keyspec_to_zmk(spec: &KeySpec) -> Result<Behavior, DeviceError> {
    match spec {
        KeySpec::Transparent => Ok(Behavior::Transparent),
        KeySpec::None => Ok(Behavior::None),
        KeySpec::KeyPress { key, modifiers } => Ok(Behavior::KeyPress(HidUsage::from_parts(
            key.page,
            key.id,
            to_zmk_mask(*modifiers),
        ))),
        KeySpec::KeyToggle { key, modifiers } => Ok(Behavior::KeyToggle(HidUsage::from_parts(
            key.page,
            key.id,
            to_zmk_mask(*modifiers),
        ))),
        KeySpec::LayerTap {
            layer,
            tap,
            tap_modifiers,
        } => Ok(Behavior::LayerTap {
            layer_id: *layer as u32,
            tap: HidUsage::from_parts(tap.page, tap.id, to_zmk_mask(*tap_modifiers)),
        }),
        KeySpec::ModTap {
            hold,
            tap,
            tap_modifiers,
        } => {
            let mask = to_zmk_mask(*hold);
            let hold_usage = if mask != 0 {
                HidUsage::from_modifier_mask(mask)
            } else {
                HidUsage::from_parts(0x07, 0, 0)
            };
            Ok(Behavior::ModTap {
                hold: hold_usage,
                tap: HidUsage::from_parts(tap.page, tap.id, to_zmk_mask(*tap_modifiers)),
            })
        }
        KeySpec::Layer { layer, activation } => match activation {
            LayerActivation::Momentary => Ok(Behavior::MomentaryLayer {
                layer_id: *layer as u32,
            }),
            LayerActivation::Toggle => Ok(Behavior::ToggleLayer {
                layer_id: *layer as u32,
            }),
            LayerActivation::To => Ok(Behavior::ToLayer {
                layer_id: *layer as u32,
            }),
            LayerActivation::Sticky => Ok(Behavior::StickyLayer {
                layer_id: *layer as u32,
            }),
            LayerActivation::TapToggle => Err(DeviceError::Unsupported(
                "Tap-toggle layer mode not supported in ZMK".to_string(),
            )),
            _ => Err(DeviceError::Unsupported(
                "Layer activation mode not supported on ZMK".to_string(),
            )),
        },
        KeySpec::StickyKey { key, modifiers } => match key {
            None => {
                let mask = to_zmk_mask(*modifiers);
                let usage = if mask != 0 {
                    HidUsage::from_modifier_mask(mask)
                } else {
                    HidUsage::from_parts(0x07, 0, 0)
                };
                Ok(Behavior::StickyKey(usage))
            }
            Some(k) => Ok(Behavior::StickyKey(HidUsage::from_parts(
                k.page,
                k.id,
                to_zmk_mask(*modifiers),
            ))),
        },
        KeySpec::CapsWord => Ok(Behavior::CapsWord),
        KeySpec::KeyRepeat => Ok(Behavior::KeyRepeat),
        KeySpec::GraveEscape => Ok(Behavior::GraveEscape),
        KeySpec::Power(pwr) => match pwr {
            PowerAction::Reset => Ok(Behavior::Reset),
            PowerAction::Bootloader => Ok(Behavior::Bootloader),
            PowerAction::SoftOff => Ok(Behavior::SoftOff),
            PowerAction::UnlockKeymap => Ok(Behavior::StudioUnlock),
            PowerAction::Off => Ok(Behavior::ExternalPower(ExternalPowerCommand::Off)),
            PowerAction::On => Ok(Behavior::ExternalPower(ExternalPowerCommand::On)),
            PowerAction::Toggle => Ok(Behavior::ExternalPower(ExternalPowerCommand::Toggle)),
            PowerAction::Other(n) => Ok(Behavior::ExternalPower(ExternalPowerCommand::Other(*n))),
        },
        KeySpec::Bluetooth(bt) => Ok(Behavior::Bluetooth(domain_to_bluetooth(bt))),
        KeySpec::Output(out) => Ok(Behavior::OutputSelection(domain_to_output(out))),
        KeySpec::Lighting(lighting) => match lighting {
            LightingAction::Backlight(bl) => Ok(Behavior::Backlight(domain_to_backlight(bl)?)),
            LightingAction::Rgb(ug) => Ok(Behavior::Underglow(domain_to_underglow(ug))),
            LightingAction::RgbMatrix(_) => Err(DeviceError::Unsupported(
                "RGB Matrix not supported in ZMK Studio".to_string(),
            )),
        },
        KeySpec::Audio(_) => Err(DeviceError::Unsupported(
            "Audio commands not supported in ZMK".to_string(),
        )),
        KeySpec::Mouse(action) => match action {
            MouseAction::Press(btn) => Ok(Behavior::MouseKeyPress(domain_to_mouse_btn(btn))),
            MouseAction::Move { x, y } => Ok(Behavior::MouseMove { x: *x, y: *y }),
            MouseAction::Scroll { x, y } => Ok(Behavior::MouseScroll { x: *x, y: *y }),
            _ => Err(DeviceError::Unsupported(
                "Mouse acceleration not supported in ZMK".to_string(),
            )),
        },
        KeySpec::Custom(binding) => {
            if let Some(name) = &binding.name {
                Ok(Behavior::Custom {
                    behavior_id: binding.id,
                    display_name: name.clone(),
                    param1: domain_to_param(binding.param1),
                    param2: domain_to_param(binding.param2),
                })
            } else {
                Ok(Behavior::Unknown {
                    behavior_id: binding.id as i32,
                    param1: match binding.param1 {
                        Some(CustomParam::Number(n)) => n,
                        _ => 0,
                    },
                    param2: match binding.param2 {
                        Some(CustomParam::Number(n)) => n,
                        _ => 0,
                    },
                })
            }
        }
    }
}

fn bluetooth_to_domain(cmd: &BluetoothCommand) -> BluetoothAction {
    match cmd {
        BluetoothCommand::Clear => BluetoothAction::Clear,
        BluetoothCommand::Next => BluetoothAction::Next,
        BluetoothCommand::Prev => BluetoothAction::Prev,
        BluetoothCommand::Select(n) => BluetoothAction::Select(*n),
        BluetoothCommand::ClearAll => BluetoothAction::ClearAll,
        BluetoothCommand::Disconnect(n) => BluetoothAction::Disconnect(*n),
        BluetoothCommand::Other { command, value } => BluetoothAction::Other {
            command: *command,
            value: *value,
        },
    }
}

fn domain_to_bluetooth(action: &BluetoothAction) -> BluetoothCommand {
    match action {
        BluetoothAction::Clear => BluetoothCommand::Clear,
        BluetoothAction::Next => BluetoothCommand::Next,
        BluetoothAction::Prev => BluetoothCommand::Prev,
        BluetoothAction::Select(n) => BluetoothCommand::Select(*n),
        BluetoothAction::ClearAll => BluetoothCommand::ClearAll,
        BluetoothAction::Disconnect(n) => BluetoothCommand::Disconnect(*n),
        BluetoothAction::Other { command, value } => BluetoothCommand::Other {
            command: *command,
            value: *value,
        },
    }
}

fn output_to_domain(out: &OutputSelection) -> OutputTarget {
    match out {
        OutputSelection::Toggle => OutputTarget::Toggle,
        OutputSelection::Usb => OutputTarget::Usb,
        OutputSelection::Ble => OutputTarget::Ble,
        OutputSelection::None => OutputTarget::None,
        OutputSelection::Other(n) => OutputTarget::Other(*n),
    }
}

fn domain_to_output(target: &OutputTarget) -> OutputSelection {
    match target {
        OutputTarget::Toggle => OutputSelection::Toggle,
        OutputTarget::Usb => OutputSelection::Usb,
        OutputTarget::Ble => OutputSelection::Ble,
        OutputTarget::None => OutputSelection::None,
        OutputTarget::Other(n) => OutputSelection::Other(*n),
    }
}

fn external_power_to_domain(ep: &ExternalPowerCommand) -> PowerAction {
    match ep {
        ExternalPowerCommand::Off => PowerAction::Off,
        ExternalPowerCommand::On => PowerAction::On,
        ExternalPowerCommand::Toggle => PowerAction::Toggle,
        ExternalPowerCommand::Other(n) => PowerAction::Other(*n),
    }
}

fn backlight_to_domain(bl: &BacklightCommand) -> BacklightAction {
    match bl {
        BacklightCommand::On => BacklightAction::On,
        BacklightCommand::Off => BacklightAction::Off,
        BacklightCommand::Toggle => BacklightAction::Toggle,
        BacklightCommand::Inc => BacklightAction::Inc,
        BacklightCommand::Dec => BacklightAction::Dec,
        BacklightCommand::Cycle => BacklightAction::Cycle,
        BacklightCommand::Set(n) => BacklightAction::Set(*n),
        BacklightCommand::Other { command, value } => BacklightAction::Other {
            command: *command,
            value: *value,
        },
    }
}

fn domain_to_backlight(action: &BacklightAction) -> Result<BacklightCommand, DeviceError> {
    match action {
        BacklightAction::On => Ok(BacklightCommand::On),
        BacklightAction::Off => Ok(BacklightCommand::Off),
        BacklightAction::Toggle => Ok(BacklightCommand::Toggle),
        BacklightAction::Inc => Ok(BacklightCommand::Inc),
        BacklightAction::Dec => Ok(BacklightCommand::Dec),
        BacklightAction::Cycle => Ok(BacklightCommand::Cycle),
        BacklightAction::Set(n) => Ok(BacklightCommand::Set(*n)),
        BacklightAction::Other { command, value } => Ok(BacklightCommand::Other {
            command: *command,
            value: *value,
        }),
        BacklightAction::BreathingToggle => Err(DeviceError::Unsupported(
            "Backlight breathing toggle not supported in ZMK".to_string(),
        )),
    }
}

fn underglow_to_domain(ug: &UnderglowCommand) -> RgbAction {
    match ug {
        UnderglowCommand::Toggle => RgbAction::Toggle,
        UnderglowCommand::On => RgbAction::On,
        UnderglowCommand::Off => RgbAction::Off,
        UnderglowCommand::HueInc => RgbAction::HueInc,
        UnderglowCommand::HueDec => RgbAction::HueDec,
        UnderglowCommand::SatInc => RgbAction::SatInc,
        UnderglowCommand::SatDec => RgbAction::SatDec,
        UnderglowCommand::BrightInc => RgbAction::BrightInc,
        UnderglowCommand::BrightDec => RgbAction::BrightDec,
        UnderglowCommand::SpeedInc => RgbAction::SpeedInc,
        UnderglowCommand::SpeedDec => RgbAction::SpeedDec,
        UnderglowCommand::EffectInc => RgbAction::EffectInc,
        UnderglowCommand::EffectDec => RgbAction::EffectDec,
        UnderglowCommand::EffectSet => RgbAction::EffectSet,
        UnderglowCommand::Color => RgbAction::Color,
        UnderglowCommand::Other { command, value } => RgbAction::Other {
            command: *command,
            value: *value,
        },
    }
}

fn domain_to_underglow(action: &RgbAction) -> UnderglowCommand {
    match action {
        RgbAction::Toggle => UnderglowCommand::Toggle,
        RgbAction::On => UnderglowCommand::On,
        RgbAction::Off => UnderglowCommand::Off,
        RgbAction::HueInc => UnderglowCommand::HueInc,
        RgbAction::HueDec => UnderglowCommand::HueDec,
        RgbAction::SatInc => UnderglowCommand::SatInc,
        RgbAction::SatDec => UnderglowCommand::SatDec,
        RgbAction::BrightInc => UnderglowCommand::BrightInc,
        RgbAction::BrightDec => UnderglowCommand::BrightDec,
        RgbAction::SpeedInc => UnderglowCommand::SpeedInc,
        RgbAction::SpeedDec => UnderglowCommand::SpeedDec,
        RgbAction::EffectInc => UnderglowCommand::EffectInc,
        RgbAction::EffectDec => UnderglowCommand::EffectDec,
        RgbAction::EffectSet => UnderglowCommand::EffectSet,
        RgbAction::Color => UnderglowCommand::Color,
        RgbAction::Other { command, value } => UnderglowCommand::Other {
            command: *command,
            value: *value,
        },
    }
}

fn mouse_btn_to_domain(btn: &ZmkMouseButton) -> MouseButton {
    match btn {
        ZmkMouseButton::Left => MouseButton::Left,
        ZmkMouseButton::Right => MouseButton::Right,
        ZmkMouseButton::Middle => MouseButton::Middle,
        ZmkMouseButton::Button4 => MouseButton::Button4,
        ZmkMouseButton::Button5 => MouseButton::Button5,
        ZmkMouseButton::Other(n) => MouseButton::Other(*n),
    }
}

fn domain_to_mouse_btn(btn: &MouseButton) -> ZmkMouseButton {
    match btn {
        MouseButton::Left => ZmkMouseButton::Left,
        MouseButton::Right => ZmkMouseButton::Right,
        MouseButton::Middle => ZmkMouseButton::Middle,
        MouseButton::Button4 => ZmkMouseButton::Button4,
        MouseButton::Button5 => ZmkMouseButton::Button5,
        MouseButton::Other(n) => ZmkMouseButton::Other(*n),
    }
}

fn param_to_domain(param: &BehaviorParam) -> Option<CustomParam> {
    match param {
        BehaviorParam::Unused => None,
        BehaviorParam::Keycode(usage) => {
            let id = if usage.id() == 0 && usage.modifiers() != 0 {
                match usage.modifiers() {
                    zmk_studio_api::MOD_LCTL => 0xE0,
                    zmk_studio_api::MOD_LSFT => 0xE1,
                    zmk_studio_api::MOD_LALT => 0xE2,
                    zmk_studio_api::MOD_LGUI => 0xE3,
                    zmk_studio_api::MOD_RCTL => 0xE4,
                    zmk_studio_api::MOD_RSFT => 0xE5,
                    zmk_studio_api::MOD_RALT => 0xE6,
                    zmk_studio_api::MOD_RGUI => 0xE7,
                    _ => 0,
                }
            } else {
                usage.id()
            };
            Some(CustomParam::Key(HidKey::new(usage.page(), id)))
        }
        BehaviorParam::LayerId(layer_id) => Some(CustomParam::Layer(*layer_id as u8)),
        BehaviorParam::Number(n) => Some(CustomParam::Number(*n)),
    }
}

fn domain_to_param(param: Option<CustomParam>) -> BehaviorParam {
    match param {
        None => BehaviorParam::Unused,
        Some(CustomParam::Key(key)) => {
            let usage = match (key.page, key.id) {
                (0x07, 0xE0) => HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_LCTL),
                (0x07, 0xE1) => HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_LSFT),
                (0x07, 0xE2) => HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_LALT),
                (0x07, 0xE3) => HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_LGUI),
                (0x07, 0xE4) => HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_RCTL),
                (0x07, 0xE5) => HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_RSFT),
                (0x07, 0xE6) => HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_RALT),
                (0x07, 0xE7) => HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_RGUI),
                _ => HidUsage::from_parts(key.page, key.id, 0),
            };
            BehaviorParam::Keycode(usage)
        }
        Some(CustomParam::Layer(layer)) => BehaviorParam::LayerId(layer as u32),
        Some(CustomParam::Number(n)) => BehaviorParam::Number(n),
    }
}

/// Translates a ZMK modifier bitmask into a domain [`Modifiers`] struct.
pub fn from_zmk_mask(mods: u8) -> Modifiers {
    Modifiers::from_hid_mask(mods)
}

/// Translates a domain [`Modifiers`] struct into a ZMK modifier bitmask.
pub fn to_zmk_mask(mods: Modifiers) -> u8 {
    mods.to_hid_mask()
}

/// Returns ZMK keycode search tokens for a given standard HID usage.
pub fn zmk_search_tokens_for_hid(page: u16, id: u16) -> Vec<String> {
    let mut tokens = Vec::new();
    let usage = HidUsage::from_parts(page, id, 0);
    let encoded = usage.to_hid_usage();
    tokens.push(format!("{:08x}", encoded));
    if let Ok(kc) = zmk_studio_api::Keycode::try_from(encoded) {
        tokens.push(kc.as_ref().to_string());
        let name = kc.to_name();
        if !name.is_empty() && name != kc.as_ref() {
            tokens.push(name.to_string());
        }
    }
    tokens
}

/// Enumerates all USB HID keyboard usage IDs in ZMK Studio.
pub fn zmk_all_keyboard_usages() -> Vec<u16> {
    zmk_studio_api::Keycode::all_keyboard()
        .iter()
        .map(|k| {
            let usage = HidUsage::from_encoded(k.to_hid_usage());
            usage.id()
        })
        .collect()
}

/// Enumerates all USB HID consumer usage IDs in ZMK Studio.
pub fn zmk_all_consumer_usages() -> Vec<u16> {
    zmk_studio_api::Keycode::all_consumer()
        .iter()
        .map(|k| {
            let usage = HidUsage::from_encoded(k.to_hid_usage());
            usage.id()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zmk_round_trips() {
        let behaviors = [
            Behavior::Transparent,
            Behavior::None,
            Behavior::KeyPress(HidUsage::from_parts(0x07, 0x04, 0)),
            Behavior::KeyPress(HidUsage::from_parts(0x07, 0x04, zmk_studio_api::MOD_LCTL)),
            Behavior::MomentaryLayer { layer_id: 2 },
            Behavior::ToggleLayer { layer_id: 3 },
            Behavior::CapsWord,
            Behavior::KeyRepeat,
            Behavior::Reset,
            Behavior::Bootloader,
            Behavior::Bluetooth(BluetoothCommand::Select(1)),
            Behavior::OutputSelection(OutputSelection::Usb),
            Behavior::MouseKeyPress(ZmkMouseButton::Left),
            Behavior::MouseMove { x: 0, y: -1 },
            Behavior::LayerTap {
                layer_id: 2,
                tap: HidUsage::from_parts(0x07, 0x1C, zmk_studio_api::MOD_LSFT),
            },
            Behavior::KeyToggle(HidUsage::from_parts(0x07, 0x05, 0)),
            Behavior::KeyToggle(HidUsage::from_parts(0x07, 0x05, zmk_studio_api::MOD_LSFT)),
            Behavior::StickyKey(HidUsage::from_parts(0x07, 0x04, 0)),
            Behavior::StickyKey(HidUsage::from_parts(0x07, 0x04, zmk_studio_api::MOD_LCTL)),
            Behavior::StickyKey(HidUsage::from_modifier_mask(zmk_studio_api::MOD_LSFT)),
            Behavior::ModTap {
                hold: HidUsage::from_modifier_mask(zmk_studio_api::MOD_LALT),
                tap: HidUsage::from_parts(0x07, 0x06, zmk_studio_api::MOD_LCTL),
            },
        ];

        for b in behaviors {
            let spec = zmk_to_keyspec(&b);
            let round_trip = keyspec_to_zmk(&spec).expect("should encode");
            assert_eq!(b, round_trip);
        }
    }

    #[test]
    fn test_zmk_display_fidelity_parity() {
        let layer_names = vec!["Base".to_string(), "Nav".to_string(), "Sym".to_string()];
        let test_behaviors = [
            Behavior::Transparent,
            Behavior::None,
            Behavior::KeyPress(HidUsage::from_parts(0x07, 0x04, 0)),
            Behavior::KeyPress(HidUsage::from_parts(0x07, 0x04, zmk_studio_api::MOD_LCTL)),
            Behavior::KeyPress(HidUsage::from_parts(
                0x07,
                0x04,
                zmk_studio_api::MOD_LCTL | zmk_studio_api::MOD_LSFT,
            )),
            Behavior::KeyToggle(HidUsage::from_parts(0x07, 0x05, 0)),
            Behavior::MomentaryLayer { layer_id: 1 },
            Behavior::ToggleLayer { layer_id: 2 },
            Behavior::ToLayer { layer_id: 0 },
            Behavior::StickyLayer { layer_id: 1 },
            Behavior::LayerTap {
                layer_id: 1,
                tap: HidUsage::from_parts(0x07, 0x2C, 0),
            },
            Behavior::ModTap {
                hold: HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_LCTL),
                tap: HidUsage::from_parts(0x07, 0x04, 0),
            },
            Behavior::StickyKey(HidUsage::from_parts(0x07, 0, zmk_studio_api::MOD_LSFT)),
            Behavior::CapsWord,
            Behavior::KeyRepeat,
            Behavior::Reset,
            Behavior::Bootloader,
            Behavior::SoftOff,
            Behavior::StudioUnlock,
            Behavior::GraveEscape,
            Behavior::Bluetooth(BluetoothCommand::Clear),
            Behavior::Bluetooth(BluetoothCommand::Select(2)),
            Behavior::Bluetooth(BluetoothCommand::Disconnect(1)),
            Behavior::OutputSelection(OutputSelection::Ble),
            Behavior::ExternalPower(ExternalPowerCommand::Toggle),
            Behavior::Backlight(BacklightCommand::Toggle),
            Behavior::Backlight(BacklightCommand::Set(5)),
            Behavior::Underglow(UnderglowCommand::Toggle),
            Behavior::Underglow(UnderglowCommand::HueInc),
            Behavior::MouseKeyPress(ZmkMouseButton::Left),
            Behavior::MouseMove { x: 0, y: -1 },
            Behavior::MouseScroll { x: 1, y: 0 },
            Behavior::Custom {
                behavior_id: 1,
                display_name: "home_row_mod".to_string(),
                param1: BehaviorParam::Keycode(HidUsage::from_parts(
                    0x07,
                    0,
                    zmk_studio_api::MOD_LGUI,
                )),
                param2: BehaviorParam::Keycode(HidUsage::from_parts(0x07, 0x04, 0)),
            },
        ];

        for b in &test_behaviors {
            if matches!(b, Behavior::StickyKey(_)) {
                // StickyKey modifier-only was previously rendered with empty tap and argument in legacy ZMK;
                // KeySpec normalizes it to standard one-shot mod with tap glyph and proper mod_mask.
                continue;
            }
            let legacy_layout =
                super::super::keycode_labels::behavior_to_layout_key(b, &layer_names);
            let spec = zmk_to_keyspec(b);
            use crate::key_presenter::KeyPresenter;
            let presenter = crate::firmware::zmk::ZmkKeyPresenter;
            let spec_layout = presenter.present_key(&spec, &layer_names);
            assert_eq!(
                legacy_layout, spec_layout,
                "Parity mismatch for behavior {:?}",
                b
            );
        }
    }

    #[test]
    fn test_zmk_unsupported_features_properly_rejected() {
        let tt_spec = KeySpec::Layer {
            layer: 1,
            activation: LayerActivation::TapToggle,
        };
        assert!(matches!(
            keyspec_to_zmk(&tt_spec),
            Err(DeviceError::Unsupported(_))
        ));

        let bl_spec =
            KeySpec::Lighting(LightingAction::Backlight(BacklightAction::BreathingToggle));
        assert!(matches!(
            keyspec_to_zmk(&bl_spec),
            Err(DeviceError::Unsupported(_))
        ));

        let accel_spec = KeySpec::Mouse(MouseAction::Acceleration(1));
        assert!(matches!(
            keyspec_to_zmk(&accel_spec),
            Err(DeviceError::Unsupported(_))
        ));
    }

    #[test]
    fn test_zmk_modifiers_round_trip() {
        for mask in 0..=255u8 {
            let mods = from_zmk_mask(mask);
            assert_eq!(to_zmk_mask(mods), mask);
        }
    }
}
