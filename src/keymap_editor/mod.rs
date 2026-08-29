//! Persistent "Edit key" window: its content follows the most recently clicked
//! key on a pinned layer.

mod picker;
mod qmk_catalog;
mod qmk_editor;
mod zmk_catalog;
mod zmk_editor;

pub use qmk_editor::QmkDraft;
pub use zmk_editor::ZmkDraft;

use crate::key_action::KeyAction;
use crate::key_paint::{self, KeyDisplay};
use crate::keyboard::Keyboard;
use crate::layout_key::{BorderStyle, Label};
use crate::protocols::WriteSupport;
use egui::Window;
use std::sync::mpsc;

/// Pixels per key-unit for the header's preview of the current assignment.
const PREVIEW_KEY_UNIT: f32 = 68.0;

/// Which key the editor window is currently targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditTarget {
    pub layer_index: usize,
    pub row: usize,
    pub col: usize,
}

/// What an in-flight `pending` receiver is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingKind {
    Open,
    Set,
    Save,
}

/// Lifecycle of the transient ZMK Studio session backing the editor window:
/// it opens as soon as the window appears, carries the writes and the
/// close-save, and ends when the window closes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ZmkSessionState {
    /// No session yet; the window open — or a retarget after a failure —
    /// starts one.
    Idle,
    /// The session is being established on the reader thread; the picker is
    /// disabled until it lands.
    Opening,
    /// The session is open and writes can proceed.
    Ready,
    /// The open failed (locked device, port in use); the error shows in the
    /// window and no automatic retry happens until the next retarget or
    /// window reopen.
    Failed,
}

/// Editor window state, owned by `OverlayApp`.
pub struct EditorState {
    /// `None` = window closed.
    pub target: Option<EditTarget>,
    /// An in-flight write; polled each frame.
    pub pending: Option<mpsc::Receiver<Result<(), String>>>,
    /// What the in-flight write is for.
    pub pending_kind: Option<PendingKind>,
    /// Last-write-wins slot for a write requested while one is in flight:
    /// rapid edits (every valid pick applies instantly) must queue instead of
    /// being dropped, so the final state always reaches the device.
    pub queued: Option<(EditTarget, KeyAction)>,
    /// Last write/read error shown in the window.
    pub error: Option<String>,
    /// Per-firmware draft state, rebuilt on each retarget.
    pub qmk_draft: QmkDraft,
    pub zmk_draft: ZmkDraft,
    /// ZMK: whether there are unsaved changes in device RAM.
    pub zmk_dirty: bool,
    /// ZMK: lifecycle of the transient Studio session behind the window.
    pub zmk_session: ZmkSessionState,
    /// ZMK: a close was requested while dirty; the window saves first, then
    /// closes itself. No further edits or retargets happen while closing.
    pub closing: bool,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            target: None,
            pending: None,
            pending_kind: None,
            queued: None,
            error: None,
            qmk_draft: QmkDraft::default(),
            zmk_draft: ZmkDraft::default(),
            zmk_dirty: false,
            zmk_session: ZmkSessionState::Idle,
            closing: false,
        }
    }
}

/// The raw firmware form of a binding, shown under the header key: hex for
/// QMK keycodes, the behavior's debug form for ZMK.
pub fn raw_value_text(action: &KeyAction) -> String {
    match action {
        KeyAction::Qmk(code) => format!("0x{code:04X}"),
        KeyAction::Zmk(behavior) => format!("{behavior:?}"),
    }
}

/// The editor's two-pane layout, shared by the QMK and ZMK editors: a
/// fixed-width, non-resizable left list of entries and a scrolling central
/// pane, both filling the window instead of growing it. `state` is the draft
/// both panes edit; `left` runs first, since the central pane reads what it
/// selects.
fn editor_panes<D>(
    ui: &mut egui::Ui,
    left_id: &str,
    left_width: f32,
    state: &mut D,
    left: impl FnOnce(&mut egui::Ui, &mut D),
    central: impl FnOnce(&mut egui::Ui, &mut D),
) {
    let left_id = egui::Id::new(left_id);
    // No separator line between the panes: the central pane's group outlines
    // carry the visual separation.
    egui::Panel::left(left_id)
        .resizable(false)
        .exact_size(left_width)
        .show_separator_line(false)
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(2.0);
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    left(ui, state);
                });
            });
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| central(ui, state));
    });
}

