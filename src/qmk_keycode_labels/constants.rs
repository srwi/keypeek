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
pub const QK_MACRO: Range<u16> = 0x7700..0x7780;
pub const QK_KB: Range<u16> = 0x7E00..0x7F00;

pub const QK_LCTL: u16 = 0x0100;
pub const QK_LSFT: u16 = 0x0200;
pub const QK_LALT: u16 = 0x0400;
pub const QK_LGUI: u16 = 0x0800;
pub const QK_RMODS_MIN: u16 = 0x1000;
pub const QK_RCTL: u16 = 0x1100;
pub const QK_RSFT: u16 = 0x1200;
pub const QK_RALT: u16 = 0x1400;
pub const QK_RGUI: u16 = 0x1800;

pub const MODIFIER_KEY_TO_VALUE: &[(&str, u16)] = &[
    ("LCtl", QK_LCTL),
    ("LCTL", QK_LCTL),
    ("C", QK_LCTL),
    ("LSft", QK_LSFT),
    ("LSFT", QK_LSFT),
    ("S", QK_LSFT),
    ("LAlt", QK_LALT),
    ("LALT", QK_LALT),
    ("A", QK_LALT),
    ("LGui", QK_LGUI),
    ("LGUI", QK_LGUI),
    ("LCMD", QK_LGUI),
    ("LWIN", QK_LGUI),
    ("G", QK_LGUI),
    ("RCtl", QK_RCTL),
    ("RCTL", QK_RCTL),
    ("RSft", QK_RSFT),
    ("RSFT", QK_RSFT),
    ("RAlt", QK_RALT),
    ("ALGR", QK_RALT),
    ("RALT", QK_RALT),
    ("RGui", QK_RGUI),
    ("RCMD", QK_RGUI),
    ("RWIN", QK_RGUI),
    ("RGUI", QK_RGUI),
    ("SCMD", QK_LSFT | QK_LGUI),
    ("SWIN", QK_LSFT | QK_LGUI),
    ("SGUI", QK_LSFT | QK_LGUI),
    ("LSG", QK_LSFT | QK_LGUI),
    ("LAG", QK_LALT | QK_LGUI),
    ("RSG", QK_RSFT | QK_RGUI),
    ("RAG", QK_RALT | QK_RGUI),
    ("LCA", QK_LCTL | QK_LALT),
    ("LSA", QK_LSFT | QK_LALT),
    ("SAGR", QK_RSFT | QK_RALT),
    ("RSA", QK_RSFT | QK_RALT),
    ("RCS", QK_RCTL | QK_RSFT),
    ("LCAG", QK_LCTL | QK_LALT | QK_LGUI),
    ("MEH", QK_LCTL | QK_LALT | QK_LSFT),
    ("HYPR", QK_LCTL | QK_LALT | QK_LSFT | QK_LGUI),
];
