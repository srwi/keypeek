//! Windows backend: resolves a USB HID Keyboard/Keypad usage ID to the
//! character that the *active* OS keyboard layout produces for it.
//!
//! Pipeline: HID usage -> scan code (layout-independent physical position) ->
//! VK via `MapVirtualKeyEx` (layout-dependent) -> character via `ToUnicodeEx`.
//!
//! We read the layout of the *foreground* thread (the thread that actually
//! receives input). This honors per-app keyboard layouts. It also avoids the
//! known UWP/Edge `ApplicationFrameHost` stale-layout bug that breaks the
//! naive `GetForegroundWindow()` approach (see `GetGUIThreadInfo` below).

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, MapVirtualKeyExW, ToUnicodeEx, HKL, MAPVK_VSC_TO_VK_EX, VK_CONTROL, VK_MENU,
    VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
};

use super::Modifier;

/// Marks HID usages that this table does not map. `0xFF` collides with no
/// real PS/2 set-1 make code in this table (the highest mapped one is `0x58`).
const UNMAPPED: u8 = 0xFF;

/// USB HID Keyboard/Keypad usage ID -> Windows PS/2 (set 1) scan code.
/// Scan codes are layout-independent physical positions. The alphanumeric
/// block matches the kernel's evdev table, but the codes are different, thus
/// this is its own table. 0xFF marks unmapped entries.
///
/// NUHS (0x32) has the same scan code as BACKSLASH (0x31). The kernel's table
/// also puts both at `KEY_BACKSLASH`. This is one physical key. ISO boards put
/// it left of Return. ANSI boards put it above Return. NUBS (0x64) is an extra
/// key on ISO boards. It gets its own scan code, the 102nd-key code.
///
/// Extended keys are arrows, the navigation cluster, KP/, KP-Enter, and the
/// right-hand modifiers. They need a two-byte scan code with the prefix `0xE0`.
/// A `u8` cannot hold such a code. These keys produce no characters, thus the
/// table does not map them. `resolve` returns `None`, and the caller keeps its
/// static label.
#[rustfmt::skip]
const HID_TO_SCAN: [u8; 256] = [
    // 0x00-0x0F  (0x00-0x03 unmapped) A B C D E F G H I J K L
    0xFF,0xFF,0xFF,0xFF,0x1E,0x30,0x2E,0x20,0x12,0x21,0x22,0x23,0x17,0x24,0x25,0x26,
    // 0x10-0x1F  M N O P Q R S T U V W X Y Z 1 2
    0x32,0x31,0x18,0x19,0x10,0x13,0x1F,0x14,0x16,0x2F,0x11,0x2D,0x15,0x2C,0x02,0x03,
    // 0x20-0x2F  3 4 5 6 7 8 9 0 Enter Esc BSpace Tab Space - = [
    0x04,0x05,0x06,0x07,0x08,0x09,0x0A,0x0B,0x1C,0x01,0x0E,0x0F,0x39,0x0C,0x0D,0x1A,
    // 0x30-0x3F  ] \ NUHS ; ' ` , . / Caps F1..F6
    0x1B,0x2B,0x2B,0x27,0x28,0x29,0x33,0x34,0x35,0x3A,0x3B,0x3C,0x3D,0x3E,0x3F,0x40,
    // 0x40-0x4F  F7..F12 PrtScr Scroll Pause Ins Home PgUp Del End PgDn Right
    //             (Ins..Right are extended-only keys)
    0x41,0x42,0x43,0x44,0x57,0x58,0xFF,0x46,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0x50-0x5F  Left Down Up NumLock KP/ KP* KP- KP+ KPEnter KP1..KP7
    0xFF,0xFF,0xFF,0x45,0xFF,0x37,0x4A,0x4E,0xFF,0x4F,0x50,0x51,0x4B,0x4C,0x4D,0x47,
    // 0x60-0x6F  KP8 KP9 KP0 KP. NUBS
    0x48,0x49,0x52,0x53,0x56,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0x70-0xDF  (unmapped)
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0xE0-0xEF  LCtrl LShift LAlt LGui RCtrl RShift RAlt RGui
    0x1D,0x2A,0x38,0xFF,0xFF,0x36,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
    // 0xF0-0xFF  (unmapped)
    0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
];

