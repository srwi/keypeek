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

/// What kind of write an in-flight `pending` receiver belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingKind {
    Set,
    Save,
    Discard,
}

enum PromptAnswer {
    Save,
    Discard,
    Cancel,
}

/// Editor window state, owned by `OverlayApp`.
pub struct EditorState {
    /// `None` = window closed.
    pub target: Option<EditTarget>,
    /// An in-flight write; polled each frame.
    pub pending: Option<mpsc::Receiver<Result<(), String>>>,
    /// What the in-flight write is for.
    pub pending_kind: Option<PendingKind>,
    /// Last write/read error shown in the window.
    pub error: Option<String>,
    /// Per-firmware draft state, rebuilt on each retarget.
    pub qmk_draft: QmkDraft,
    pub zmk_draft: ZmkDraft,
    /// ZMK: whether there are unsaved changes in device RAM.
    pub zmk_dirty: bool,
    /// ZMK: a settings-close was requested while dirty; show the save prompt.
    pub close_prompt: bool,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            target: None,
            pending: None,
            pending_kind: None,
            error: None,
            qmk_draft: QmkDraft::default(),
            zmk_draft: ZmkDraft::default(),
            zmk_dirty: false,
            close_prompt: false,
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

        self.poll_pending_write(ctx);

        let mut open = true;
        // A fixed-ish, user-resizable window: panes scroll internally instead
        // of growing the window to their content height.
        Window::new("Edit key")
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(480.0, 620.0))
            .min_size(egui::vec2(440.0, 320.0))
            .show(ctx, |ui| {
                // Header: the current assignment rendered as the exact key it
                // paints on the overlay, with its raw firmware value beneath.
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
                // Colors follow the overlay rules; an unbound slot renders
                // ghosted, like a pinned transparent binding.
                let colors_for = |layer: u8| style.colors_for(layer, kind, false, false);
                let colors = if action.is_none() {
                    colors_for(layout_key.layer_ref.unwrap_or(target.layer_index as u8)).ghosted()
                } else {
                    colors_for(layout_key.layer_ref.unwrap_or(target.layer_index as u8))
                };

                let raw_text = match &action {
                    Some(action) => raw_value_text(action),
                    None => "No binding at this key".to_string(),
                };

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
                            key: &layout_key,
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

                let write_support = keyboard.write_support();
                match (write_support, action.as_ref()) {
                    (WriteSupport::Immediate, Some(KeyAction::Qmk(_))) => {
                        ui.add_space(8.0);
                        ui.separator();
                        let mut draft = self.editor.qmk_draft.clone();
                        self.draw_qmk_editor_body(ui, keyboard, target, &mut draft);
                        self.editor.qmk_draft = draft;
                    }
                    (WriteSupport::Session, Some(KeyAction::Zmk(_))) => {
                        ui.add_space(8.0);
                        ui.separator();
                        let mut draft = self.editor.zmk_draft.clone();
                        self.draw_zmk_editor_body(ui, keyboard, target, &mut draft);
                        self.editor.zmk_draft = draft;
                        self.draw_save_bar(ui, keyboard);
                    }
                    _ => {
                        ui.add_space(8.0);
                        ui.weak("This key cannot be edited in this version.");
                    }
                }

                if let Some(error) = &self.editor.error {
                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
                }
            });

        if !open {
            self.close_editor_window();
        }
    }

    /// The Unsaved changes / Save / Discard bar for ZMK sessions.
    fn draw_save_bar(&mut self, ui: &mut egui::Ui, keyboard: &Keyboard) {
        if !self.editor.zmk_dirty {
            return;
        }
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(220, 180, 60), "Unsaved changes");
            if ui.button("Save").clicked() {
                self.start_save(keyboard);
            }
            if ui.button("Discard").clicked() {
                self.start_discard(keyboard);
            }
        });
    }

    fn start_save(&mut self, keyboard: &Keyboard) {
        if self.editor.pending.is_some() {
            return;
        }
        self.editor.pending = Some(keyboard.save_keymap());
        self.editor.pending_kind = Some(PendingKind::Save);
        self.editor.error = None;
    }

    fn start_discard(&mut self, keyboard: &Keyboard) {
        if self.editor.pending.is_some() {
            return;
        }
        self.editor.pending = Some(keyboard.discard_keymap());
        self.editor.pending_kind = Some(PendingKind::Discard);
        self.editor.error = None;
    }

    /// Starts writing a finished binding to the target key, unless one is
    /// already in flight. ZMK writes are session changes tracked by the save
    /// bar; QMK writes go straight to the device.
    fn apply_write(&mut self, keyboard: &Keyboard, target: EditTarget, action: KeyAction) {
        if self.editor.pending.is_some() {
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

    /// Polls an in-flight write once per frame, repainting while it is pending.
    fn poll_pending_write(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.editor.pending else {
            return;
        };
        let kind = self.editor.pending_kind;
        match receiver.try_recv() {
            Ok(Ok(())) => {
                self.editor.pending = None;
                self.editor.pending_kind = None;
                match kind {
                    Some(PendingKind::Set) => self.editor.zmk_dirty = true,
                    Some(PendingKind::Save) | Some(PendingKind::Discard) => {
                        self.editor.zmk_dirty = false
                    }
                    None => {}
                }
            }
            Ok(Err(e)) => {
                self.editor.pending = None;
                self.editor.pending_kind = None;
                self.editor.error = Some(e);
            }
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.editor.pending = None;
                self.editor.pending_kind = None;
                self.editor.error = Some("Connection lost".to_string());
            }
        }
    }

    /// Modal shown when settings are closed with unsaved ZMK changes.
    pub(super) fn draw_close_prompt(&mut self, ctx: &egui::Context, keyboard: &Keyboard) {
        if !self.editor.close_prompt {
            return;
        }
        let mut answer = None;
        egui::Window::new("Unsaved changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("You have unsaved keymap changes on the keyboard.");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        answer = Some(PromptAnswer::Save);
                    }
                    if ui.button("Discard").clicked() {
                        answer = Some(PromptAnswer::Discard);
                    }
                    if ui.button("Cancel").clicked() {
                        answer = Some(PromptAnswer::Cancel);
                    }
                });
            });

        match answer {
            Some(PromptAnswer::Save) => {
                self.editor.pending = Some(keyboard.save_keymap());
                self.editor.pending_kind = Some(PendingKind::Save);
                self.finish_close();
            }
            Some(PromptAnswer::Discard) => {
                self.editor.pending = Some(keyboard.discard_keymap());
                self.editor.pending_kind = Some(PendingKind::Discard);
                self.finish_close();
            }
            Some(PromptAnswer::Cancel) => {
                self.editor.close_prompt = false;
            }
            None => {}
        }
    }

    fn finish_close(&mut self) {
        self.editor.zmk_dirty = false;
        self.editor.close_prompt = false;
        self.ui.settings_visible = false;
        self.ui.pinned_layer = None;
        self.close_editor();
        self.persist_settings();
    }

    fn close_editor_window(&mut self) {
        self.editor.target = None;
        self.editor.pending = None;
        self.editor.pending_kind = None;
        self.editor.error = None;
        self.editor.qmk_draft = Default::default();
        self.editor.zmk_draft = Default::default();
        self.editor.zmk_dirty = false;
        self.editor.close_prompt = false;
    }
}
