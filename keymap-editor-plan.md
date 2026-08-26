# Keymap editing — implementation plan

Goal: in settings mode (overlay constantly visible), let the user select which layer the
overlay displays and edit key assignments by clicking keys in the overlay. Editing opens a
persistent "Edit key" window whose content follows the most recently clicked key. ZMK gets a
behavior-based editor, QMK/Vial gets a VIA-style keycode picker.

Scope decisions (already made — do not re-litigate):

- **QMK/Vial editor scope:** basic + media keycodes, layer keys (MO/TG/TO/OSL/TT/DF),
  modifier combos (`LSFT(kc)`), one-shot mods, mod-tap `MT(mod, kc)`, layer-tap `LT(layer, kc)`,
  and a raw-hex "Any key" fallback.
- **ZMK editor scope:** built-in behaviors only. Keys bound to custom behaviors (e.g. a
  home-row-mod hold-tap from the user's keymap) are *displayed* and can be *replaced* by a
  built-in, but their parameters are not editable in v1.
- **ZMK persistence:** ZMK Studio-style. Edits apply to device RAM immediately; a visible
  "unsaved changes" state with **Save** / **Discard** buttons; closing settings with unsaved
  changes prompts once (Save / Discard / Cancel). QMK/VIA writes persist immediately by
  protocol design — no save UI there.
- **Click-to-edit only on pinned layers.** The "Active" dropdown entry stays a read-only live
  view. Clicking keys is enabled only when a specific layer is selected in the dropdown.
- **No combos, macros, tap-dance editing.** Keys currently bound to those are shown (existing
  labels) and may be overwritten with a supported action; their definitions are untouched.

---

## 1. Current architecture (read before coding)

Data flow today, with the files that implement it:

```
protocol connect                    read path (labels only)                     UI
─────────────────                   ───────────────────────                    ──────────────
via.rs / vial.rs   ── u16 keycode ──► qmk_keycode_labels::get_layout_key ──┐
zmk_rpc.rs         ── Behavior ─────► zmk_keycode_labels::behavior_to_    ─┼─► Vec<Vec<Vec<Option<LayoutKey>>>>
mock.rs            ── u16 keycode ──► layout_key (at CONNECT time)         ┘        │
                                                                           KeyMatrix (key_matrix.rs)
                                                                                     │  Arc<Mutex<..>> in
                                                                           Keyboard (keyboard.rs)
                                                                                     │
                                                            overlay_window/ui_overlay.rs (paints labels)
```

Key facts that shape this plan:

1. **Raw actions are discarded at read time.** `KeyboardProtocol::read_all_keys`
   (`src/protocols/mod.rs`) returns resolved `LayoutKey` labels. The `u16` VIA keycode /
   `zmk_studio_api::Behavior` is gone by the time anything reaches the UI. This is the main
   preparation work: keep the raw action alongside the label.

2. **The protocol object is moved into a background thread.** `Keyboard::new`
   (`src/keyboard.rs`) moves the `Box<dyn KeyboardProtocol>` into the HID reader thread that
   receives layer-state packets. The UI has no protocol handle. Any write path must go
   *through that thread* (a command channel), because:

3. **VIA/Vial writes race the reader thread.** `qmk_via_api::KeyboardApi::set_key` sends a
   report and does exactly **one** read, expecting the command echo
   (`hid_command_on_device` in the crate). The reader thread continuously reads the same HID
   handle, so a write issued from another thread/handle would have its response stolen.
   Routing writes through the owning thread (pausing reads around a command) is the only
   clean option.

4. **ZMK writes use a different transport than reads.** `zmk_rpc::fetch_zmk_data`
   (`src/protocols/zmk_rpc.rs`) opens a ZMK Studio RPC client (serial or BLE), resolves the
   keymap, then **drops the client**. Layer events afterwards come over plain HID
   (`src/protocols/zmk.rs`). Writing requires re-opening a Studio client
   (`StudioClient::set_key_at`, `save_changes`, `discard_changes` — all already exist in
   `zmk-studio-api` 0.5.1). The device may be *locked* again at that point
   (`DeviceLocked`), which must surface as a friendly error.

5. **Write persistence differs.** `qmk-via-api`'s `set_key` writes the dynamic keymap in
   EEPROM — persistent immediately, no save/discard concept. ZMK's `set_key_at` changes RAM;
   `save_changes` persists to flash; `discard_changes` reverts RAM to the saved state.

6. **Layer identity differs.** ZMK layers have a stable `id` *and* a position (index);
   `set_key_at(layer_id, ...)` takes the stable id, while `KeyMatrix` and
   `get_effective_key_layer` operate on indices. QMK layers are just indices. Layer metadata
   (id + name) must travel to the UI.

7. **The overlay window is currently non-interactive.** `draw_overlay_window`
   (`src/overlay_window/ui_overlay.rs`) uses `.interactable(false)`; mouse passthrough is
   disabled while settings are open (`sync_mouse_passthrough`), so egui *does* receive mouse
   events in settings mode — the window just ignores them. Keys can be rotated (`Key::r`),
   so hit-testing must account for rotation.

No new dependencies are needed. Both API crates already ship the required write calls.

### Ground rules for the implementing model

- One stage = one commit/PR. Do not start a stage before the previous one compiles, passes
  `cargo test`, `cargo fmt --check`, and `cargo clippy`.
- Do not refactor code that a stage does not require. Do not rename existing public items
  unless the stage says so.
- Comments describe the current state only — never the change history or this plan.
- macOS dev note: `cargo run` (unbundled) aborts during the BLE scan unless Bluetooth is
  off. Use the **mock device** (debug builds only) for all UI development; it appears in the
  device dropdown automatically.

---

## 2. Target architecture

New shared model in a new file `src/key_action.rs`:

```rust
/// The firmware-level assignment of one key on one layer. This is what the
/// device stores; `LayoutKey` labels are derived from it.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    /// A VIA/Vial dynamic-keymap keycode (protocol v12 encoding).
    Qmk(u16),
    /// A resolved ZMK binding.
    Zmk(zmk_studio_api::Behavior),
}

impl KeyAction {
    /// Derive the display label. `None` = transparent (falls through to lower layers).
    pub fn resolve_label(&self, layer_names: &[String]) -> Option<LayoutKey> {
        match self {
            KeyAction::Qmk(code) => crate::qmk_keycode_labels::get_layout_key(*code),
            KeyAction::Zmk(b) => crate::zmk_keycode_labels::behavior_to_layout_key(b, layer_names),
        }
    }
}

/// Identity of one layer as the device reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerInfo {
    /// Stable ZMK layer id, used by ZMK Studio write RPCs. Equals the index for QMK/mock.
    pub id: u32,
    /// ZMK layer name. `None` for QMK (display falls back to "Layer {index}").
    pub name: Option<String>,
}

/// Everything the protocol knows about the keymap, actions included.
pub struct KeymapSnapshot {
    pub layers: Vec<LayerInfo>,
    /// `[layer][row][col]`. `None` = no binding at this position (padding).
    pub actions: Vec<Vec<Vec<Option<KeyAction>>>>,
}
```

`KeyMatrix` cells become:

```rust
pub struct BoundKey {
    pub action: KeyAction,
    /// `None` = transparent binding (renders as fall-through, still editable).
    pub label: Option<LayoutKey>,
}
```

so `keys: Vec<Vec<Vec<Option<BoundKey>>>>` where the *outer* `None` means "no binding slot
here" (not clickable), and `label: None` means "transparent binding" (clickable, renders as
fall-through). `KeyMatrix` also stores `layers: Vec<LayerInfo>`.

