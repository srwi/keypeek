use crate::hid_labels::Modifiers;

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
    /// Tap-toggle: tap N times to toggle, hold for momentary (e.g. QMK `TT(layer)`).
    TapToggle,
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
    UnlockKeymap,
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
    BreathingToggle,
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

/// RGB Matrix per-key lighting command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RgbMatrixAction {
    Toggle,
    On,
    Off,
    ModeNext,
    ModePrev,
    HueInc,
    HueDec,
    SatInc,
    SatDec,
    BrightInc,
    BrightDec,
    SpeedInc,
    SpeedDec,
    Other { command: u32, value: u32 },
}

/// Audio synthesizer and clicky command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AudioAction {
    On,
    Off,
    Toggle,
    ClickyToggle,
    ClickyOn,
    ClickyOff,
    ClickyUp,
    ClickyDown,
    ClickyReset,
    MusicOn,
    MusicOff,
    MusicToggle,
    MusicModeNext,
    VoiceNext,
    VoicePrev,
    Other(u32),
}

/// Lighting command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LightingAction {
    Backlight(BacklightAction),
    Rgb(RgbAction),
    RgbMatrix(RgbMatrixAction),
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
    KeyToggle {
        key: HidKey,
        modifiers: Modifiers,
    },
    /// Tap produces a key with optional modifiers, holding activates a layer (e.g. `LT(1, KC_SPC)`).
    LayerTap {
        layer: u8,
        tap: HidKey,
        tap_modifiers: Modifiers,
    },
    /// Tap produces a key with optional modifiers, holding acts as a modifier (e.g. `MT(MOD_LCTL, KC_ENT)`).
    ModTap {
        hold: Modifiers,
        tap: HidKey,
        tap_modifiers: Modifiers,
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
    Audio(AudioAction),
    Mouse(MouseAction),
    /// Vendor/firmware-specific user extensions
    Custom(CustomBinding),
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
    fn test_hid_key_constructors() {
        let k = HidKey::keyboard(0x04);
        assert_eq!(k.page, 0x07);
        assert_eq!(k.id, 0x04);

        let c = HidKey::consumer(0xcd);
        assert_eq!(c.page, 0x0C);
        assert_eq!(c.id, 0xcd);

        let s = HidKey::system(0x81);
        assert_eq!(s.page, 0x01);
        assert_eq!(s.id, 0x81);
    }

    #[test]
    fn test_layer_info_short_name() {
        let unnamed = LayerInfo { id: 1, name: None };
        assert_eq!(unnamed.short_name(1), "L1");

        let named = LayerInfo {
            id: 2,
            name: Some("Nav".to_string()),
        };
        assert_eq!(named.short_name(2), "Nav");
    }

    #[test]
    fn test_keyspec_equality_and_cloning() {
        let key1 = KeySpec::KeyPress {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers::default(),
        };
        let key2 = key1.clone();
        assert_eq!(key1, key2);
    }
}
