/// Normalized 8-bit modifier flags matching standard USB HID Usage Tables (Page 0x07, Usages 0xE0..=0xE7).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub gui: bool,
    pub right_ctrl: bool,
    pub right_shift: bool,
    pub right_alt: bool,
    pub right_gui: bool,
}

impl Modifiers {
    pub const fn is_empty(&self) -> bool {
        !self.ctrl
            && !self.shift
            && !self.alt
            && !self.gui
            && !self.right_ctrl
            && !self.right_shift
            && !self.right_alt
            && !self.right_gui
    }

    pub const fn to_hid_mask(self) -> u8 {
        let mut mask = 0;
        if self.ctrl {
            mask |= 0x01;
        }
        if self.shift {
            mask |= 0x02;
        }
        if self.alt {
            mask |= 0x04;
        }
        if self.gui {
            mask |= 0x08;
        }
        if self.right_ctrl {
            mask |= 0x10;
        }
        if self.right_shift {
            mask |= 0x20;
        }
        if self.right_alt {
            mask |= 0x40;
        }
        if self.right_gui {
            mask |= 0x80;
        }
        mask
    }

    pub const fn from_hid_mask(mask: u8) -> Self {
        Self {
            ctrl: (mask & 0x01) != 0,
            shift: (mask & 0x02) != 0,
            alt: (mask & 0x04) != 0,
            gui: (mask & 0x08) != 0,
            right_ctrl: (mask & 0x10) != 0,
            right_shift: (mask & 0x20) != 0,
            right_alt: (mask & 0x40) != 0,
            right_gui: (mask & 0x80) != 0,
        }
    }
}

impl From<u8> for Modifiers {
    fn from(mask: u8) -> Self {
        Self::from_hid_mask(mask)
    }
}

impl From<Modifiers> for u8 {
    fn from(mods: Modifiers) -> Self {
        mods.to_hid_mask()
    }
}

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
    /// Active only while key is held (for example QMK `MO`, ZMK `&mo`).
    Momentary,
    /// Toggles layer state on each press (for example QMK `TG`, ZMK `&tog`).
    Toggle,
    /// Switches to layer and clears other active layers (for example QMK `TO`, ZMK `&to`).
    To,
    /// Activates layer for the next single keypress, then reverts (for example QMK `OSL`, ZMK `&sl`).
    Sticky,
    /// Momentary layer activation that also applies modifiers (for example QMK `LM(layer, mod)`).
    LayerMod(Modifiers),
    /// Switches the default layer (for example QMK `DF`).
    Default,
    /// Tap to toggle, hold for momentary activation (for example QMK `TT(layer)`).
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

/// Firmware-independent key assignment description.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KeySpec {
    /// Transparent slot that falls through to lower layers.
    Transparent,
    /// Unbound key or no-op.
    None,
    /// Standard key press with optional modifiers (for example `Ctrl+A`).
    KeyPress {
        key: HidKey,
        modifiers: Modifiers,
    },
    /// Key toggle that stays pressed until toggled again.
    KeyToggle {
        key: HidKey,
        modifiers: Modifiers,
    },
    /// Tap sends key with modifiers; hold activates a layer (for example `LT(1, KC_SPC)`).
    LayerTap {
        layer: u8,
        tap: HidKey,
        tap_modifiers: Modifiers,
    },
    /// Tap sends key with modifiers; hold acts as a modifier (for example `MT(MOD_LCTL, KC_ENT)`).
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
    /// One-shot or sticky modifier or key.
    StickyKey {
        key: Option<HidKey>,
        modifiers: Modifiers,
    },
    /// Typing extensions
    CapsWord,
    KeyRepeat,
    GraveEscape,
    /// Hardware and connectivity controls
    Bluetooth(BluetoothAction),
    Output(OutputTarget),
    Power(PowerAction),
    Lighting(LightingAction),
    Audio(AudioAction),
    Mouse(MouseAction),
    /// Vendor-specific custom extensions
    Custom(CustomBinding),
}

/// Identity of one layer as reported by the keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerInfo {
    /// Stable layer id, used by write RPCs. Equals index for QMK.
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

impl KeymapSnapshot {
    /// Updates a single action binding cell at (layer, row, col).
    pub fn set_action(&mut self, layer: usize, row: usize, col: usize, action: Option<KeySpec>) {
        if let Some(layer_actions) = self.actions.get_mut(layer) {
            if let Some(row_actions) = layer_actions.get_mut(row) {
                if let Some(cell) = row_actions.get_mut(col) {
                    *cell = action;
                }
            }
        }
    }
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
