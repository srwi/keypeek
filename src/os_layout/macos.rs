//! macOS backend: resolves a USB HID Keyboard/Keypad usage ID to the character
//! that the active OS keyboard layout produces for it.
//!
//! macOS virtual keycodes (`kVK_*`) are layout-independent physical positions.
//! The physical A position is `kVK_ANSI_A` on every layout. The pipeline is:
//! HID usage -> virtual keycode -> character via `UCKeyTranslate` against the
//! current layout's `'uchr'` data.
//!
//! RAlt on macOS is the **Option** layer. Thus the third legend maps to the
//! option modifier.
//!
//! The system reads the layout on one thread and translates with it on other
//! threads. `TISGetInputSourceProperty` asserts that it runs on the main
//! dispatch queue and stops the process otherwise, but keycap labels are
//! resolved on the connection thread. Thus `init` copies the layout's `'uchr'`
//! table and the hardware keyboard type on the main thread, and `resolve` uses
//! those copies. `UCKeyTranslate` is a pure function over its inputs and has no
//! such limit.

use std::ffi::c_void;
use std::sync::OnceLock;

use super::Modifier;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut c_void;
    fn TISGetInputSourceProperty(src: *mut c_void, key: *const c_void) -> *mut c_void;
    #[allow(non_upper_case_globals)]
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    /// Physical keyboard geometry of the attached hardware (ANSI/ISO/JIS).
    /// `UCKeyTranslate` needs it because some `'uchr'` tables differ per
    /// geometry, notably on JIS boards.
    #[allow(non_snake_case)]
    fn LMGetKbdType() -> i8;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFDataGetBytePtr(the_data: *mut c_void) -> *const u8;
    fn CFDataGetLength(the_data: *mut c_void) -> isize;
    fn CFRelease(cf: *mut c_void);
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    #[allow(non_snake_case)]
    fn UCKeyTranslate(
        key_layout_ptr: *const c_void,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        key_translate_options: u32,
        dead_key_state: *mut u32,
        max_string_length: usize,
        actual_string_length: *mut usize,
        unicode_string: *mut u16,
    ) -> i32;
}

const K_UC_KEY_ACTION_DOWN: u16 = 0;
const K_UC_KEY_TRANSLATE_NO_DEAD_KEYS: u32 = 1 << 5; // 0x20 = kUCKeyTranslateNoDeadKeysMask

// modifierKeyState = (EventRecord.modifiers >> 8) & 0xFF:
//   shiftKey  = 1 << 9  -> bit 1
//   optionKey = 1 << 11 -> bit 3  (RAlt)
const MOD_SHIFT: u32 = 1 << 1;
const MOD_OPTION: u32 = 1 << 3;

// Fallback when `LMGetKbdType()` reports nothing usable. Most layouts'
// `'uchr'` data ignores the keyboard type, so this only affects the few
// geometry-sensitive ones (JIS).
const K_UC_KEYBOARD_TYPE_ANSI: u32 = 0;

/// Marks HID usages that this table does not map. The value cannot be `0`,
/// because `0` is a real keycode (`kVK_ANSI_A`). A 0 marker silently dropped
/// the A key. macOS virtual keycodes end at `0x7E`, thus `0xFF` is free.
const UNMAPPED: u8 = 0xFF;

/// USB HID Keyboard/Keypad usage ID -> macOS virtual keycode (kVK_ANSI_* /
/// layout-independent physical position).
///
/// NUHS (0x32) and BACKSLASH (0x31) use the same virtual keycode,
/// `kVK_ANSI_Backslash`. This is one Apple key. On ANSI boards it sits above
/// Return. On ISO boards it sits left of Return. This is why the key shows
/// `#`/`~` on a British layout.
#[rustfmt::skip]
const HID_TO_VK: [u8; 256] = [
    // 0x00-0x0F  (0x00-0x03 unmapped) A B C D E F G H I J K L
    0xFF,0xFF,0xFF,0xFF,0x00,0x0B,0x08,0x02,0x0E,0x03,0x05,0x04,0x22,0x26,0x28,0x25,
    // 0x10-0x1F  M N O P Q R S T U V W X Y Z 1 2
    0x2E,0x2D,0x1F,0x23,0x0C,0x0F,0x01,0x11,0x20,0x09,0x0D,0x07,0x10,0x06,0x12,0x13,
    // 0x20-0x2F  3 4 5 6 7 8 9 0 Enter Esc Backspace Tab Space - = [
    0x14,0x15,0x17,0x16,0x1A,0x1C,0x19,0x1D,0x24,0x35,0x33,0x30,0x31,0x1B,0x18,0x21,
    // 0x30-0x3F  ] \ NUHS ; ' ` , . / Caps
    0x1E,0x2A,0x2A,0x29,0x27,0x32,0x2B,0x2F,0x2C,0x39,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0x40-0x4F
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0x50-0x5F
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0x60-0x6F  NUBS
    0xFF,0xFF,0xFF,0xFF,0x0A,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0x70-0xDF  (media/function keys — not used for character resolution)
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0xE0-0xEF  LCtrl LShift LAlt LGui RCtrl RShift RAlt RGui
    0x3B,0x38,0x3A,0x37,0x3E,0x3C,0x3D,0x36,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0xF0-0xFF
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
];

