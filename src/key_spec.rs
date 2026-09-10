use crate::hid_labels::Modifiers;
use crate::layout_key::{behavior_names, BorderStyle, KeycodeKind, Label, LayoutKey};

/// Standard USB HID key reference (page + 16-bit usage ID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HidKey {
    pub page: u16,
    pub id: u16,
}

impl HidKey {
    pub const fn new(page: u16, id: u16) -> Self {
        Self { page, id }
    }

    pub const fn keyboard(id: u16) -> Self {
        Self { page: 0x07, id }
    }

    pub const fn consumer(id: u16) -> Self {
        Self { page: 0x0C, id }
    }

    pub const fn system(id: u16) -> Self {
        Self { page: 0x01, id }
    }
}

/// How a layer activation behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LayerActivation {
    /// Layer is active only while key is held (e.g. QMK `MO`, ZMK `&mo`).
    Momentary,
    /// Key clicks on and off (e.g. QMK `TG`, ZMK `&tog`).
    Toggle,
    /// Switches to layer and turns off other active layers (e.g. QMK `TO`, ZMK `&to`).
    To,
    /// Layer activates for the next single keypress, then reverts (e.g. QMK `OSL`, ZMK `&sl`).
    Sticky,
    /// Layer activates momentarily while applying modifiers (e.g. QMK `LM(layer, mod)`).
    LayerMod(Modifiers),
    /// Default layer switch (e.g. QMK `DF`).
    Default,
}

/// Semantic category of vendor/firmware extension or custom user bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CustomKind {
    User,
    TapDance,
    Macro,
    Keyboard,
    Raw,
}

/// Bluetooth control action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BluetoothAction {
    Clear,
    Next,
    Prev,
    Select(u8),
    ClearAll,
    Disconnect(u8),
    Other { command: u32, value: u32 },
}

/// Output target selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum OutputTarget {
    Toggle,
    Usb,
    Ble,
    None,
    Other(u32),
}

/// Hardware power & boot control action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PowerAction {
    Off,
    On,
    Toggle,
    SoftOff,
    Reset,
    Bootloader,
    StudioUnlock,
    Other(u32),
}

/// Backlight command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BacklightAction {
    On,
    Off,
    Toggle,
    Inc,
    Dec,
    Cycle,
    Set(u8),
    Other { command: u32, value: u32 },
}

/// RGB underglow / lighting command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RgbAction {
    Toggle,
    On,
    Off,
    HueInc,
    HueDec,
    SatInc,
    SatDec,
    BrightInc,
    BrightDec,
    SpeedInc,
    SpeedDec,
    EffectInc,
    EffectDec,
    EffectSet,
    Color,
    Other { command: u32, value: u32 },
}

/// Lighting command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LightingAction {
    Backlight(BacklightAction),
    Rgb(RgbAction),
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Button4,
    Button5,
    Other(u32),
}

/// Mouse motion or button action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MouseAction {
    Press(MouseButton),
    Move { x: i16, y: i16 },
    Scroll { x: i16, y: i16 },
    Acceleration(u8),
}

/// Parameter for custom or vendor-specific bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CustomParam {
    Key(HidKey),
    Layer(u8),
    Number(u32),
}

/// Description of custom or vendor extensions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CustomBinding {
    pub kind: CustomKind,
    pub id: u32,
    pub name: Option<String>,
    pub param1: Option<CustomParam>,
    pub param2: Option<CustomParam>,
}

/// Normalized, firmware-agnostic description of an assigned key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KeySpec {
    /// Transparent slot (falls through to lower layers).
    Transparent,
    /// Unbound key / no-op.
    None,
    /// Standard key press with optional modifiers (e.g. `Ctrl+A`).
    KeyPress {
        key: HidKey,
        modifiers: Modifiers,
    },
    /// Key toggle (locks key in pressed state until toggled again).
    KeyToggle(HidKey),
    /// Tap produces a key, holding activates a layer (e.g. `LT(1, KC_SPC)`).
    LayerTap {
        layer: u8,
        tap: HidKey,
    },
    /// Tap produces a key, holding acts as a modifier (e.g. `MT(MOD_LCTL, KC_ENT)`).
    ModTap {
        hold: Modifiers,
        tap: HidKey,
    },
    /// Layer activation (Momentary, Toggle, To, Sticky, LayerMod, Default).
    Layer {
        layer: u8,
        activation: LayerActivation,
    },
    /// One-shot / sticky modifier or key.
    StickyKey {
        key: Option<HidKey>,
        modifiers: Modifiers,
    },
    /// Typing extensions
    CapsWord,
    KeyRepeat,
    GraveEscape,
    /// Hardware & connectivity controls
    Bluetooth(BluetoothAction),
    Output(OutputTarget),
    Power(PowerAction),
    Lighting(LightingAction),
    Mouse(MouseAction),
    /// Vendor/firmware-specific user extensions
    Custom(CustomBinding),
}

