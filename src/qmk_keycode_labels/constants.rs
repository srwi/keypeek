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
pub const QK_KB: Range<u16> = 0x7E00..0x7E40;
pub const QK_USER: Range<u16> = 0x7E40..0x8000;

pub const MOD_LCTL: u16 = 0x01;
pub const MOD_LSFT: u16 = 0x02;
pub const MOD_LALT: u16 = 0x04;
pub const MOD_LGUI: u16 = 0x08;
/// QMK's "right-hand variant" bit, e.g. `MOD_LALT | MOD_RIGHT_FLAG` is RAlt/AltGr.
pub const MOD_RIGHT_FLAG: u16 = 0x10;

/// Translate a raw QMK mod value (bits 0-4, `MOD_L*`/`MOD_RIGHT_FLAG`) into the
/// protocol-agnostic `HELD_MOD_SHIFT`/`HELD_MOD_RALT` flags `LayoutKey::mod_mask` uses.
pub fn to_held_mod_mask(mods: u16) -> u16 {
    let mut mask = 0;
    if mods & MOD_LSFT != 0 {
        mask |= crate::layout_key::HELD_MOD_SHIFT;
    }
    if mods & MOD_LALT != 0 && mods & MOD_RIGHT_FLAG != 0 {
        mask |= crate::layout_key::HELD_MOD_RALT;
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_key::{HELD_MOD_RALT, HELD_MOD_SHIFT};

    #[test]
    fn to_held_mod_mask_distinguishes_shift_and_ralt() {
        assert_eq!(to_held_mod_mask(MOD_LSFT), HELD_MOD_SHIFT);
        assert_eq!(to_held_mod_mask(MOD_LALT | MOD_RIGHT_FLAG), HELD_MOD_RALT);
        // Plain (left) Alt is not RAlt — the right-hand flag is what makes it so.
        assert_eq!(to_held_mod_mask(MOD_LALT), 0);
        assert_eq!(to_held_mod_mask(MOD_LCTL), 0);
        assert_eq!(to_held_mod_mask(MOD_LGUI), 0);
    }
}
