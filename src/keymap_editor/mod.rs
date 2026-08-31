//! "Edit key" window. Displays and updates the key selected on the overlay.

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

/// Target key position in the keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditTarget {
    pub layer_index: usize,
    pub row: usize,
    pub col: usize,
}

/// Operation type of an active background task.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingKind {
    Open,
    Set,
    Save,
}

/// Connection state of the ZMK Studio session.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ZmkSessionState {
    /// No active session.
    Idle,
    /// Session is connecting.
    Opening,
    /// Session is connected and ready for writes.
    Ready,
    /// Session connection failed.
    Failed,
}

/// Active background operation and its kind.
pub struct PendingTask {
    pub kind: PendingKind,
    pub receiver: mpsc::Receiver<Result<(), String>>,
}

/// Editor window state, owned by `OverlayApp`.
pub struct EditorState {
    /// Active target key, or `None` when the window is closed.
    pub target: Option<EditTarget>,
    /// Active background operation.
    pub pending: Option<PendingTask>,
    /// Queued write operation to send when the current operation completes.
    pub queued: Option<(EditTarget, KeyAction)>,
    /// Error message to display in the window.
    pub error: Option<String>,
    /// Draft state for QMK keycodes.
    pub qmk_draft: QmkDraft,
    /// Draft state for ZMK behaviors.
    pub zmk_draft: ZmkDraft,
    /// Indicates unsaved ZMK changes on the device.
    pub zmk_dirty: bool,
    /// State of the ZMK Studio session.
    pub zmk_session: ZmkSessionState,
    /// Indicates the window is saving changes before closing.
    pub closing: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            target: None,
            pending: None,
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

impl EditorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the editor state to default values.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Starts a background operation and clears any previous error.
    pub fn start_task(&mut self, kind: PendingKind, receiver: mpsc::Receiver<Result<(), String>>) {
        self.pending = Some(PendingTask { kind, receiver });
        self.error = None;
    }

    /// Clears the active background task and records an error, cancelling queued and closing states.
    pub fn fail_task(&mut self, error: impl Into<String>) {
        if let Some(task) = self.pending.take() {
            if task.kind == PendingKind::Open {
                self.zmk_session = ZmkSessionState::Failed;
            }
        }
        self.queued = None;
        self.closing = false;
        self.error = Some(error.into());
    }

    /// Requests window close. If ZMK has unsaved changes, starts a save operation
    /// before closing. Returns `true` if closed immediately.
    pub fn request_close(&mut self) -> bool {
        self.error = None;
        if self.zmk_dirty {
            self.closing = true;
            false
        } else {
            self.reset();
            true
        }
    }

    /// Sets the target key and loads its current binding into the draft.
    pub fn retarget(&mut self, keyboard: &Keyboard, target: EditTarget) {
        self.target = Some(target);
        self.error = None;
        if self.zmk_session == ZmkSessionState::Failed {
            self.zmk_session = ZmkSessionState::Idle;
        }
        match keyboard.get_action(target.layer_index, target.row, target.col) {
            Some(KeyAction::Qmk(code)) => {
                self.qmk_draft = QmkDraft::from_keycode(code);
                self.zmk_draft = Default::default();
            }
            Some(KeyAction::Zmk(behavior)) => {
                self.zmk_draft = ZmkDraft::from_behavior(&behavior);
                self.qmk_draft = Default::default();
            }
            _ => {
                self.qmk_draft = Default::default();
                self.zmk_draft = Default::default();
            }
        }
    }
}

/// Width of the left category panel in pixels.
const SIDEBAR_WIDTH: f32 = 110.0;
/// Right margin to prevent scrollbar overlap with group borders.
const SCROLLBAR_GUTTER: f32 = 8.0;

/// Draws the left category panel.
pub(super) fn editor_left_panel(
    ui: &mut egui::Ui,
    left_id: &str,
    content: impl FnOnce(&mut egui::Ui),
) {
    let left_id = egui::Id::new(left_id);
    egui::Panel::left(left_id)
        .resizable(false)
        .exact_size(SIDEBAR_WIDTH)
        .show_separator_line(false)
        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
            left: 0,
            right: 8,
            top: 0,
            bottom: 0,
        }))
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), content);
            });
        });
}

/// The editor's scrolling central panel.
pub(super) fn editor_central_panel(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
            left: 4,
            right: 0,
            top: 0,
            bottom: 0,
        }))
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let content_width = (ui.available_width() - SCROLLBAR_GUTTER).max(100.0);
                    ui.set_max_width(content_width);
                    content(ui);
                });
        });
}