Write path: `KeyboardProtocol` grows write methods; `Keyboard`'s reader thread becomes an
actor that also executes keymap commands sent over an `mpsc` channel; the UI polls a oneshot
receiver per request (standard egui non-blocking pattern, same idea as `ConnectionTask`).

New UI module `src/keymap_editor/` holds the editor window, the shared keycode-picker grid,
and the QMK/ZMK-specific content. `overlay_window/` gets the layer dropdown, key hit-testing,
and editor state wiring.

---

## 3. Stages

**Stage 0 is a go/no-go gate. Do not start Stage 1 before it has passed.**

### Stage 0 — hardware go/no-go spike (throwaway, do this first)

**Goal:** prove on real hardware the one assumption this plan cannot verify from code:
that a ZMK Studio RPC session works while KeyPeek's HID layer-event connection is open on
the same device. The same session also settles Stage 6's layer-parameter question. All code
written here is throwaway (an `#[ignore]`d test or a scratch `examples/` binary) — nothing
is merged.

Steps:

1. Connect exactly like KeyPeek does: call `zmk_rpc::fetch_zmk_data(transport)` (opens the
   Studio RPC client, reads the keymap, drops the client), then open the HID interface the
   way `ZmkProtocol::open_hid` does and start a thread that loops `hid_read`, printing
   layer-state packets. Press layer keys on the board to confirm events flow.
