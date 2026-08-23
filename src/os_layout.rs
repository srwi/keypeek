//! Resolves the character the OS's *active* keyboard layout produces for a
//! given key, instead of guessing from a static per-language table.
//!
//! Both QMK's `Keycode` and ZMK's `HidUsage` are numerically USB HID
//! Keyboard/Keypad-page usage IDs, so this module only needs that one
//! number — it doesn't know or care which protocol the keyboard speaks.
//!
//! Linux/Wayland only for now (other platforms return `None`, so callers
//! fall back to their existing static table). Windows/macOS/X11 are
//! comparable-effort follow-ups; add them as sibling `#[cfg(target_os =
//! "...")]` modules here without touching this one.

#[derive(Clone, Copy)]
pub enum Modifier {
    /// No modifier held — the plain/base character.
    Base,
    Shift,
    /// Right-Alt (ISO Level 3 Shift on layouts that define one).
    RAlt,
}

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub fn resolve(hid_usage: u16, modifier: Modifier) -> Option<String> {
    linux::resolve(hid_usage, modifier)
}

#[cfg(not(target_os = "linux"))]
pub fn resolve(_hid_usage: u16, _modifier: Modifier) -> Option<String> {
    None
}

/// Convenience for the `shifted` legend field.
pub fn shifted_char(hid_usage: u16) -> Option<String> {
    resolve(hid_usage, Modifier::Shift)
}

/// Convenience for the `tap` label on keys with no sensible US-layout
/// placeholder to fall back to (ISO-only keys like NUBS/NUHS).
pub fn base_char(hid_usage: u16) -> Option<String> {
    resolve(hid_usage, Modifier::Base)
}