impl crate::overlay_window::OverlayApp {
    /// Draws the persistent "Edit key" window for the current target.
    pub(super) fn draw_editor_window(&mut self, ctx: &egui::Context, keyboard: &Keyboard) {
        let Some(target) = self.editor.target else {
            return;
        };
        let layer_names: Vec<String> = keyboard
            .layer_infos()
            .iter()
            .map(|info| info.name.clone().unwrap_or_default())
            .collect();

        // A close was requested (window X or settings close): a dirty ZMK
        // session saves first, and the window dismisses itself once the save
        // lands. The save starts here, where the keyboard is at hand.
        if self.editor.closing && self.editor.pending.is_none() && self.editor.zmk_dirty {
            self.start_save(keyboard);
        }

        self.poll_pending_write(ctx, keyboard);

        // The transient ZMK Studio session opens as soon as the window is up
        // (on the reader thread), so the first write does not wait on a
        // connection. Until it lands the picker stays disabled.
        if matches!(keyboard.write_support(), WriteSupport::Session) && !self.editor.closing {
            self.ensure_zmk_session(keyboard);
        }

        // While the close-save is in flight the window has no close button:
        // it dismisses itself when the save lands.
        let closing = self.editor.closing;
        let mut window = Window::new("Edit key")
            .resizable(true)
            .default_size(egui::vec2(480.0, 620.0))
            .min_size(egui::vec2(440.0, 320.0));
        let mut open = true;
        if !closing {
            window = window.open(&mut open);
        }
        // A fixed-ish, user-resizable window: panes scroll internally instead
        // of growing the window to their content height.
        window.show(ctx, |ui| {
            if let Some(error) = &self.editor.error {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
            }

            // Header: one key slot. It shows the current assignment
            // rendered as the exact key it paints on the overlay, with its
            // raw firmware value beneath. While a staged draft is mid-
            // selection and not yet a valid binding, the slot instead
            // shows the ghosted draft with "Invalid" beneath it — a valid
            // draft applies instantly, so it never lingers visible.
            let style = self.paint_style(PREVIEW_KEY_UNIT);
            let action = keyboard.get_action(target.layer_index, target.row, target.col);
            let layout_key = action
                .as_ref()
                .and_then(|a| a.resolve_label(&layer_names))
                .unwrap_or_default();
            // Kind is taken from the layer-0 key at this position, matching
            // the overlay's coloring rule for Modifier/Special darkening.
            let kind = keyboard
                .get_key(0, target.row, target.col)
                .map(|k| k.kind)
                .unwrap_or(crate::layout_key::KeycodeKind::Basic);
            let ghost = match action.as_ref() {
                Some(KeyAction::Qmk(_)) => self.editor.qmk_draft.ghost_action(),
                Some(KeyAction::Zmk(_)) => self.editor.zmk_draft.ghost_action(),
                _ => None,
            };
            let (mut display_key, raw_text, ghosted) = match &ghost {
                Some(ghost_action) => (
                    ghost_action.resolve_label(&layer_names).unwrap_or_default(),
                    "Invalid".to_string(),
                    true,
                ),
                None => {
                    let raw_text = match &action {
                        Some(action) => raw_value_text(action),
                        None => "No binding at this key".to_string(),
                    };
                    (layout_key, raw_text, action.is_none())
                }
            };
            // Colors follow the overlay rules; a ghosted draft or unbound
            // slot renders ghosted, like a pinned transparent binding.
            let colors_for = |layer: u8| style.colors_for(layer, kind, false, false);
            let mut colors = if ghosted {
                colors_for(display_key.layer_ref.unwrap_or(target.layer_index as u8)).ghosted()
            } else {
                colors_for(display_key.layer_ref.unwrap_or(target.layer_index as u8))
            };
            // The invalid draft speaks for itself: a circle-x inside the
            // key, plus a dashed amber outline no other key state uses
            // (the styled border derives its color from the fill).
            if ghost.is_some() {
                display_key.tap = Label::new(egui_phosphor::regular::SMILEY_X_EYES);
                display_key.border = BorderStyle::Dashed;
                colors.fill = colors
                    .fill
                    .lerp_to_gamma(egui::Color32::from_rgb(220, 180, 60), 0.45);
            }

            let status_top = ui.cursor().top();
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                let (_, cell) = ui.allocate_exact_size(
                    egui::vec2(PREVIEW_KEY_UNIT, PREVIEW_KEY_UNIT),
                    egui::Sense::hover(),
                );
                key_paint::paint(
                    ui,
                    cell.rect,
                    0.0,
                    &KeyDisplay {
                        key: &display_key,
                        colors,
                        hovered: false,
                        pressed: false,
                        shift_held: false,
                        ralt_held: false,
                    },
                    &style,
                );
                ui.weak(raw_text);
            });

            // The session status floats in the header's top-right corner: an
            // absolute child UI paints there without advancing the layout,
            // so the centered preview never moves for it.
            let status_corner = egui::Rect::from_min_max(
                egui::pos2(ui.max_rect().left(), status_top),
                egui::pos2(ui.max_rect().right(), status_top + 24.0),
            );
            let mut status_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(status_corner)
                    .layout(egui::Layout::right_to_left(egui::Align::Min)),
            );
            self.draw_session_status(&mut status_ui);