2. With that thread still running, open a **second** Studio client on the same transport
   (`StudioClient::open_serial` / `open_ble`), check the lock state, and call `set_key_at`
   on some key. Verify all three: the RPC succeeds, the keyboard immediately produces the
   new binding, and layer events keep arriving on the HID thread while the client stays open.
3. Restore the original binding (`set_key_at` with the previously read value, or
   `discard_changes`).
4. While still connected, read a known `&mo`/`&lt` binding via `get_key_at` and check
   whether its layer-typed parameter is the layer's **index** (position in `get_keymap`'s
   layer list) or its stable **id**. **Record the answer in this document** — Stage 6's
   layer dropdown depends on it.

Run over serial and, if available, BLE. (macOS: unbundled `cargo run` aborts on the BLE
scan; a spike binary that opens a known device directly may still need Bluetooth permission
for the terminal — serial is the low-friction path.)

**Pass →** proceed with the plan as written.
**Fail (RPC and HID cannot coexist) →** the architecture still holds with one local change,
to be applied in Stage 2: the actor owns both connections, so it pauses the HID loop while
a Studio session is open (skip `hid_read` while `session.is_some()`). The overlay then
stops following layer changes during an edit session — acceptable, since editing happens
in settings mode. Note the outcome here either way.

**Stage 0 outcome (recorded 2026-08-26):** PASS over BLE ("Cygnus", bluest backend).
A second Studio session stayed open while the HID interface kept delivering layer-state
packets; events also kept flowing after the session closed; `set_key_at` + readback +
`discard_changes` all succeeded while HID ran.
Layer-parameter space step 4: **indistinguishable on this board — every stable layer id
equals its index** (4 layers, ids 0–3; observed `&mo 2` / `&lt 1` params match both
columns). Resolution for Stage 6: write behavior layer parameters using the target
layer's `LayerInfo::id`, which is identical to the index whenever ids are sequential and
is what `behavior_to_layout_key` already assumes when indexing `layer_names`. Re-check if
a board ever reports non-sequential ids.

### Stage 1 — carry raw actions and layer metadata to the UI (pure refactor)

**Goal:** protocols return `KeymapSnapshot` (actions), label resolution moves to one place,
`KeyMatrix` stores `BoundKey`s. Zero visible behavior change.

Changes:

1. Add `src/key_action.rs` as specified above (`mod key_action;` in `main.rs`, matching how
   the other modules are declared).
2. `src/protocols/mod.rs`: replace both `get_layer_count` and `read_all_keys` on
   `KeyboardProtocol` with:
   ```rust
   fn read_keymap(&self) -> Result<crate::key_action::KeymapSnapshot, Box<dyn Error>>;
   ```
3. `via.rs` / `vial.rs`: build the snapshot from `get_layer_count()` +
   `read_raw_matrix` — they already hold the `u16` before calling `get_layout_key`; store
   `KeyAction::Qmk(code)` instead. A layer whose `read_raw_matrix` fails keeps all-`None`
   cells (same tolerance as today). `LayerInfo { id: index as u32, name: None }`.
4. `zmk_rpc.rs`: change `ZmkData` to carry `Vec<zmk_studio_api::ResolvedLayer>` (or an
   equivalent of ids/names/behaviors) instead of pre-resolved `LayoutKey`s. Delete the
   label conversion here.
5. `zmk.rs`: `ZmkLayout` caches a `KeymapSnapshot` instead of `layout_keys`
   (`Behavior` is `Clone`). `read_keymap` clones it. Bindings beyond the physical key count
   stay `None` (current `resize` behavior). `LayerInfo { id: resolved_layer.id, name: Some(..) }`
   (empty names → `None`).
6. `mock.rs`: store `KeyAction::Qmk` per cell; snapshot built from the fixture.
7. `key_matrix.rs`: `BoundKey` cells + `layers: Vec<LayerInfo>` as specified. Constructor
   takes a `KeymapSnapshot` and resolves labels via `KeyAction::resolve_label`, passing
   layer names built as `name.unwrap_or(format!("L{index}"))`... **careful:** ZMK label
   fallback for unnamed layers already lives inside `behavior_to_layout_key`
   (`layer_arg_label`), so pass empty strings for unnamed layers, exactly as
   `zmk_rpc` does today — do not invent names here, or ZMK labels change.
   `get_key`/`is_transparent` keep their exact semantics (transparent = outer `None` **or**
   `label == None`). Add `get_action(layer, row, col) -> Option<&KeyAction>` and
   `layer_infos()`.