impl KeySpec {
    /// Derives the display `LayoutKey`. `None` = transparent (falls through to lower layers).
    pub fn resolve_label(&self, layer_names: &[String]) -> Option<LayoutKey> {
        match self {
            KeySpec::Transparent => None,

            KeySpec::None => Some(LayoutKey {
                tap: Label::new(""),
                ..Default::default()
            }),

            KeySpec::KeyPress { key, modifiers } => {
                let base = crate::hid_labels::hid_usage_to_layout_key(key.page, key.id);
                if modifiers.is_empty() {
                    base.or_else(|| {
                        // Fallback for custom / vendor codes on unknown pages
                        Some(LayoutKey {
                            tap: Label::new(format!("0x{:04X}", key.id)),
                            ..Default::default()
                        })
                    })
                } else {
                    Some(crate::hid_labels::mod_combo_key(
                        key.page, key.id, *modifiers, base,
                    ))
                }
            }

            KeySpec::KeyToggle(key) => {
                let mut layout = crate::hid_labels::hid_usage_to_layout_key(key.page, key.id)
                    .unwrap_or_else(|| LayoutKey {
                        tap: Label::new(format!("0x{:04X}", key.id)),
                        ..Default::default()
                    });
                layout.behavior = Some(behavior_names::KEY_TOGGLE.label());
                Some(layout)
            }

            KeySpec::LayerTap { layer, tap } => {
                let tap_key = crate::hid_labels::hid_usage_to_layout_key(tap.page, tap.id)
                    .unwrap_or_default();
                Some(crate::hid_labels::layer_tap_key(*layer, tap_key, None))
            }

            KeySpec::ModTap { hold, tap } => {
                let tap_key = crate::hid_labels::hid_usage_to_layout_key(tap.page, tap.id)
                    .unwrap_or_default();
                let mask = hold.to_held_mod_mask();
                Some(crate::hid_labels::mod_tap_key(
                    tap_key,
                    hold.label(),
                    (mask != 0).then_some(mask),
                    Some(behavior_names::MOD_TAP.label()),
                ))
            }

            KeySpec::Layer { layer, activation } => {
                let (border, argument) = match activation {
                    LayerActivation::Momentary => (BorderStyle::None, None),
                    LayerActivation::Toggle | LayerActivation::To => (BorderStyle::Solid, None),
                    LayerActivation::Sticky => (BorderStyle::Dashed, None),
                    LayerActivation::Default => (BorderStyle::Solid, None),
                    LayerActivation::LayerMod(mods) => {
                        (BorderStyle::None, (!mods.is_empty()).then(|| mods.label()))
                    }
                };
                let name_label = layer_names
                    .get(*layer as usize)
                    .filter(|n| !n.is_empty())
                    .map(|n| Label::new(n.as_str()))
                    .unwrap_or_else(|| Label::new(format!("L{}", layer)));
                let mut key = crate::hid_labels::layer_switch_key(*layer, name_label, border);
                if let LayerActivation::LayerMod(mods) = activation {
                    key.argument = argument;
                    let mask = mods.to_held_mod_mask();
                    key.mod_mask = (mask != 0).then_some(mask);
                } else if matches!(activation, LayerActivation::Default) {
                    key.layer_ref = None;
                }
                Some(key)
            }

            KeySpec::StickyKey { key, modifiers } => match key {
                Some(k) => {
                    let mut layout = crate::hid_labels::hid_usage_to_layout_key(k.page, k.id)
                        .unwrap_or_default();
                    layout.behavior = Some(behavior_names::STICKY_KEY.label());
                    layout.kind = KeycodeKind::Modifier;
                    Some(layout)
                }
                None => {
                    let mask = modifiers.to_held_mod_mask();
                    Some(crate::hid_labels::one_shot_mod_key(
                        modifiers.label(),
                        (mask != 0).then_some(mask),
                        Some(behavior_names::ONE_SHOT_MOD.label()),
                    ))
                }
            },

            KeySpec::CapsWord => Some(LayoutKey {
                tap: Label::with_short("Caps Word", "CW"),
                ..Default::default()
            }),

            KeySpec::KeyRepeat => Some(LayoutKey {
                tap: Label::with_short("Key Repeat", "Rep"),
                ..Default::default()
            }),

            KeySpec::GraveEscape => Some(LayoutKey {
                tap: Label::with_short("Grave Esc", "G/E"),
                ..Default::default()
            }),

            KeySpec::Bluetooth(cmd) => {
                let label = match cmd {
                    BluetoothAction::Clear => Label::new("BT Clr"),
                    BluetoothAction::Next => Label::new("BT Nxt"),
                    BluetoothAction::Prev => Label::new("BT Prv"),
                    BluetoothAction::Select(n) => {
                        Label::with_short(format!("BT Sel {n}"), format!("BT{n}"))
                    }
                    BluetoothAction::ClearAll => Label::with_short("BT Clr All", "BTClr"),
                    BluetoothAction::Disconnect(n) => {
                        Label::with_short(format!("BT Disc {n}"), format!("BTD{n}"))
                    }
                    BluetoothAction::Other { command, value: 0 } => {
                        Label::new(format!("BT {command}"))
                    }
                    BluetoothAction::Other { command, value } => {
                        Label::new(format!("BT {command} {value}"))
                    }
                };
                Some(LayoutKey {
                    tap: label,
                    ..Default::default()
                })
            }

            KeySpec::Output(out) => {
                let label = match out {
                    OutputTarget::Toggle => Label::with_short("Out Tog", "OutTg"),
                    OutputTarget::Usb => Label::new("Out USB"),
                    OutputTarget::Ble => Label::new("Out BLE"),
                    OutputTarget::None => Label::with_short("Out None", "OutNo"),
                    OutputTarget::Other(n) => Label::new(format!("Out {n}")),
                };
                Some(LayoutKey {
                    tap: label,
                    ..Default::default()
                })
            }

            KeySpec::Power(pwr) => {
                let label = match pwr {
                    PowerAction::Off => Label::with_short("ExtPwr Off", "EPOff"),
                    PowerAction::On => Label::with_short("ExtPwr On", "EPOn"),
                    PowerAction::Toggle => Label::with_short("ExtPwr Tog", "EPTog"),
                    PowerAction::SoftOff => Label::with_short("Soft Off", "Off"),
                    PowerAction::Reset => Label::with_short("Reset", "Rst"),
                    PowerAction::Bootloader => Label::with_short("Bootloader", "Boot"),
                    PowerAction::StudioUnlock => Label::with_short("Studio Unlock", "Unlock"),
                    PowerAction::Other(n) => {
                        Label::with_short(format!("ExtPwr {n}"), format!("EP{n}"))
                    }
                };
                Some(LayoutKey {
                    tap: label,
                    ..Default::default()
                })
            }

            KeySpec::Lighting(lighting) => {
                let label = match lighting {
                    LightingAction::Backlight(bl) => match bl {
                        BacklightAction::On => Label::new("BL On"),
                        BacklightAction::Off => Label::new("BL Off"),
                        BacklightAction::Toggle => Label::with_short("BL Toggle", "BLTog"),
                        BacklightAction::Inc => Label::with_short("BL Inc", "BL+"),
                        BacklightAction::Dec => Label::with_short("BL Dec", "BL-"),
                        BacklightAction::Cycle => Label::with_short("BL Cycle", "BLCyc"),
                        BacklightAction::Set(n) => {
                            Label::with_short(format!("BL Set {n}"), format!("BL{n}"))
                        }
                        BacklightAction::Other { command, value: 0 } => {
                            Label::new(format!("BL {command}"))
                        }
                        BacklightAction::Other { command, value } => {
                            Label::new(format!("BL {command} {value}"))
                        }
                    },
                    LightingAction::Rgb(ug) => match ug {
                        RgbAction::Toggle => Label::with_short("RGB Toggle", "RGBTg"),
                        RgbAction::On => Label::with_short("RGB On", "RGBOn"),
                        RgbAction::Off => Label::with_short("RGB Off", "RGBOff"),
                        RgbAction::HueInc => Label::with_short("Hue +", "Hue+"),
                        RgbAction::HueDec => Label::with_short("Hue -", "Hue-"),
                        RgbAction::SatInc => Label::with_short("Sat +", "Sat+"),
                        RgbAction::SatDec => Label::with_short("Sat -", "Sat-"),
                        RgbAction::BrightInc => Label::with_short("Bright +", "Bri+"),
                        RgbAction::BrightDec => Label::with_short("Bright -", "Bri-"),
                        RgbAction::SpeedInc => Label::with_short("Speed +", "Spd+"),
                        RgbAction::SpeedDec => Label::with_short("Speed -", "Spd-"),
                        RgbAction::EffectInc => Label::with_short("Effect +", "Eff+"),
                        RgbAction::EffectDec => Label::with_short("Effect -", "Eff-"),
                        RgbAction::EffectSet => Label::with_short("Effect Set", "EffS"),
                        RgbAction::Color => Label::with_short("RGB Color", "Color"),
                        RgbAction::Other { command, value: 0 } => {
                            Label::new(format!("RGB {command}"))
                        }
                        RgbAction::Other { command, value } => {
                            Label::new(format!("RGB {command} {value}"))
                        }
                    },
                };
                Some(LayoutKey {
                    tap: label,
                    ..Default::default()
                })
            }

            KeySpec::Mouse(action) => match action {
                MouseAction::Press(btn) => {
                    let (tap, symbol) = match btn {
                        MouseButton::Left => (
                            Label::new(""),
                            Some(egui_phosphor::regular::MOUSE_LEFT_CLICK.to_string()),
                        ),
                        MouseButton::Right => (
                            Label::new(""),
                            Some(egui_phosphor::regular::MOUSE_RIGHT_CLICK.to_string()),
                        ),
                        MouseButton::Middle => (
                            Label::new(""),
                            Some(egui_phosphor::regular::MOUSE_MIDDLE_CLICK.to_string()),
                        ),
                        MouseButton::Button4 => (Label::new("Mouse Btn4"), None),
                        MouseButton::Button5 => (Label::new("Mouse Btn5"), None),
                        MouseButton::Other(n) => (
                            Label::with_short(format!("Mouse {n}"), format!("M{n}")),
                            None,
                        ),
                    };
                    Some(LayoutKey {
                        tap,
                        symbol,
                        ..Default::default()
                    })
                }
                MouseAction::Move { x, y } => {
                    let (tap, symbol) = match (x.signum(), y.signum()) {
                        (0, -1) => (
                            Label::new(egui_phosphor::regular::ARROW_UP),
                            Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
                        ),
                        (0, 1) => (
                            Label::new(egui_phosphor::regular::ARROW_DOWN),
                            Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
                        ),
                        (-1, 0) => (
                            Label::new(egui_phosphor::regular::ARROW_LEFT),
                            Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
                        ),
                        (1, 0) => (
                            Label::new(egui_phosphor::regular::ARROW_RIGHT),
                            Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
                        ),
                        _ => (
                            Label::with_short(format!("Move ({x}, {y})"), format!("Mv {x},{y}")),
                            None,
                        ),
                    };
                    Some(LayoutKey {
                        tap,
                        symbol,
                        ..Default::default()
                    })
                }
                MouseAction::Scroll { x, y } => {
                    let (tap, symbol) = match (x.signum(), y.signum()) {
                        (0, 1) => (
                            Label::new(egui_phosphor::regular::ARROW_UP),
                            Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
                        ),
                        (0, -1) => (
                            Label::new(egui_phosphor::regular::ARROW_DOWN),
                            Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
                        ),
                        (-1, 0) => (
                            Label::new(egui_phosphor::regular::ARROW_LEFT),
                            Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
                        ),
                        (1, 0) => (
                            Label::new(egui_phosphor::regular::ARROW_RIGHT),
                            Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
                        ),
                        _ => (
                            Label::with_short(format!("Scroll ({x}, {y})"), format!("Scr {x},{y}")),
                            None,
                        ),
                    };
                    Some(LayoutKey {
                        tap,
                        symbol,
                        ..Default::default()
                    })
                }
                MouseAction::Acceleration(n) => Some(LayoutKey {
                    tap: Label::new(format!("Mouse Acc{n}")),
                    ..Default::default()
                }),
            },

            KeySpec::Custom(binding) => {
                // If the custom binding has a string name, use custom behavior formatting
                if let Some(display_name) = &binding.name {
                    return Some(resolve_custom_named_key(
                        display_name,
                        binding.param1,
                        binding.param2,
                        layer_names,
                    ));
                }

                // Check if the ID can be resolved via standard QMK keycodes (e.g. quantum magic / custom keys)
                if matches!(
                    binding.kind,
                    CustomKind::Keyboard | CustomKind::User | CustomKind::Raw
                ) {
                    if let Some(resolved) =
                        crate::qmk_keycode_labels::try_resolve_qmk_key(binding.id as u16)
                    {
                        return Some(resolved);
                    }
                }

                match binding.kind {
                    CustomKind::TapDance => Some(LayoutKey {
                        tap: Label::new(binding.id.to_string()),
                        behavior: Some(behavior_names::TAP_DANCE.label()),
                        ..Default::default()
                    }),
                    CustomKind::Macro => Some(LayoutKey {
                        tap: Label::new(binding.id.to_string()),
                        behavior: Some(behavior_names::MACRO.label()),
                        ..Default::default()
                    }),
                    CustomKind::Keyboard => Some(LayoutKey {
                        tap: Label::new(binding.id.to_string()),
                        behavior: Some(behavior_names::CUSTOM_KB.label()),
                        ..Default::default()
                    }),
                    CustomKind::User => Some(LayoutKey {
                        tap: Label::new(binding.id.to_string()),
                        behavior: Some(behavior_names::CUSTOM_USER.label()),
                        ..Default::default()
                    }),
                    CustomKind::Raw => Some(LayoutKey {
                        tap: Label::new(format!("0x{:04X}", binding.id)),
                        ..Default::default()
                    }),
                }
            }
        }
    }
}

