//! Linux backend: resolves a USB HID Keyboard/Keypad usage ID through the
//! XKB keymap of the session's current keyboard layout. Two keymap sources
//! exist:
//!
//! - **X11** (`resolve_x11` below): a keymap compiled from the X server's
//!   active RMLVO configuration (read from the `_XKB_RULES_NAMES` root
//!   window property). Fast and synchronous.
//! - **Wayland** (`run` below): the compositor's live keymap, delivered by the
//!   core `wl_keyboard` protocol. That client deliberately never creates a
//!   `wl_surface`, so it can never get keyboard focus. This is correct for an
//!   always-on-top overlay that must not steal input. The protocol delivers
//!   the keymap with no need for focus.
//!
//! A Wayland session usually runs XWayland also, thus both sources can exist
//! at the same time. The compositor keymap is the primary one there;
//! XWayland only gets a copy of it. When `WAYLAND_DISPLAY` is set, the
//! Wayland source goes first, and X11 stays as a backup for an X-only
//! session. If neither source gives a result, `resolve` returns `None`, and
//! callers use their static table.

use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock};

use wayland_client::{
    protocol::{wl_keyboard, wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
// Aliased: the name `Connection` is already taken by `wayland_client`. The
// trait is only imported for its methods (`setup`, ...), never named.
use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
use x11rb::xcb_ffi::XCBConnection;
use xkbcommon::xkb;

use super::Modifier;

/// USB HID Keyboard/Keypad usage ID -> Linux evdev keycode (X11 keycode =
/// evdev keycode + 8). Copied from the kernel's `hid_keyboard[256]` table
/// (`drivers/hid/hid-input.c`). 0 marks entries that the kernel leaves
/// unmapped.
#[rustfmt::skip]
const HID_TO_EVDEV: [u8; 256] = [
    // 0x00-0x0F  (0x00-0x03 unmapped) A B C D E F G H I J K L
      0,  0,  0,  0, 30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38,
    // 0x10-0x1F  M N O P Q R S T U V W X Y Z 1 2
     50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44,  2,  3,
    // 0x20-0x2F  3 4 5 6 7 8 9 0 Enter Esc Backspace Tab Space - = [
      4,  5,  6,  7,  8,  9, 10, 11, 28,  1, 14, 15, 57, 12, 13, 26,
    // 0x30-0x3F  ] \ NUHS ; ' ` , . / Caps F1..F6
     27, 43, 43, 39, 40, 41, 51, 52, 53, 58, 59, 60, 61, 62, 63, 64,
    // 0x40-0x4F  F7..F12 PrtScr Scroll Pause Ins Home PgUp Del End PgDn Right
     65, 66, 67, 68, 87, 88, 99, 70,119,110,102,104,111,107,109,106,
    // 0x50-0x5F  Left Down Up NumLock KP/ KP* KP- KP+ KPEnter KP1..KP7
    105,108,103, 69, 98, 55, 74, 78, 96, 79, 80, 81, 75, 76, 77, 71,
    // 0x60-0x6F  KP8 KP9 KP0 KP. NUBS Menu Power KP= F13..F20
     72, 73, 82, 83, 86,127,116,117,183,184,185,186,187,188,189,190,
    // 0x70-0x7F  F21..F24 + editor keys (Open Help Props Front Stop Again
    //             Undo Cut Copy Paste Find Mute)
    191,192,193,194,134,138,130,132,128,129,131,137,133,135,136,113,
    // 0x80-0x8F  VolUp VolDn KP, Zenkaku/Hankaku Katakana<->Hiragana Yen
    //             Henkan Muhenkan Kanji (rest unmapped)
    115,114,  0,  0,  0,121,  0, 89, 93,124, 92, 94, 95,  0,  0,  0,
    // 0x90-0x9F  Hangul Hanja Katakana Hiragana Zenkaku/Hankaku (rest
    //             unmapped, Delete at 0x9C)
    122,123, 90, 91, 85,  0,  0,  0,  0,  0,  0,  0,111,  0,  0,  0,
    // 0xA0-0xAF  (unmapped)
      0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    // 0xB0-0xBF  KP( / KP) at 0xB6/0xB7, rest unmapped
      0,  0,  0,  0,  0,  0,179,180,  0,  0,  0,  0,  0,  0,  0,  0,
    // 0xC0-0xCF  (unmapped)
      0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    // 0xD0-0xDF  Delete (0xD9), rest unmapped
      0,  0,  0,  0,  0,  0,  0,  0,  0,111,  0,  0,  0,  0,  0,  0,
    // 0xE0-0xEF  LCtrl LShift LAlt LGui RCtrl RShift RAlt RGui + media keys
     29, 42, 56,125, 97, 54,100,126,164,166,165,163,161,115,114,113,
    // 0xF0-0xFF  web/app-launch keys (WWW Back Forward Mail Sleep Calc ...)
    150,158,159,128,136,177,178,176,142,152,173,140,  0,  0,  0,  0,
];

/// A parsed XKB keymap with a reusable probe `State`. One state per keymap is
/// cheaper than one per resolve call. Both types wrap raw C pointers and are
/// `!Send`, thus they live in thread-local storage.
struct Xkb {
    // Keeps the state's parent alive; the state holds only a raw pointer.
    _keymap: xkb::Keymap,
    state: xkb::State,
}

impl Xkb {
    fn new(keymap: xkb::Keymap) -> Self {
        Self {
            _keymap: keymap.clone(),
            state: xkb::State::new(&keymap),
        }
    }

    /// Resolves a HID usage ID. Returns `None` when the usage is unmapped,
    /// when the requested modifier does not exist in this keymap, or when the
    /// key produces no character.
    fn resolve(&mut self, hid_usage: u16, modifier: Modifier) -> Option<String> {
        let evdev = *HID_TO_EVDEV.get(usize::from(hid_usage))?;
        if evdev == 0 {
            return None;
        }
        // libxkbcommon uses X11-style keycodes even for Wayland keymaps, and
        // X11 keycodes are evdev keycodes + 8 (X11 reserves keycodes 0-7).
        let keycode = xkb::Keycode::new(u32::from(evdev) + 8);

        let mask = match modifier {
            Modifier::Base => 0,
            Modifier::Shift => 1 << self.mod_index(xkb::MOD_NAME_SHIFT)?,
            // RAlt binds to "Mod5" on almost every layout that has one.
            Modifier::RAlt => 1 << self.mod_index("Mod5")?,
            // RAlt plus Shift.
            Modifier::ShiftRAlt => {
                (1 << self.mod_index(xkb::MOD_NAME_SHIFT)?) | (1 << self.mod_index("Mod5")?)
            }
        };
        // Always write the whole mask so a previous probe's Shift/RAlt does
        // not leak into this one.
        self.state.update_mask(mask, 0, 0, 0, 0, 0);
        let text = self.state.key_get_utf8(keycode);
        (!text.is_empty()).then_some(text)
    }

    fn mod_index(&self, name: &str) -> Option<u32> {
        let idx = self._keymap.mod_get_index(name);
        (idx != xkb::MOD_INVALID).then_some(idx)
    }
}

struct State {
    xkb_context: xkb::Context,
    // Holds the keymap's text form, not an `xkb::Keymap`. An `xkb::Keymap`
    // wraps a raw C pointer that is not `Send`. Each resolve thread keeps its
    // own parsed copy of this text (see `PARSED_KEYMAP`).
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
            name,
            interface,
            version,
        } = event
        {
            if interface == "wl_seat" {
                // Binding a version higher than the advertised one is a
                // protocol error that kills the client, so clamp.
                let seat = registry.bind::<wl_seat::WlSeat, _, _>(
                    name,
                    version.min(WL_SEAT_VERSION),
                    qh,
                    (),
                );
                let _keyboard = seat.get_keyboard(qh, ());
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
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
        // The compositor sends this again on runtime layout changes. Thus
        // `shared` always shows the active layout.
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
        return; // Not on Wayland (e.g. an X11 session). resolve() stays None.
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

// The `wl_seat` version this client binds. It must not exceed the version
// that the compositor advertises.
const WL_SEAT_VERSION: u32 = 7;

// The wait budget for the first keymap delivery: 20 attempts x 25 ms = 500 ms.
const KEYMAP_WAIT_ATTEMPTS: usize = 20;
const KEYMAP_WAIT_INTERVAL_MS: u64 = 25;

/// Waits up to 500 ms for the compositor's first keymap delivery. Labels are
/// resolved one time per connect, so a lost startup race against the connect
/// handshake would otherwise stay visible for the whole session. After the
/// first delivery, the shared state answers instantly.
fn wait_for_keymap_text() -> Option<String> {
    let shared = shared_state();
    for _ in 0..KEYMAP_WAIT_ATTEMPTS {
        if let Some(text) = shared.lock().unwrap().clone() {
            return Some(text);
        }
        std::thread::sleep(std::time::Duration::from_millis(KEYMAP_WAIT_INTERVAL_MS));
    }
    None
}

fn resolve_wayland(hid_usage: u16, modifier: Modifier) -> Option<String> {
    let keymap_text = wait_for_keymap_text()?;
    PARSED_KEYMAP.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.as_ref().is_none_or(|(text, _)| text != &keymap_text) {
            let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
            let keymap = xkb::Keymap::new_from_string(
                &context,
                keymap_text.clone(),
                xkb::KEYMAP_FORMAT_TEXT_V1,
                xkb::KEYMAP_COMPILE_NO_FLAGS,
            )?;
            *cache = Some((keymap_text, Xkb::new(keymap)));
        }
        cache.as_mut()?.1.resolve(hid_usage, modifier)
    })
}

thread_local! {
    // Per-thread cache of the last parsed Wayland keymap, keyed by its text.
    // Parsing is expensive, and one thread resolves many legends. New text
    // from a runtime layout change replaces the entry.
    static PARSED_KEYMAP: RefCell<Option<(String, Xkb)>> = const { RefCell::new(None) };
}

// The active X server keymap with its probe state. `None` = not yet fetched.
// `Some(None)` = fetched and not present (no X server), so a missing server
// is not probed again on every call. The C keymap keeps its own reference to
// the context, so the `Context` can be dropped right after building.
thread_local! {
    static KEYMAP: RefCell<Option<Option<Xkb>>> = const { RefCell::new(None) };
}

fn resolve_x11(hid_usage: u16, modifier: Modifier) -> Option<String> {
    KEYMAP.with(|cell| {
        if cell.borrow().is_none() {
            *cell.borrow_mut() = Some(build_keymap().map(Xkb::new));
        }
        cell.borrow_mut()
            .as_mut()?
            .as_mut()?
            .resolve(hid_usage, modifier)
    })
}

fn build_keymap() -> Option<xkb::Keymap> {
    // The live server keymap would need libxkbcommon-x11, which is not
    // linked. Instead read the server's active RMLVO configuration from the
    // `_XKB_RULES_NAMES` root window property and compile the keymap from it
    // (the same approach winit uses for its X11 backend).
    let (conn, screen_num) = XCBConnection::connect(None).ok()?;
    let root = conn.setup().roots.get(screen_num)?.root;
    let atom = conn
        .intern_atom(false, b"_XKB_RULES_NAMES")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let reply = conn
        .get_property(false, root, atom, AtomEnum::STRING, 0, 1024)
        .ok()?
        .reply()
        .ok()?;
    if reply.value_len == 0 {
        return None; // Property absent or empty.
    }
    // NUL-separated: rules, model, layout, variant, options.
    let field = |i: usize| {
        String::from_utf8_lossy(reply.value.split(|&b| b == 0).nth(i).unwrap_or_default())
            .into_owned()
    };
    let rules = field(0);
    let model = field(1);
    let layout = field(2);
    let variant = field(3);
    let options = field(4);

    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    xkb::Keymap::new_from_names(
        &context,
        &rules,
        &model,
        &layout,
        &variant,
        (!options.is_empty()).then_some(options),
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
}

pub fn resolve(hid_usage: u16, modifier: Modifier) -> Option<String> {
    // The compositor keymap is the primary source. Thus a session with
    // WAYLAND_DISPLAY set tries Wayland first; X11 (XWayland) stays as a
    // backup.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        resolve_wayland(hid_usage, modifier).or_else(|| resolve_x11(hid_usage, modifier))
    } else {
        resolve_x11(hid_usage, modifier).or_else(|| resolve_wayland(hid_usage, modifier))
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve, HID_TO_EVDEV};
    use crate::os_layout::Modifier;

    // This test guards against a transcription error in the evdev table. The
    // values are checked against known-good evdev keycodes
    // (KEY_A=30, KEY_Q=16, KEY_1=2, KEY_MINUS=12).
    #[test]
    fn hid_to_evdev_known_values() {
        assert_eq!(HID_TO_EVDEV[0x04], 30); // KEY_A
        assert_eq!(HID_TO_EVDEV[0x14], 16); // KEY_Q
        assert_eq!(HID_TO_EVDEV[0x1E], 2); // KEY_1
        assert_eq!(HID_TO_EVDEV[0x2D], 12); // KEY_MINUS
    }

    // Needs a real Wayland session with a non-US layout active, thus not part
    // of the normal `cargo test` run. Run `cargo test -- --ignored` on a
    // machine set to German to confirm the live path end to end.
    #[test]
    #[ignore]
    fn live_german_shift_matches_actual_layout() {
        assert_eq!(resolve(0x1F, Modifier::Shift).as_deref(), Some("\"")); // KC_2
        assert_eq!(resolve(0x24, Modifier::Shift).as_deref(), Some("/")); // KC_7
    }
}