8. `keyboard.rs`: `Keyboard::new` calls `read_keymap()` once; drop the separate layer-count
   call. Expose `layer_infos(&self) -> Vec<LayerInfo>` and
   `get_action(&self, layer, row, col) -> Option<KeyAction>` (cloned out of the mutex, like
   `get_key`).

Acceptance:

- `cargo test` passes; existing mock tests unchanged apart from mechanical adaptation.
- New unit test (mock): a `MO(1)` fixture key resolves to the same `LayoutKey` as before the
  refactor, and `get_action` returns `KeyAction::Qmk(0x5221)`.
- Manual: mock device connects, overlay renders identically, layer cycling still works.

### Stage 2 — write engine (no UI yet)

**Goal:** `Keyboard::set_key(...)` works end-to-end for mock, VIA/Vial, and ZMK; ZMK also
gets save/discard/end-session. Testable through unit tests on the mock.

Changes:

1. `src/protocols/mod.rs`, trait additions (defaults keep read-only protocols compiling):
   ```rust
   #[derive(Clone, Copy, PartialEq)]
   pub enum WriteSupport {
       None,
       Immediate,   // QMK/Vial/mock: every write persists at once
       Session,     // ZMK: writes are RAM until save_keymap()
   }

   fn write_support(&self) -> WriteSupport { WriteSupport::None }
   fn set_key(&mut self, layer: &LayerInfo, layer_index: usize, row: usize, col: usize,
              action: &KeyAction) -> Result<(), Box<dyn Error>> { Err("not supported".into()) }
   /// ZMK: save_changes RPC. Immediate protocols: Ok(()).
   fn save_keymap(&mut self) -> Result<(), Box<dyn Error>> { Ok(()) }
   /// ZMK: discard_changes + re-read; returns the reverted keymap.
   fn discard_keymap(&mut self) -> Result<KeymapSnapshot, Box<dyn Error>> { Err("not supported".into()) }
   /// Close any transient write connection (ZMK Studio client). Default no-op.
   fn end_edit_session(&mut self) {}
   ```
2. **VIA/Vial** (`via.rs`, `vial.rs`):
   - `set_key` matches `KeyAction::Qmk(code)` → `self.api.set_key(layer_index, row, col, code)`
     (reject `KeyAction::Zmk` with an error — it cannot occur, but do not panic).
   - Construct `KeyboardApi::new(vid, pid, 0xff60, Some(250))` instead of `None` so
     `hid_read` times out and the actor loop stays responsive. A timed-out read returns a
     zeroed 32-byte buffer, which the existing packet parser already ignores.
   - Race note: a layer-state packet arriving between `set_key`'s send and read makes the
     crate return `BadCommandResponse` even though the write usually applied (and the layer
     packet is lost — harmless). Mitigation: on that error, retry `api.get_key(layer,row,col)`
     up to 3 times and treat a matching readback as success.
3. **ZMK** (`zmk.rs`, `zmk_rpc.rs`):
   - `ZmkProtocol` keeps the `ZmkTransport` it connected with (thread it through
     `connect_live`, and through `ZmkReopener` so a reconnected protocol can still write).
   - New in `zmk_rpc.rs`:
     ```rust
     pub enum ZmkStudioSession { Serial(StudioClient<..serial io..>), Ble(StudioClient<PlatformBleTransport>) }
     impl ZmkStudioSession {
         pub fn open(transport: &ZmkTransport) -> Result<Self, Box<dyn Error>>; // maps locked → DeviceLocked
         pub fn set_key(&mut self, layer_id: u32, position: i32, b: Behavior) -> Result<(), Box<dyn Error>>;
         pub fn save(&mut self) -> Result<(), Box<dyn Error>>;
         pub fn discard(&mut self) -> Result<Vec<ResolvedLayer>, Box<dyn Error>>; // discard + resolve_keymap
     }
     ```
     Small enum with match arms per transport — the two `StudioClient` instantiations are
     different concrete types; do not try to unify them generically.
   - `ZmkProtocol::set_key` lazily opens `Option<ZmkStudioSession>` on first write, reuses it
     for the whole edit session (BLE opens are slow), calls
     `session.set_key(layer.id, key_index_as_position, behavior)`. ZMK's matrix is 1×N
     (`row` is always 0, `col` is the key position) — use `col as i32` for `key_position`.
   - On success, also update the cached `KeymapSnapshot` so `read_keymap` and the reopener
     stay truthful after a reconnect. That cache is shared with `ZmkReopener` via `Arc` —
     wrap the snapshot part in a `Mutex` (`Arc<ZmkLayout>` stays, its `snapshot` field
     becomes `Mutex<KeymapSnapshot>`).
   - `discard_keymap` rebuilds the snapshot from the device and replaces the cache.
   - `end_edit_session` drops the session (`self.session = None`).
   - Error mapping: `DeviceLocked` → the existing friendly unlock message
     (`connection.rs` has the constant; move or duplicate the text so the editor can show it).
