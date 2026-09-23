//! Resolves the character that the OS's *active* keyboard layout produces for
//! a key, instead of using a static per-language table.
//!
//! QMK `Keycode` values and ZMK `HidUsage` values are both USB HID
//! Keyboard/Keypad-page usage IDs, thus this module needs only that one number
//! and does not know which protocol the keyboard speaks.
//!
//! Each platform has a sibling `#[cfg(target_os = "...")]` backend module:
//! Linux, Windows, and macOS. On unsupported targets, `resolve` returns
//! `None`, and callers use their static table.

#[derive(Clone, Copy)]
pub enum Modifier {
    /// No modifier held: selects the plain base character.
    Base,
    Shift,
    /// Right-Alt (ISO Level 3 Shift on layouts that define one).
    RAlt,
    /// Shift + Right-Alt held together.
    ShiftRAlt,
}

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
pub fn resolve(hid_usage: u16, modifier: Modifier) -> Option<String> {
    linux::resolve(hid_usage, modifier)
}

#[cfg(target_os = "windows")]
pub fn resolve(hid_usage: u16, modifier: Modifier) -> Option<String> {
    windows::resolve(hid_usage, modifier)
}

#[cfg(target_os = "macos")]
pub fn resolve(hid_usage: u16, modifier: Modifier) -> Option<String> {
    macos::resolve(hid_usage, modifier)
}

/// Gives each backend a chance to read data on the main thread before other
/// threads start label resolution. Call this one time from `main`. Backends
/// with no data to prepare do nothing.
pub fn init() {
    #[cfg(target_os = "macos")]
    macos::init();
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn resolve(_hid_usage: u16, _modifier: Modifier) -> Option<String> {
    None
}

/// Helper for the `shifted` legend field.
pub fn shifted_char(hid_usage: u16) -> Option<String> {
    resolve(hid_usage, Modifier::Shift)
}

/// Helper for the `ralt` legend field. A modifier that types the same
/// character as no-modifier has no legend to offer (layouts without an
/// AltGr mapping echo the base character instead of failing).
pub fn ralt_char(hid_usage: u16) -> Option<String> {
    let ralt = resolve(hid_usage, Modifier::RAlt)?;
    (Some(&ralt) != resolve(hid_usage, Modifier::Base).as_ref()).then_some(ralt)
}

/// Helper for the `ralt_shifted` legend field. Same rule as `ralt_char`,
/// checked against both lesser combinations.
pub fn ralt_shifted_char(hid_usage: u16) -> Option<String> {
    let shift_ralt = resolve(hid_usage, Modifier::ShiftRAlt)?;
    let is_distinct = Some(&shift_ralt) != resolve(hid_usage, Modifier::Base).as_ref()
        && Some(&shift_ralt) != resolve(hid_usage, Modifier::Shift).as_ref();
    is_distinct.then_some(shift_ralt)
}

/// Helper for the `tap` label on keys with no sensible US-layout placeholder
/// (ISO-only keys like NUBS/NUHS).
pub fn base_char(hid_usage: u16) -> Option<String> {
    resolve(hid_usage, Modifier::Base)
}
