use crate::layout_key::{Label, LayoutKey};

/// Applies active OS keyboard layout character overrides to standard keyboard keys.
///
/// Letters (usage 0x04..=0x1D) are localized and uppercased for the tap legend,
/// and their RAlt / Shift+RAlt characters are populated for live preview.
///
/// Symbols and digits (keys with a `shifted` legend) have their base and shifted
/// legends updated from the OS layout if available.
pub fn apply_os_overrides(key: &mut LayoutKey, usage_id: u16) {
    // A-Z (HID usage 0x04..=0x1D) move between layouts too: QWERTZ swaps
    // Y and Z with QWERTY, and AZERTY reshuffles nearly the whole row. So
    // `tap` needs the same OS override, just uppercased and without ever
    // touching `shifted` (letters intentionally show no separate legend).
    if (0x04..=0x1D).contains(&usage_id) {
        if let Some(os_base) = crate::os_layout::base_char(usage_id) {
            key.tap = Label::new(os_base.to_uppercase());
        }
        // Some letters carry an RAlt legend too (German RAlt+Q -> "@",
        // +E -> "€"); needed for the Single-legend live preview, even
        // though Dual mode never shows it (letters get no stacked legend).
        key.ralt = crate::os_layout::ralt_char(usage_id);
        key.ralt_shifted = crate::os_layout::ralt_shifted_char(usage_id);
        return;
    }

    // Only *replace* a symbol/digit key's legend; the static table set both
    // `tap` and `shifted` there. Space, Enter, etc. would otherwise pick up a
    // real but useless character from the OS (including literal control chars),
    // so they were deliberately left `None`.
    if key.shifted.is_some() {
        let os_base = crate::os_layout::base_char(usage_id);
        let os_shifted = crate::os_layout::shifted_char(usage_id);

        if let Some(base) = &os_base {
            // A layout can put a real letter on a US-symbol slot (the German
            // semicolon-slot -> "ö"). Render it like a letter (uppercase, no
            // stacked legend) only if Shift does nothing but capitalize it.
            // AZERTY puts accented letters on the digit row instead ("2" key
            // -> base "é", shifted "2"): there Shift produces a different,
            // useful character, so keep the normal Base+Shifted stack.
            let is_mere_capitalization = base.chars().next().is_some_and(char::is_alphabetic)
                && os_shifted
                    .as_deref()
                    .is_none_or(|s| s == base.to_uppercase());
            if is_mere_capitalization {
                key.tap = Label::new(base.to_uppercase());
                key.shifted = None;
                return;
            }
            key.tap = Label::new(base.clone());
        }
        if let Some(shifted) = os_shifted {
            key.shifted = Some(shifted);
        }
        // RAlt's result, for the Single-legend live preview.
        key.ralt = crate::os_layout::ralt_char(usage_id);
        key.ralt_shifted = crate::os_layout::ralt_shifted_char(usage_id);
    }
}