4. **Mock** (`mock.rs`): `set_key` mutates the in-memory layer table. `WriteSupport::Immediate`.
5. **Keyboard actor** (`keyboard.rs`):
   ```rust
   pub enum KeymapCommand {
       SetKey { layer_index: usize, row: usize, col: usize, action: KeyAction,
                respond: mpsc::Sender<Result<(), String>> },
       Save    { respond: mpsc::Sender<Result<(), String>> },
       Discard { respond: mpsc::Sender<Result<(), String>> },
       EndEditSession,
   }
   ```
   - `Keyboard::new` creates the channel; the reader thread drains `try_recv()` once per
     loop iteration (right after `hid_read` returns or times out) and executes commands on
     the protocol (make the moved box `mut`).
   - On successful `SetKey`, the actor updates the matrix cell: store the action and
     `resolve_label` with the layer names (keep the names accessible to the thread — clone
     them from the snapshot at startup). Then `ui_wake.request_repaint()`.
   - On successful `Discard`, rebuild the whole matrix content from the returned snapshot
     (preserve `pressed` state).
   - `Keyboard` public API: `write_support()` (captured before the move),
     `set_key(..) -> mpsc::Receiver<Result<(), String>>`, `save_keymap()`,
     `discard_keymap()` (same receiver pattern), `end_edit_session()` (fire-and-forget).
   - If the reader thread has died (connection dropped), the channel `send` fails or the
     receiver disconnects — map both to a "connection lost" error result so the UI shows
     something sensible instead of hanging on the receiver.
   - Latency note: commands wait at most one `hid_read` timeout (250 ms VIA, 200 ms ZMK,
     1.5 s mock tick — acceptable for dev).

Acceptance:

- Unit tests on mock: `set_key` for a plain keycode, a `MO(n)`, and `KC_TRANSPARENT`; the
  matrix action *and* label update; result arrives on the receiver.
- ZMK integration check on hardware: one `set_key` through the real actor path (RPC/HID
  coexistence itself was already proven — or ruled out, with the pause-the-loop fallback
  applied — by the Stage 0 gate).

### Stage 3 — layer dropdown + pinned-layer rendering (shippable on its own)