fn translate(layout: &[u8], keyboard_type: u32, vk: u16, modifier: Modifier) -> Option<String> {
    unsafe {
        let mods = match modifier {
            Modifier::Base => 0,
            Modifier::Shift => MOD_SHIFT,
            Modifier::RAlt => MOD_OPTION,
        };
        let mut dead_key_state: u32 = 0;
        let mut chars = [0u16; 4];
        let mut actual_length: usize = 0;
        let status = UCKeyTranslate(
            layout.as_ptr() as *const c_void,
            vk,
            K_UC_KEY_ACTION_DOWN,
            mods,
            keyboard_type,
            K_UC_KEY_TRANSLATE_NO_DEAD_KEYS,
            &mut dead_key_state,
            chars.len(),
            &mut actual_length,
            chars.as_mut_ptr(),
        );
        if status == 0 && actual_length > 0 {
            Some(String::from_utf16_lossy(&chars[..actual_length]))
        } else {
            None
        }
    }
}

/// The `'uchr'` table of the active layout plus the hardware keyboard type,
/// copied out of the system. Any thread can read the copies. `Some(None)`
/// means the lookup did not succeed. Callers must then use their static table.
static LAYOUT: OnceLock<Option<(Vec<u8>, u32)>> = OnceLock::new();

/// Copies the `'uchr'` table of the active keyboard layout and the hardware
/// keyboard type. Call this from the main thread only (see the module
/// comment). Later calls do nothing. The copy shows the layout and hardware
/// that were present at startup.
pub fn init() {
    LAYOUT.get_or_init(snapshot);
}

fn snapshot() -> Option<(Vec<u8>, u32)> {
    unsafe {
        // `TISCopy*` returns a reference that we own (+1 retain count). The
        // layout data is a property of the input source. The input source must
        // stay alive until after `UCKeyTranslate` reads the data. Release it
        // last.
        let source = TISCopyCurrentKeyboardLayoutInputSource();
        if source.is_null() {
            return None;
        }
        let data = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        let bytes = if data.is_null() {
            None
        } else {
            let ptr = CFDataGetBytePtr(data);
            let len = CFDataGetLength(data);
            if ptr.is_null() || len <= 0 {
                None
            } else {
                Some(std::slice::from_raw_parts(ptr, len as usize).to_vec())
            }
        };
        CFRelease(source);
        let bytes = bytes?;
        // `LMGetKbdType` returns a signed byte; a non-positive value carries no
        // usable geometry, so fall back to ANSI.
        let keyboard_type = match LMGetKbdType() {
            t if t > 0 => u32::from(t as u8),
            _ => K_UC_KEYBOARD_TYPE_ANSI,
        };
        Some((bytes, keyboard_type))
    }
}

pub fn resolve(hid_usage: u16, modifier: Modifier) -> Option<String> {
    let vk = *HID_TO_VK.get(usize::from(hid_usage))?;
    if vk == UNMAPPED {
        return None;
    }
    let (layout, keyboard_type) = LAYOUT.get()?.as_ref()?;
    translate(layout, *keyboard_type, u16::from(vk), modifier)
}

#[cfg(test)]
mod tests {
    use super::{HID_TO_VK, UNMAPPED};

    // `kVK_ANSI_A` is 0. A 0 marker would make the A key look unmapped.
    #[test]
    fn a_is_mapped_and_distinct_from_the_unmapped_marker() {
        assert_eq!(HID_TO_VK[0x04], 0); // kVK_ANSI_A
        assert_ne!(UNMAPPED, 0);
        assert_eq!(HID_TO_VK[0x00], UNMAPPED); // HID usage 0 is unmapped
    }

    // NUHS and NUBS are different physical keys. They must not resolve to the
    // same virtual keycode. That would give them identical legends.
    #[test]
    fn iso_keys_are_distinct() {
        let nuhs = HID_TO_VK[0x32];
        let nubs = HID_TO_VK[0x64];
        assert_ne!(nuhs, nubs);
        assert_ne!(nuhs, UNMAPPED);
        assert_ne!(nubs, UNMAPPED);
    }

    // Guards against a transcription error. The values are checked against
    // known `kVK_*` values.
    #[test]
    fn hid_to_vk_known_values() {
        assert_eq!(HID_TO_VK[0x14], 0x0C); // Q -> kVK_ANSI_Q
        assert_eq!(HID_TO_VK[0x1E], 0x12); // 1 -> kVK_ANSI_1
        assert_eq!(HID_TO_VK[0x2D], 0x1B); // - -> kVK_ANSI_Minus
        assert_eq!(HID_TO_VK[0x64], 0x0A); // NUBS -> kVK_ISO_Section
    }
}