            // While closing, the save is in flight; nothing here may
            // start another write that would land after the save.
            if closing {
                ui.disable();
            }
            let write_support = keyboard.write_support();
            match (write_support, action.as_ref()) {
                (WriteSupport::Immediate, Some(KeyAction::Qmk(_))) => {
                    ui.add_space(8.0);
                    let mut draft = self.editor.qmk_draft.clone();
                    self.draw_qmk_editor_body(ui, keyboard, target, &mut draft);
                    self.editor.qmk_draft = draft;
                }
                (WriteSupport::Session, Some(KeyAction::Zmk(_))) => {
                    ui.add_space(8.0);
                    // The picker stays disabled until the Studio session is
                    // open, so no write can be requested before the device
                    // accepts them.
                    let session_ready = self.editor.zmk_session == ZmkSessionState::Ready;
                    ui.add_enabled_ui(session_ready, |ui| {
                        let mut draft = self.editor.zmk_draft.clone();
                        self.draw_zmk_editor_body(ui, keyboard, target, &mut draft);
                        self.editor.zmk_draft = draft;
                    });
                }
                _ => {
                    ui.add_space(8.0);
                    ui.weak("This key cannot be edited in this version.");
                }
            }
        });

        if !open {
            self.on_close_request();
        }
    }

    /// Handles the window's close button. ZMK sessions save first: the
    /// window stays up (spinner in the bar) until the save lands, then
    /// dismisses itself. A failed save revives the window with the error.
    fn on_close_request(&mut self) {
        self.editor.error = None;
        if self.editor.zmk_dirty {
            self.editor.closing = true;
        } else {
            self.close_editor();
        }
    }

    /// The session status, floated into the header's top-right corner: a
    /// spinner while the Studio session connects, an amber reminder while the
    /// device holds unsaved changes, and a spinner while a close-save is in
    /// flight.
    fn draw_session_status(&self, ui: &mut egui::Ui) {
        if self.editor.zmk_session == ZmkSessionState::Opening {
            ui.label("Connecting…");
            ui.add(egui::Spinner::new().size(14.0));
            return;
        }
        if !self.editor.zmk_dirty {
            return;
        }
        // The host child UI is laid out right-to-left so the group hugs the
        // corner's right edge; `Align::Min` keeps the row content-height.
        if self.editor.closing {
            ui.label("Saving…");
            ui.add(egui::Spinner::new().size(14.0));
        } else {
            ui.colored_label(egui::Color32::from_rgb(220, 180, 60), "Unsaved changes")
                .on_hover_text("Changes are saved automatically when this window closes.");
        }
    }

    fn start_save(&mut self, keyboard: &Keyboard) {
        if self.editor.pending.is_some() {
            return;
        }
        self.editor.pending = Some(keyboard.save_keymap());
        self.editor.pending_kind = Some(PendingKind::Save);
        self.editor.error = None;
    }

    /// Starts opening the transient ZMK Studio session, once per window
    /// lifetime: `Idle` arms it, `Opening` waits for the reader thread, and a
    /// `Failed` attempt is not retried until the next retarget or window
    /// reopen.
    fn ensure_zmk_session(&mut self, keyboard: &Keyboard) {
        if self.editor.zmk_session != ZmkSessionState::Idle || self.editor.pending.is_some() {
            return;
        }
        self.editor.zmk_session = ZmkSessionState::Opening;
        self.editor.pending = Some(keyboard.open_edit_session());
        self.editor.pending_kind = Some(PendingKind::Open);
        self.editor.error = None;
    }

    /// Starts writing a finished binding to the target key. ZMK writes are
    /// session changes tracked by the session bar; QMK writes go straight to
    /// the device. While a write is in flight the request waits in the
    /// last-write-wins queue instead of being dropped — the keymap worker
    /// serializes commands, so the queued write lands right after it. While
    /// closing, only such queued edits still arrive (the UI is disabled);
    /// they drain before the save.
    fn apply_write(&mut self, keyboard: &Keyboard, target: EditTarget, action: KeyAction) {
        if self.editor.pending.is_some() {
            self.editor.queued = Some((target, action));
            return;
        }
        let is_session = matches!(action, KeyAction::Zmk(_));
        let receiver = keyboard.set_key(target.layer_index, target.row, target.col, action);
        self.editor.pending = Some(receiver);
        if is_session {
            self.editor.pending_kind = Some(PendingKind::Set);
        }
        self.editor.error = None;
    }

    /// Polls an in-flight write once per frame, repainting while it is
    /// pending, and kicks off the queued write when the device is ready.
    /// While closing, the pipeline drains write → save → close.
    fn poll_pending_write(&mut self, ctx: &egui::Context, keyboard: &Keyboard) {
        let Some(receiver) = &self.editor.pending else {
            return;
        };
        let kind = self.editor.pending_kind;
        match receiver.try_recv() {
            Ok(Ok(())) => {
                self.editor.pending = None;
                self.editor.pending_kind = None;
                match kind {
                    Some(PendingKind::Open) => self.editor.zmk_session = ZmkSessionState::Ready,
                    Some(PendingKind::Set) => self.editor.zmk_dirty = true,
                    Some(PendingKind::Save) => self.editor.zmk_dirty = false,
                    None => {}
                }
                if let Some((target, action)) = self.editor.queued.take() {
                    self.apply_write(keyboard, target, action);
                }
                if self.editor.closing {
                    if self.editor.zmk_dirty {
                        self.start_save(keyboard);
                    } else if self.editor.pending.is_none() {
                        self.close_editor();
                    }
                }
            }
            Ok(Err(e)) => {
                if kind == Some(PendingKind::Open) {
                    self.editor.zmk_session = ZmkSessionState::Failed;
                }
                self.editor.pending = None;
                self.editor.pending_kind = None;
                self.editor.queued = None;
                self.editor.closing = false;
                self.editor.error = Some(e);
            }
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.editor.pending = None;
                self.editor.pending_kind = None;
                self.editor.queued = None;
                self.editor.closing = false;
                self.editor.error = Some("Connection lost".to_string());
            }
        }
    }
}