fn resolve_custom_named_key(
    display_name: &str,
    param1: Option<CustomParam>,
    param2: Option<CustomParam>,
    layer_names: &[String],
) -> LayoutKey {
    let name = behavior_label(display_name);

    match (param1, param2) {
        (Some(CustomParam::Key(hold)), Some(CustomParam::Key(tap))) => {
            let hold_key =
                crate::hid_labels::hid_usage_to_layout_key(hold.page, hold.id).unwrap_or_default();
            let tap_key =
                crate::hid_labels::hid_usage_to_layout_key(tap.page, tap.id).unwrap_or_default();
            let hold_label = if hold.page == 0x07 && (0xE0..=0xE7).contains(&hold.id) {
                match hold.id {
                    0xE0 | 0xE4 => Modifiers {
                        ctrl: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE1 | 0xE5 => Modifiers {
                        shift: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE2 => Modifiers {
                        alt: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE6 => Modifiers {
                        alt: true,
                        right_alt: true,
                        ..Default::default()
                    }
                    .label(),
                    0xE3 | 0xE7 => Modifiers {
                        gui: true,
                        ..Default::default()
                    }
                    .label(),
                    _ => hold_key.tap,
                }
            } else if let Some(arg) = hold_key.argument {
                arg
            } else if let Some(sym) = hold_key.symbol {
                Label::new(sym)
            } else {
                hold_key.tap
            };
            crate::hid_labels::mod_tap_key(tap_key, hold_label, hold_key.mod_mask, Some(name))
        }
        (Some(CustomParam::Layer(layer_id)), Some(CustomParam::Key(tap))) => {
            let tap_key =
                crate::hid_labels::hid_usage_to_layout_key(tap.page, tap.id).unwrap_or_default();
            crate::hid_labels::layer_tap_key(layer_id, tap_key, Some(name))
        }
        (Some(CustomParam::Key(key)), None) | (None, Some(CustomParam::Key(key))) => {
            let mut k =
                crate::hid_labels::hid_usage_to_layout_key(key.page, key.id).unwrap_or_default();
            k.behavior = Some(name);
            k
        }
        (Some(CustomParam::Layer(layer_id)), None) | (None, Some(CustomParam::Layer(layer_id))) => {
            let mut k = crate::hid_labels::layer_switch_key(
                layer_id,
                layer_names
                    .get(layer_id as usize)
                    .filter(|n| !n.is_empty())
                    .map(|n| Label::new(n.as_str()))
                    .unwrap_or_else(|| Label::new(format!("L{}", layer_id))),
                BorderStyle::None,
            );
            k.behavior = Some(name);
            k
        }
        (None, None) => LayoutKey {
            tap: name,
            ..Default::default()
        },
        (first, second) => {
            let mut parts = Vec::new();
            for p in [first, second].into_iter().flatten() {
                match p {
                    CustomParam::Key(k) => {
                        let key = crate::hid_labels::hid_usage_to_layout_key(k.page, k.id)
                            .unwrap_or_default();
                        parts.push(key.symbol.unwrap_or(key.tap.full));
                    }
                    CustomParam::Layer(l) => {
                        let l_name = layer_names
                            .get(l as usize)
                            .filter(|n| !n.is_empty())
                            .cloned()
                            .unwrap_or_else(|| format!("L{}", l));
                        parts.push(l_name);
                    }
                    CustomParam::Number(num) => parts.push(num.to_string()),
                }
            }
            LayoutKey {
                tap: name,
                argument: (!parts.is_empty()).then(|| Label::new(parts.join(" "))),
                ..Default::default()
            }
        }
    }
}

fn behavior_label(display_name: &str) -> Label {
    let initials: String = display_name
        .split(|c: char| c == '_' || c == '-' || c.is_whitespace())
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect();

    if initials.chars().count() > 1 {
        Label::with_short(display_name, initials)
    } else {
        Label::new(display_name)
    }
}

/// Identity of one layer as reported by the keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerInfo {
    /// Stable layer id, used by write RPCs. Equals index for QMK/mock.
    pub id: u32,
    /// User-facing layer name.
    pub name: Option<String>,
}

impl LayerInfo {
    pub fn indexed(count: usize) -> Vec<Self> {
        (0..count as u32)
            .map(|id| Self { id, name: None })
            .collect()
    }

    pub fn short_name(&self, index: usize) -> std::borrow::Cow<'_, str> {
        match &self.name {
            Some(name) if !name.is_empty() => std::borrow::Cow::Borrowed(name.as_str()),
            _ => std::borrow::Cow::Owned(format!("L{index}")),
        }
    }
}

/// Everything known about the keymap, bindings included.
#[derive(Clone, Debug, PartialEq)]
pub struct KeymapSnapshot {
    pub layers: Vec<LayerInfo>,
    /// `[layer][row][col]`. `None` = no binding at this position (padding).
    pub actions: Vec<Vec<Vec<Option<KeySpec>>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transparent_resolves_to_none() {
        assert_eq!(KeySpec::Transparent.resolve_label(&[]), None);
    }

    #[test]
    fn test_plain_key_press() {
        let key = KeySpec::KeyPress {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers::default(),
        };
        let label = key.resolve_label(&[]).expect("should resolve");
        assert_eq!(label.tap.full, "A");
    }

    #[test]
    fn test_modified_key_press() {
        let mods = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let key = KeySpec::KeyPress {
            key: HidKey::keyboard(0x06),
            modifiers: mods,
        };
        let label = key.resolve_label(&[]).expect("should resolve");
        assert_eq!(label.tap.full, "C");
        assert_eq!(
            label.argument.as_ref().map(|a| a.full.as_str()),
            Some(crate::layout_key::modifier_symbols::MOD_CTRL.full)
        );
    }

    #[test]
    fn test_layer_tap() {
        let key = KeySpec::LayerTap {
            layer: 2,
            tap: HidKey::keyboard(0x2C), // Space
        };
        let label = key.resolve_label(&[]).expect("should resolve");
        assert_eq!(label.layer_ref, Some(2));
    }
}
