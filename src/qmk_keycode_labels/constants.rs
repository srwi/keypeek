#[allow(unused_imports)]
pub use qmk_via_api::ranges::*;
pub use qmk_via_api::QmkModMask;

#[allow(dead_code)]
pub const MOD_LCTL: u16 = QmkModMask::LCTL as u16;
#[allow(dead_code)]
pub const MOD_LSFT: u16 = QmkModMask::LSFT as u16;
#[allow(dead_code)]
pub const MOD_LALT: u16 = QmkModMask::LALT as u16;
#[allow(dead_code)]
pub const MOD_LGUI: u16 = QmkModMask::LGUI as u16;
/// QMK's "right-hand variant" bit, e.g. `MOD_LALT | MOD_RIGHT_FLAG` is RAlt.
#[allow(dead_code)]
pub const MOD_RIGHT_FLAG: u16 = QmkModMask::RIGHT_HAND as u16;

/// Translate a QMK mod mask into the protocol-agnostic
/// `HELD_MOD_SHIFT`/`HELD_MOD_RALT` flags `LayoutKey::mod_mask` uses.
pub fn to_held_mod_mask(mods: QmkModMask) -> u16 {
    let mut mask = 0;
    if mods.has_shift() {
        mask |= crate::layout_key::HELD_MOD_SHIFT;
    }
    // On macOS left Alt is Option, the same Level-3 shift as RAlt, so it
    // triggers the live preview too. Elsewhere only the right-hand flag does.
    if mods.has_alt() && (cfg!(target_os = "macos") || mods.is_right()) {
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
        assert_eq!(
            to_held_mod_mask(QmkModMask::from_bits(QmkModMask::LSFT)),
            HELD_MOD_SHIFT
        );
        assert_eq!(
            to_held_mod_mask(QmkModMask::from_bits(
                QmkModMask::LALT | QmkModMask::RIGHT_HAND
            )),
            HELD_MOD_RALT
        );
        // Plain (left) Alt is not RAlt on Windows/Linux; the right-hand flag
        // makes it so. On macOS it is Option, the same Level-3 shift.
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(to_held_mod_mask(QmkModMask::from_bits(QmkModMask::LALT)), 0);
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                to_held_mod_mask(QmkModMask::from_bits(QmkModMask::LALT)),
                HELD_MOD_RALT
            );
        }
        assert_eq!(to_held_mod_mask(QmkModMask::from_bits(QmkModMask::LCTL)), 0);
        assert_eq!(to_held_mod_mask(QmkModMask::from_bits(QmkModMask::LGUI)), 0);
    }
}