**Goal:** in settings mode a dropdown next to the overlay selects "Active" (default,
today's behavior) or a specific layer, which renders that layer flat.

Changes:

1. `overlay_window/state.rs`: add `pinned_layer: Option<usize>` to `UiState` (`None` =
   Active). Reset to `None` whenever settings close (`draw_settings_window`'s close branch)
   and on disconnect; also reset if the pinned index is out of range for the current matrix
   (the layer count can change across a reconnect).
2. `ui_overlay.rs`: `draw_overlay_window` already returns nothing; capture the window's
   response rect (`Window::show` return value) and store it for this frame. When
   `settings_visible && connected`, draw a small auto-sized `egui::Window` (no title bar,
   not collapsible) adjacent to the overlay rect: above it when the overlay is anchored to a
   bottom position, below it otherwise (derive from `self.settings.active.position`).
   Content: one `ComboBox` with "Active" plus one entry per layer, labeled
   `"{index}: {name}"` (name from `LayerInfo`, fallback `"Layer {index}"`).
3. Pinned rendering in the key loop: when `pinned_layer == Some(n)`, use `effective_layer = n`,
   `is_background_key = false` for every key, and skip the live-preview shift/ralt logic.
   Transparent cells (`get_key` returns `None`) render as a dimmed empty key: the layer's
   fill color at low alpha (e.g. `fill.gamma_multiply(0.25)`), default thin border, no label.
   In Active mode nothing changes.

Acceptance: with the mock connected and settings open, pinning layer 2 freezes the overlay
on layer 2 (mock keeps cycling layer state — display must ignore it), transparent keys are
visibly dimmed, "Active" restores today's behavior, closing settings resets to Active.

### Stage 4 — click-to-edit plumbing + editor window shell

**Goal:** clicking a key on a pinned layer opens a persistent "Edit key" window showing the
key's current binding; clicking another key retargets the same window. No editing yet.

Changes:

1. New module `src/keymap_editor/mod.rs` with the editor state, owned by `OverlayApp`:
   ```rust
   pub struct EditorState {
       pub target: Option<EditTarget>,           // None = window closed
       pub pending: Option<mpsc::Receiver<Result<(), String>>>,
       pub error: Option<String>,
       // per-firmware draft state added in stages 5/6
   }
   pub struct EditTarget { pub layer_index: usize, pub row: usize, pub col: usize }
   ```
2. Hit-testing in `draw_overlay_window`: when `settings_visible` **and** a layer is pinned,
   create the window with `.interactable(true)` (keep `false` otherwise). Do **not** read
   the pointer from `ctx.input` directly — the settings window can overlap the overlay, and
   raw pointer reads would fire on keys visually underneath it. Instead go through egui's
   pointer routing: `allocate_space` already returns an id and the full overlay rect, so
   replace it with `let response = ui.interact(rect, id, egui::Sense::click())` (allocate
   the space first as today, then interact on the returned rect). Then, per key, with
   `rect`, `center`, `angle` already computed:
   - hover = `response.hover_pos()` is `Some(p)` and
     `rect.contains(rotate_point(p, center, -angle))` (the existing `rotate_point` with the
     negated angle inverse-rotates the pointer into the key's unrotated frame);
   - highlight hovered keys (e.g. border `lerp_to_gamma(WHITE, 0.45)`, pointer cursor via
     `ctx.set_cursor_icon`);
   - on `response.clicked()` with a hovered key: set `editor.target`, clear `editor.error`,
     rebuild the editor draft from `keyboard.get_action(..)`.
   Only keys with an existing binding slot are clickable (`get_action` is `Some`; transparent
   counts as clickable, absent does not).
3. Editor window (in `keymap_editor/mod.rs`, called from `OverlayApp::ui` when
   `target.is_some()`): a titled, closable, movable `egui::Window` ("Edit key"). Header
   shows: layer (`"{index}: {name}"`), key coordinates, and the current binding rendered as
   text — the resolved label's `full` strings plus the raw form (`0x{:04X}` for QMK; the
   `Behavior` variant name and params for ZMK). Body says "editing arrives in the next
   stages" only implicitly — just render the header and an error slot for now.
4. Lifecycle: window closes on its close button, on disconnect, and when settings close
   (also send `keyboard.end_edit_session()` on settings close). Switching the pinned layer
   keeps the window open; the target keeps its own layer index until the next click.

Acceptance: with mock pinned to a layer, clicking keys opens/retargets the window and the
shown binding matches the overlay; Active mode does not react to clicks; hover feedback only
appears on pinned layers.

### Stage 5 — QMK/Vial editor content

**Goal:** full VIA-style editing for `WriteSupport::Immediate` + `KeyAction::Qmk` targets.
Selections apply immediately (VIA behavior — no Apply button).

Changes:

1. `src/keymap_editor/qmk_catalog.rs`: hand-curated static candidate lists, grouped:
   ```rust
   pub struct Category { pub name: &'static str, pub codes: &'static [u16] }
   pub const CATEGORIES: &[Category] = &[ /* Basic, Media, Special */ ];
   ```
   Populate from `qmk_via_api::keycodes::Keycode` values: Basic = KC_NO, KC_TRANSPARENT,
   letters, digits, Enter/Esc/Backspace/Tab/Space, punctuation, F1–F24, nav cluster,
   keypad, modifiers (LCTL…RGUI), Caps Lock, Print Screen etc.; Media = the consumer/media
   and brightness keycodes the label table renders. Rule of thumb: include every keycode
   `< 0x0100` for which `get_basic_layout_key` returns a label; skip the rest.
2. Shared picker widget in `src/keymap_editor/picker.rs`: a scrollable grid of key-shaped
   buttons. Each candidate renders its resolved label (`KeyAction::Qmk(code).resolve_label(&[])`,
   using `full`/`short`/`symbol` — a plain `ui.button` with the label text is fine for v1;
   do **not** replicate the overlay's painter). Special-case the two candidates whose
   resolved label is unusable as button text: `KC_TRANSPARENT` resolves to `None` (show
   "▽ Trans") and `KC_NO` resolves to an empty label (show "None"); same for ZMK's
   `Transparent`/`None` behaviors in Stage 6. The currently assigned code is highlighted.
   Parameterize over a generic candidate type so Stage 6 can reuse it for ZMK keycodes.
3. Editor body (`qmk_editor.rs`), driven by a draft struct rebuilt on each retarget.
   Sections (e.g. a `ComboBox` or tab row): **Basic**, **Media**, **Layers**, **Mods**,
   **Any**.
   - *Basic/Media*: picker grid; clicking a candidate sends the write.
   - *Layers*: kind dropdown (MO/TG/TO/OSL/TT/DF) + layer index dropdown (0..layer count).
     Encode via the ranges in `qmk_keycode_labels/constants.rs`
     (e.g. `QK_MOMENTARY.start + layer`). Apply button here (two inputs).
   - *Mods*: three sub-modes. Encoding constraints, verified against the decoders in
     `advanced.rs` (decode and encode must round-trip; the unit test below enforces it):
     the wrapped/tap keycode is only **8 bits** (`remainder & 0xFF`), so the base/tap-key
     picker in this section must offer only basic keycodes `<= 0xFF`.
     - modifier combo `LSFT(kc)`: checkboxes Ctrl/Shift/Alt/GUI + Left/Right toggle + base
       key from the picker → `(mods << 8) | keycode` (bits 8–11 = MOD_LCTL..MOD_LGUI,
       bit 12 = MOD_RIGHT_FLAG). At least one modifier must be checked — `mods == 0` would
       fall out of `QK_MODS` into the basic range;
     - one-shot mod: mods checkboxes (non-zero) → `QK_ONE_SHOT_MOD.start + mods`;
     - mod-tap `MT(mod, kc)` / layer-tap `LT(layer, kc)`: mods-or-layer + tap key →
       `QK_MOD_TAP.start + (mods << 8) + keycode` / `QK_LAYER_TAP.start + (layer << 8) + keycode`.
       `LT` carries only 4 layer bits — limit its layer dropdown to 0–15 (the MO/TG/… ranges
       in the *Layers* section allow 0–31).
     Apply button.
   - *Any*: hex text field (`0x????`), live label preview via `resolve_label`, Apply button.
   - When retargeting, pre-select the section and pre-fill the draft by decoding the current
     action (reverse of the encodings above; unknown ranges land in *Any*).
4. Write flow: send `keyboard.set_key(...)`, store the receiver in `editor.pending`, disable
   editor input while pending, poll with `try_recv` each frame (request a repaint while
   pending). On `Err`, show the message in the editor window. The overlay updates by itself
   (the actor already updated the matrix).

Acceptance: end-to-end on the mock — reassign a basic key, a `MO(n)`, an `LSFT(kc)`, an
`MT`, and a raw hex value; overlay label updates immediately each time; decode→encode
round-trips (add a unit test for the encoders against `advanced.rs`/`layer.rs` decoders,
e.g. encode then `get_layout_key` and compare with a directly-constructed expectation).
Then verify on a real Vial board.

### Stage 6 — ZMK editor content + Save/Discard

**Goal:** behavior-based editing for `KeyAction::Zmk` targets, with the session save flow.

Changes:

1. `src/keymap_editor/zmk_catalog.rs`: static ZMK keycode candidate lists (mirroring the
   QMK categories, values from `zmk_studio_api::Keycode` — keyboard page + consumer page),
   reusing the Stage 5 picker with `KeyAction::Zmk(Behavior::KeyPress(usage))` labels.
2. Editor body (`zmk_editor.rs`): a **behavior** dropdown + per-behavior parameter editors.
   Behaviors offered (all map onto `zmk_studio_api::Behavior` variants, written via the
   existing `set_key_at`):
   - Key Press / Key Toggle / Sticky Key → keycode picker + modifier checkboxes
     (`HidUsage::from_parts(page, id, modifiers)`; modifiers use the `MOD_*` consts from
     `zmk_studio_api`).
   - Momentary / Toggle / To / Sticky Layer → layer dropdown.
   - Layer-Tap → layer dropdown + tap keycode picker. Mod-Tap → hold-modifier dropdown
     (the 8 modifier keycodes) + tap keycode picker.
   - Transparent, None, Caps Word, Key Repeat, Grave Escape, Studio Unlock, Reset,
     Bootloader, Soft Off → no parameters.
   - Bluetooth / Output Selection / Backlight / Underglow / Mouse buttons → command
     dropdowns matching the label tables in `zmk_keycode_labels/behavior.rs` (Bluetooth
     select/disconnect additionally take a profile number). Mouse move/scroll: offer the
     four directions only. These are cheap; if any turns out fiddly, defer it — they are not
     acceptance-critical.
   - A key currently bound to `Custom`/`Unknown` shows its display name read-only with a
     note; choosing any behavior above replaces it.
   - **Layer-parameter space:** the layer dropdown writes layer-typed *behavior parameters*
     (`MomentaryLayer { layer_id }` etc.) in whichever space the Stage 0 spike recorded
     (index vs stable id — step 4 of Stage 0 answers this; `behavior_to_layout_key`
     currently indexes `layer_names` with it, suggesting index). The outer
     `set_key_at(layer_id, ...)` always uses the stable `LayerInfo::id` regardless.
   - Apply button for parametered behaviors; parameterless ones apply on selection.
3. Save/Discard UI: `OverlayApp` tracks `zmk_dirty: bool` — set on every successful ZMK
   write, cleared on successful Save/Discard. When `write_support == Session` and dirty,
   the editor window (and/or the layer-dropdown strip) shows "Unsaved changes" with
   **Save** and **Discard** buttons wired to `keyboard.save_keymap()` /
   `keyboard.discard_keymap()` (same pending-receiver pattern). Discard success already
   refreshes the matrix via the actor.
4. Close-with-unsaved prompt: when settings are closing (`draw_settings_window` close
   branch) and `zmk_dirty`, veto the close once and show a modal (reuse the
   `message_window` style with three buttons): Save / Discard / Cancel.
5. Error surfacing: `DeviceLocked` from any write → editor error text telling the user to
   press the Studio unlock combo and retry (the session retries opening on the next write).

Acceptance (real ZMK hardware, serial and BLE): reassign a key press with modifiers, a
momentary layer, a mod-tap; overlay updates; keyboard produces the new binding immediately;
Save persists across a reboot; Discard reverts both device and overlay; locking the device
mid-session produces the friendly error; closing settings with unsaved changes prompts.

---

## 4. Risks & verification points (keep visible while implementing)

| Risk | Stage | Mitigation |
|---|---|---|
| ZMK Studio RPC and HID layer events on one device simultaneously (esp. BLE) | 0 | The Stage 0 go/no-go gate, run before any feature work; if coexistence fails, Stage 2 pauses the HID loop during edit sessions (the actor owns both, so this is a local change). |
| VIA response stolen by an interleaved layer packet | 2 | Retry/readback via `get_key` (see Stage 2). |
| ZMK layer-typed behavior params: index vs stable id | 0 | Answered by Stage 0 step 4 and recorded in this document; Stage 6 reads the answer from here. |
| Device re-locks mid edit session | 2/6 | Map `DeviceLocked` to a friendly retryable error everywhere. |
| Edits lost from display after a ZMK BLE reconnect | 2 | Reopener shares the mutable `KeymapSnapshot` cache; update it on every successful write. |
| Rotated-key hit testing | 4 | Inverse-rotate the pointer with the existing `rotate_point`; test on the mock by giving one fixture key a rotation. |
| VIA connect flakiness after adding the 250 ms read timeout | 2 | Timeout only affects command reads; bump to 500 ms if a real board misses it. |
| First ZMK write per session is slow: it opens the Studio client **and** fetches the behavior catalog (`ensure_behavior_catalog` RPCs), blocking the actor and pausing layer events for a few seconds on BLE | 2/6 | Known tradeoff, acceptable in settings mode. The UI stays responsive (pending receiver); just do not add a timeout shorter than ~10 s to the pending state. |

## 5. Explicitly out of scope (v1)

- Editing macros, combos, tap dances, encoders; editing ZMK custom-behavior parameters.
- Adding/removing/renaming/reordering layers.
- Editing while the overlay is in normal (non-settings) mode, or on the "Active" view.
- Any keymap import/export.



