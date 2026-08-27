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
use crate::keyboard::Keyboard;
use crate::protocols::WriteSupport;
use egui::Window;
use std::sync::mpsc;

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

/// Text describing a key's current assignment for the editor header: the
/// resolved label(s) plus the raw firmware form.
pub fn describe_action(action: &KeyAction, layer_names: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(label) = action.resolve_label(layer_names) {
        if !label.tap.full.is_empty() {
            parts.push(label.tap.full.clone());
        }
        if let Some(symbol) = &label.symbol {
            if !symbol.is_empty() {
                parts.push(symbol.clone());
            }
        }
        for strip in [&label.behavior, &label.argument] {
            if let Some(strip) = strip {
                if !strip.full.is_empty() {
                    parts.push(strip.full.clone());
                }
            }
        }
    }

    let raw = match action {
        KeyAction::Qmk(code) => format!("0x{code:04X}"),
        KeyAction::Zmk(behavior) => format!("{behavior:?}"),
    };
    parts.push(raw);

    parts.join("  ")
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
        let layer_label = keyboard
            .layer_infos()
            .get(target.layer_index)
            .and_then(|info| info.name.clone())
            .unwrap_or_else(|| format!("Layer {}", target.layer_index));

        self.poll_pending_write(ctx);

        let mut open = true;
        Window::new("Edit key")
            .open(&mut open)
            .resizable(false)
            .default_width(340.0)
            .max_width(360.0)
            .min_width(300.0)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Layer {layer_label}  ·  key ({}, {})",
                    target.row, target.col
                ));
                ui.add_space(4.0);

                match keyboard.get_action(target.layer_index, target.row, target.col) {
                    Some(action) => {
                        let text = describe_action(&action, &layer_names);
                        ui.weak(format!("Current: {text}"));
                    }
                    None => {
                        ui.weak("No binding at this key");
                    }
                }

                let write_support = keyboard.write_support();
                match (
                    write_support,
                    keyboard.get_action(target.layer_index, target.row, target.col),
                ) {
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
        self.editor.close_prompt = false;
    }
}
