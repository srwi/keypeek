use std::ops::Range;

// The constants may be different for protocol versions other than 12:
pub const QK_MODS: Range<u16> = 0x0100..0x2000;
pub const QK_MOD_TAP: Range<u16> = 0x2000..0x4000;
pub const QK_LAYER_TAP: Range<u16> = 0x4000..0x5000;
pub const QK_LAYER_MOD: Range<u16> = 0x5000..0x5200;
pub const QK_TO: Range<u16> = 0x5200..0x5220;
pub const QK_MOMENTARY: Range<u16> = 0x5220..0x5240;
pub const QK_DEF_LAYER: Range<u16> = 0x5240..0x5260;
pub const QK_TOGGLE_LAYER: Range<u16> = 0x5260..0x5280;
pub const QK_ONE_SHOT_LAYER: Range<u16> = 0x5280..0x52A0;
pub const QK_ONE_SHOT_MOD: Range<u16> = 0x52a0..0x52c0;
pub const QK_LAYER_TAP_TOGGLE: Range<u16> = 0x52C0..0x52E0;
pub const QK_TAP_DANCE: Range<u16> = 0x5700..0x5800;
pub const QK_MACRO: Range<u16> = 0x7700..0x7780;
pub const QK_KB: Range<u16> = 0x7E00..0x7F00;

pub const MOD_LCTL: u16 = 0x01;
pub const MOD_LSFT: u16 = 0x02;
pub const MOD_LALT: u16 = 0x04;
pub const MOD_LGUI: u16 = 0x08;