/// The foreground thread that actually receives input. `GetForegroundWindow`
/// can return a host frame (UWP/Edge) whose thread has a stale layout.
/// `GetGUIThreadInfo` with thread ID 0 returns the thread that receives the
/// user input, which is the correct answer to "what would this key type now".
fn foreground_hkl() -> HKL {
    unsafe {
        let mut gti = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        // GetGUIThreadInfo(0, ...) returns the info of the foreground thread.
        if GetGUIThreadInfo(0, &mut gti).is_ok() && !gti.hwndFocus.0.is_null() {
            let tid = GetWindowThreadProcessId(gti.hwndFocus, None);
            let hkl = GetKeyboardLayout(tid);
            if !hkl.is_invalid() {
                return hkl;
            }
        }
        GetKeyboardLayout(0)
    }
}

fn char_for(hkl: HKL, vk: u32, scan: u32, modifier: Modifier) -> Option<String> {
    unsafe {
        let mut state = [0u8; 256];
        match modifier {
            Modifier::Base => {}
            // RAlt on Windows is Left Ctrl + Right Alt. ToUnicodeEx honors it
            // only when the generic VK_CONTROL/VK_MENU bits are set. The
            // left/right-specific VKs alone resolve the base character
            // (verified on a German layout).
            Modifier::Shift => state[VK_SHIFT.0 as usize] = 0x80,
            Modifier::RAlt => {
                state[VK_CONTROL.0 as usize] = 0x80;
                state[VK_MENU.0 as usize] = 0x80;
            }
        }
        let mut buf = [0u16; 4];
        // Flag 0x4 = KEYBOARD_STATE_NOT_CHANGED: this call must not change the
        // kernel's dead-key buffer. Concurrent calls stay safe, and a dead key
        // does not leak into the next call.
        let n = ToUnicodeEx(vk, scan, &state, &mut buf, 0x4, Some(hkl));
        if n > 0 {
            Some(String::from_utf16_lossy(&buf[..n as usize]))
        } else {
            None
        }
    }
}

pub fn resolve(hid_usage: u16, modifier: Modifier) -> Option<String> {
    let scan = *HID_TO_SCAN.get(usize::from(hid_usage))?;
    if scan == UNMAPPED {
        return None;
    }
    let hkl = foreground_hkl();
    // Scan -> VK is layout-dependent. The A position is VK_A on a US layout
    // but VK_Q on a French layout. Thus this step must use the active layout's
    // HKL.
    let vk = unsafe { MapVirtualKeyExW(scan as u32, MAPVK_VSC_TO_VK_EX, Some(hkl)) };
    if vk == 0 {
        return None;
    }
    char_for(hkl, vk, scan as u32, modifier)
}

#[cfg(test)]
mod tests {
    use super::{HID_TO_SCAN, UNMAPPED};

    #[test]
    fn unmapped_marker_is_distinct_from_real_scan_codes() {
        assert_ne!(UNMAPPED, 0);
        assert_eq!(HID_TO_SCAN[0x00], UNMAPPED); // HID usage 0 is unmapped
        assert_eq!(HID_TO_SCAN[0x04], 0x1E); // A stays mapped
    }

    // NUHS is the same physical key as BACKSLASH. NUBS is the extra ISO key,
    // thus it must have its own scan code. Otherwise the two render identically.
    #[test]
    fn iso_keys_follow_the_kernel_pattern() {
        assert_eq!(HID_TO_SCAN[0x32], HID_TO_SCAN[0x31]); // NUHS == BACKSLASH
        assert_eq!(HID_TO_SCAN[0x64], 0x56); // NUBS -> 102nd key
        assert_ne!(HID_TO_SCAN[0x64], HID_TO_SCAN[0x32]);
    }

    // This test guards against a transcription error. The values are checked
    // against known scan codes.
    #[test]
    fn hid_to_scan_known_values() {
        assert_eq!(HID_TO_SCAN[0x04], 0x1E); // A
        assert_eq!(HID_TO_SCAN[0x14], 0x10); // Q
        assert_eq!(HID_TO_SCAN[0x1E], 0x02); // 1
        assert_eq!(HID_TO_SCAN[0x2D], 0x0C); // minus
    }
}