impl crate::overlay_window::OverlayApp {
    /// Draws the edit key window.
    pub(super) fn draw_editor_window(&mut self, ctx: &egui::Context, keyboard: &Keyboard) {
        let Some(target) = self.editor.target else {
            return;
        };

        // Save unsaved ZMK changes before closing the window.
        if self.editor.closing && self.editor.pending.is_none() && self.editor.zmk_dirty {
            self.start_save(keyboard);
        }

        self.poll_pending_write(ctx, keyboard);

        if matches!(keyboard.write_support(), WriteSupport::Session) && !self.editor.closing {
            self.ensure_zmk_session(keyboard);
        }

        let closing = self.editor.closing;
        let mut window = Window::new("Edit key")
            .resizable(true)
            .default_size(egui::vec2(480.0, 620.0))
            .min_size(egui::vec2(440.0, 320.0));
        let mut open = true;
        if !closing {
            window = window.open(&mut open);
        }
        window.show(ctx, |ui| {
            if let Some(error) = &self.editor.error {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
            }

            let target = self.draw_editor_header(ui, keyboard, target);

            // Disable input while saving before close.
            if closing {
                ui.disable();
            }
            let action = keyboard.get_action(target.layer_index, target.row, target.col);
            let write_support = keyboard.write_support();
            match (write_support, action.as_ref()) {
                (WriteSupport::Immediate, Some(KeyAction::Qmk(_))) => {
                    ui.add_space(8.0);
                    self.draw_qmk_editor_body(ui, keyboard, target);
                }
                (WriteSupport::Session, Some(KeyAction::Zmk(_))) => {
                    ui.add_space(8.0);
                    let session_ready = self.editor.zmk_session == ZmkSessionState::Ready;
                    ui.add_enabled_ui(session_ready, |ui| {
                        self.draw_zmk_editor_body(ui, keyboard, target);
                    });
                }
                _ => {
                    ui.add_space(8.0);
                    ui.weak("This key cannot be edited in this version.");
                }
            }
        });

        if !open {
            self.request_close_editor();
        }
    }

    /// Applies a staged binding if it is complete and different from the current key.
    fn commit_staged(
        &mut self,
        keyboard: &Keyboard,
        target: EditTarget,
        staged: Option<KeyAction>,
    ) {
        if let Some(action) = staged {
            if keyboard
                .get_action(target.layer_index, target.row, target.col)
                .as_ref()
                != Some(&action)
            {
                self.apply_write(keyboard, target, action);
            }
        }
    }

    /// Draws the header row with layer switcher buttons and session status.
    fn draw_editor_header(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
    ) -> EditTarget {
        let layer_infos = keyboard.layer_infos();
        let style = self.paint_style(picker::KEY_UNIT);
        let mut selected_layer = None;

        ui.horizontal(|ui| {
            for (i, info) in layer_infos.iter().enumerate() {
                let label = info.short_name(i);
                let is_selected = target.layer_index == i;
                if layer_button(ui, &label, i, is_selected, &style).clicked() {
                    selected_layer = Some(i);
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.draw_session_status(ui);
            });
        });

        if !self.editor.closing {
            if let Some(new_layer) = selected_layer {
                if new_layer != target.layer_index {
                    let new_target = EditTarget {
                        layer_index: new_layer,
                        row: target.row,
                        col: target.col,
                    };
                    self.editor.retarget(keyboard, new_target);
                    return new_target;
                }
            }
        }

        target
    }

    pub(crate) fn retarget_editor(&mut self, keyboard: &Keyboard, target: EditTarget) {
        self.editor.retarget(keyboard, target);
    }

    /// Draws the ZMK session status indicator.
    fn draw_session_status(&mut self, ui: &mut egui::Ui) {
        if self.editor.zmk_session == ZmkSessionState::Opening {
            ui.label("Connecting…");
            ui.add(egui::Spinner::new().size(14.0));
            return;
        }
        if self.editor.zmk_session == ZmkSessionState::Failed {
            if ui.button("Retry").clicked() {
                self.editor.zmk_session = ZmkSessionState::Idle;
                self.editor.error = None;
            }
            return;
        }
        if !self.editor.zmk_dirty {
            return;
        }
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
        self.editor
            .start_task(PendingKind::Save, keyboard.save_keymap());
    }

    /// Starts the ZMK Studio session connection if idle.
    fn ensure_zmk_session(&mut self, keyboard: &Keyboard) {
        if self.editor.zmk_session != ZmkSessionState::Idle || self.editor.pending.is_some() {
            return;
        }
        self.editor.zmk_session = ZmkSessionState::Opening;
        self.editor
            .start_task(PendingKind::Open, keyboard.open_edit_session());
    }

    /// Sends a write command to the device, or queues it if an operation is in progress.
    fn apply_write(&mut self, keyboard: &Keyboard, target: EditTarget, action: KeyAction) {
        if self.editor.pending.is_some() {
            self.editor.queued = Some((target, action));
            return;
        }
        let receiver = keyboard.set_key(target.layer_index, target.row, target.col, action);
        self.editor.start_task(PendingKind::Set, receiver);
    }

    /// Polls active background operations and processes queued write commands.
    fn poll_pending_write(&mut self, ctx: &egui::Context, keyboard: &Keyboard) {
        let Some(task) = &self.editor.pending else {
            return;
        };
        match task.receiver.try_recv() {
            Ok(Ok(())) => {
                let kind = self.editor.pending.take().map(|t| t.kind);
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
                self.editor.fail_task(e);
            }
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.editor.fail_task("Connection lost");
            }
        }
    }
}

fn layer_button(
    ui: &mut egui::Ui,
    label: &str,
    layer_index: usize,
    selected: bool,
    style: &crate::key_paint::KeyPaintStyle,
) -> egui::Response {
    let colors = style.colors_for(
        layer_index as u8,
        crate::layout_key::KeycodeKind::Modifier,
        false,
        selected,
    );
    let text = egui::RichText::new(label).color(colors.font).size(12.0);

    let stroke_width = if selected { 2.0_f32 } else { 1.0_f32 };
    let button = egui::Button::new(text)
        .fill(colors.fill)
        .stroke(egui::Stroke::new(stroke_width, colors.border))
        .corner_radius(4.0)
        .min_size(egui::vec2(32.0, 22.0));

    let response = ui.add(button);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        let highlight_stroke = egui::Stroke::new(stroke_width, colors.highlight_border());
        ui.painter().rect_stroke(
            response.rect,
            4.0,
            highlight_stroke,
            egui::StrokeKind::Inside,
        );
    }
    response
}