#[cfg(test)]
mod window_growth_probe {
    use egui::{Align, Color32, Layout, RawInput, Rect, Sense, Vec2};

    /// Mirrors the Edit key window's structure: header with the session
    /// status floated into its top-right corner, left panel + central panel,
    /// both with tall scroll content. Returns the window height per frame.
    #[expect(deprecated)] // `Context::run` is the headless-friendly pass driver
    fn editor_window_heights(dirty_bar: bool) -> Vec<f32> {
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(1920.0, 1080.0));
        let mut heights = Vec::new();
        for frame in 0..30 {
            let _ = ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 * 0.016),
                    ..Default::default()
                },
                |ctx| {
                    let mut open = true;
                    let response = egui::Window::new("Edit key")
                        .open(&mut open)
                        .resizable(true)
                        .default_size(Vec2::new(480.0, 620.0))
                        .min_size(Vec2::new(440.0, 320.0))
                        .show(ctx, |ui| {
                            let status_top = ui.cursor().top();
                            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                                ui.allocate_exact_size(Vec2::splat(68.0), Sense::hover());
                                ui.weak("0x1234");
                            });
                            if dirty_bar {
                                let status_corner = Rect::from_min_max(
                                    egui::pos2(ui.max_rect().left(), status_top),
                                    egui::pos2(ui.max_rect().right(), status_top + 24.0),
                                );
                                let mut status_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(status_corner)
                                        .layout(Layout::right_to_left(Align::Min)),
                                );
                                let _ = status_ui.button("Discard");
                                let _ = status_ui.button("Save");
                                status_ui.colored_label(
                                    Color32::from_rgb(220, 180, 60),
                                    "Unsaved changes",
                                );
                            }
                            egui::Panel::left("zmk_kinds")
                                .resizable(false)
                                .exact_size(110.0)
                                .show_separator_line(false)
                                .show_inside(ui, |ui| {
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        for i in 0..20 {
                                            ui.label(format!("entry {i}"));
                                        }
                                    });
                                });
                            egui::CentralPanel::default().show_inside(ui, |ui| {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for i in 0..300 {
                                        ui.label(format!("row {i}"));
                                    }
                                });
                            });
                        });
                    if let Some(rect) = response.map(|r| r.response.rect) {
                        heights.push(rect.height());
                    }
                },
            );
        }
        heights
    }

    /// The session status floats over the header; with it visible the window
    /// must hold its size instead of ratcheting taller every frame.
    #[test]
    fn session_status_does_not_grow_the_window() {
        let heights = editor_window_heights(true);
        let first = heights[0];
        let worst = heights.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            worst - first < 2.0,
            "window grew from {first:.0} to {worst:.0}"
        );
    }

    /// Same structure without the bar, as the baseline.
    #[test]
    fn window_is_stable_without_the_session_bar() {
        let heights = editor_window_heights(false);
        let first = heights[0];
        let worst = heights.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            worst - first < 2.0,
            "window grew from {first:.0} to {worst:.0}"
        );
    }
}
