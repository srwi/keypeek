//! Linux backend: reads the compositor's live XKB keymap over Wayland and
//! translates a USB HID usage ID through it. Deliberately never creates a
//! `wl_surface` — this client can never be given keyboard focus, which is
//! correct for an always-on-top overlay that must not steal input from
//! whatever the user is actually typing into. Core `wl_keyboard` protocol
//! delivers the active keymap regardless of focus (verified in practice).
//!
//! X11 isn't covered here (Wayland only) and falls back to `None` like
//! every other unsupported case.

use std::sync::{Arc, Mutex, OnceLock};

use wayland_client::{
    protocol::{wl_keyboard, wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use xkbcommon::xkb;

use super::Modifier;

struct State {
    xkb_context: xkb::Context,
    // Holds the keymap's *text* form, not an `xkb::State`/`xkb::Keymap` —
    // those wrap a raw C pointer that isn't `Send`, so each `resolve()`
    // call parses its own throwaway `State` from this string instead of
    // sharing one across the thread boundary.
    shared: Arc<Mutex<Option<String>>>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        _: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            if interface == "wl_seat" {
                let seat = registry.bind::<wl_seat::WlSeat, _, _>(name, 7, qh, ());
                let _keyboard = seat.get_keyboard(qh, ());
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(_: &mut Self, _: &wl_seat::WlSeat, _: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // The compositor resends this if the user switches layout at
        // runtime, so `shared` always reflects the currently active one.
        if let wl_keyboard::Event::Keymap { fd, size, .. } = event {
            let parsed = unsafe {
                xkb::Keymap::new_from_fd(
                    &state.xkb_context,
                    fd,
                    size as usize,
                    xkb::KEYMAP_FORMAT_TEXT_V1,
                    xkb::KEYMAP_COMPILE_NO_FLAGS,
                )
            };
            if let Ok(Some(keymap)) = parsed {
                let text = keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);
                *state.shared.lock().unwrap() = Some(text);
            }
        }
    }
}

/// USB HID Keyboard/Keypad usage ID -> Linux evdev keycode. Verbatim from
/// the kernel's own `hid_keyboard[256]` table (`drivers/hid/hid-input.c`),
/// not hand-guessed — 0 marks entries the kernel itself leaves unmapped.
#[rustfmt::skip]
const HID_TO_EVDEV: [u8; 256] = [
      0,  0,  0,  0, 30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38,
     50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44,  2,  3,
      4,  5,  6,  7,  8,  9, 10, 11, 28,  1, 14, 15, 57, 12, 13, 26,
     27, 43, 43, 39, 40, 41, 51, 52, 53, 58, 59, 60, 61, 62, 63, 64,
     65, 66, 67, 68, 87, 88, 99, 70,119,110,102,104,111,107,109,106,
    105,108,103, 69, 98, 55, 74, 78, 96, 79, 80, 81, 75, 76, 77, 71,
     72, 73, 82, 83, 86,127,116,117,183,184,185,186,187,188,189,190,
    191,192,193,194,134,138,130,132,128,129,131,137,133,135,136,113,
    115,114,  0,  0,  0,121,  0, 89, 93,124, 92, 94, 95,  0,  0,  0,
    122,123, 90, 91, 85,  0,  0,  0,  0,  0,  0,  0,111,  0,  0,  0,
      0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
      0,  0,  0,  0,  0,  0,179,180,  0,  0,  0,  0,  0,  0,  0,  0,
      0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
      0,  0,  0,  0,  0,  0,  0,  0,111,  0,  0,  0,  0,  0,  0,  0,
     29, 42, 56,125, 97, 54,100,126,164,166,165,163,161,115,114,113,
    150,158,159,128,136,177,178,176,142,152,173,140,  0,  0,  0,  0,
];

fn shared_state() -> &'static Arc<Mutex<Option<String>>> {
    static SHARED: OnceLock<Arc<Mutex<Option<String>>>> = OnceLock::new();
    SHARED.get_or_init(|| {
        let shared = Arc::new(Mutex::new(None));
        let for_thread = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("os-layout-wayland".into())
            .spawn(move || run(for_thread))
            .ok();
        shared
    })
}

fn run(shared: Arc<Mutex<Option<String>>>) {
    let Ok(conn) = Connection::connect_to_env() else {
        return; // Not on Wayland (e.g. X11 session) — resolve() stays None.
    };
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let mut state = State {
        xkb_context: xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
        shared,
    };
    loop {
        if event_queue.blocking_dispatch(&mut state).is_err() {
            return;
        }
    }
}

/// Key labels get resolved once per connect, not every frame, so losing the
/// startup race against this module's own connect-handshake would stick for
/// the whole session rather than just one glance. Bounded wait, not a real
/// stall: the connect is normally done in well under this on a live
/// compositor; if it's never coming (no Wayland at all) every caller pays
/// this once, then the shared state answers instantly from then on.
fn wait_for_keymap_text() -> Option<String> {
    let shared = shared_state();
    for _ in 0..20 {
        if let Some(text) = shared.lock().unwrap().clone() {
            return Some(text);
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    None
}

pub fn resolve(hid_usage: u16, modifier: Modifier) -> Option<String> {
    let evdev = *HID_TO_EVDEV.get(usize::from(hid_usage))?;
    if evdev == 0 {
        return None;
    }
    let keycode = xkb::Keycode::new(u32::from(evdev) + 8);

    let keymap_text = wait_for_keymap_text()?;
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = xkb::Keymap::new_from_string(
        &context,
        keymap_text,
        xkb::KEYMAP_FORMAT_TEXT_V1,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )?;

    let mut probe = xkb::State::new(&keymap);
    if let Modifier::Shift | Modifier::RAlt = modifier {
        let mod_name = match modifier {
            Modifier::Shift => xkb::MOD_NAME_SHIFT,
            // "Mod5" is what RAlt/AltGr is bound to on virtually every layout
            // that has one; ISO_Level3_Shift/"AltGr" are the same virtual
            // modifier under different names depending on the keymap's
            // rules.
            Modifier::RAlt => "Mod5",
            Modifier::Base => unreachable!(),
        };
        let idx = keymap.mod_get_index(mod_name);
        if idx == xkb::MOD_INVALID {
            return None;
        }
        probe.update_mask(1 << idx, 0, 0, 0, 0, 0);
    }
    let text = probe.key_get_utf8(keycode);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve, HID_TO_EVDEV};
    use crate::os_layout::Modifier;

    // Guards against a transcription typo in the table — checked against
    // known-good Linux evdev keycodes (KEY_A=30, KEY_Q=16, KEY_1=2,
    // KEY_MINUS=12), not just "does it compile".
    #[test]
    fn hid_to_evdev_known_values() {
        assert_eq!(HID_TO_EVDEV[0x04], 30); // KEY_A
        assert_eq!(HID_TO_EVDEV[0x14], 16); // KEY_Q
        assert_eq!(HID_TO_EVDEV[0x1E], 2); // KEY_1
        assert_eq!(HID_TO_EVDEV[0x2D], 12); // KEY_MINUS
    }

    // Needs a real Wayland session with a non-US layout active, so it's not
    // part of the normal `cargo test` run — `cargo test -- --ignored` on a
    // machine set to German confirms the live path end to end.
    #[test]
    #[ignore]
    fn live_german_shift_matches_actual_layout() {
        assert_eq!(resolve(0x1F, Modifier::Shift).as_deref(), Some("\"")); // KC_2
        assert_eq!(resolve(0x24, Modifier::Shift).as_deref(), Some("/")); // KC_7
    }
}
