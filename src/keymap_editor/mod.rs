//! "Edit key" window. Displays and updates the key selected on the overlay.

mod picker;
mod qmk_catalog;
mod qmk_editor;
mod zmk_catalog;
mod zmk_editor;
pub use picker::KEY_UNIT;
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

impl EditTarget {
    /// Returns the current key action at this target position.
    pub fn action(self, keyboard: &Keyboard) -> Option<KeyAction> {
        keyboard.get_action(self.layer_index, self.row, self.col)
    }

    /// Sends a write command for this target key.
    pub fn set_key(
        self,
        keyboard: &Keyboard,
        action: KeyAction,
    ) -> mpsc::Receiver<Result<(), String>> {
        keyboard.set_key(self.layer_index, self.row, self.col, action)
    }
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

/// Blocking overlay state for connecting, saving, or failed operations.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum EditorOverlay {
    Connecting,
    Saving,
    Failed,
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
    /// Active search filter for key candidate groups.
    pub search_query: String,
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
            search_query: String::new(),
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

    /// Returns the active blocking overlay state, if any.
    pub fn overlay(&self) -> Option<EditorOverlay> {
        if self.closing {
            Some(EditorOverlay::Saving)
        } else if self.zmk_session == ZmkSessionState::Opening {
            Some(EditorOverlay::Connecting)
        } else if self.zmk_session == ZmkSessionState::Failed {
            Some(EditorOverlay::Failed)
        } else {
            None
        }
    }

    /// Starts a background operation and clears any previous error.
    pub fn start_task(&mut self, kind: PendingKind, receiver: mpsc::Receiver<Result<(), String>>) {
        self.pending = Some(PendingTask { kind, receiver });
        self.error = None;
    }

    /// Updates session or dirty flags upon successful completion of a background operation.
    pub fn complete_task(&mut self, kind: PendingKind) {
        match kind {
            PendingKind::Open => self.zmk_session = ZmkSessionState::Ready,
            PendingKind::Set => self.zmk_dirty = true,
            PendingKind::Save => self.zmk_dirty = false,
        }
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
        self.search_query.clear();
        if self.zmk_session == ZmkSessionState::Failed {
            self.zmk_session = ZmkSessionState::Idle;
        }
        match target.action(keyboard) {
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

    /// Resets the QMK draft for the given section.
    pub fn reset_qmk_section(
        &mut self,
        section: qmk_editor::Section,
        current_action: Option<&KeyAction>,
    ) {
        self.qmk_draft = QmkDraft::for_section(section, current_action);
    }

    /// Resets the ZMK draft for the given behavior kind.
    pub fn reset_zmk_kind(
        &mut self,
        kind: zmk_catalog::ZmkBehaviorKind,
        current_action: Option<&KeyAction>,
    ) {
        self.zmk_draft = ZmkDraft::for_kind(kind, current_action);
    }
}

/// Width of the left category panel in pixels.
const SIDEBAR_WIDTH: f32 = 110.0;
/// Right margin to prevent scrollbar overlap with group borders.
const SCROLLBAR_GUTTER: f32 = 8.0;

/// Section grouping for the editor's left sidebar.
pub(super) struct SidebarSection<T: 'static> {
    pub title: &'static str,
    pub items: &'static [T],
}

/// Item in the editor's left sidebar.
pub(super) trait SidebarItem: Copy + PartialEq {
    fn label(self) -> &'static str;
    fn is_supported(self, keyboard: &Keyboard) -> bool;
}

/// Draws the left category panel with sectioned items and a search bar pinned to the bottom.
pub(super) fn editor_left_panel<T: SidebarItem>(
    ui: &mut egui::Ui,
    left_id: &str,
    keyboard: &Keyboard,
    current: T,
    sections: &[SidebarSection<T>],
    search_query: &mut String,
) -> Option<T> {
    let mut selected = None;
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
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(2.0);
                picker::search_bar(ui, search_query);
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            for section in sections {
                                let supported: Vec<T> = section
                                    .items
                                    .iter()
                                    .copied()
                                    .filter(|item| item.is_supported(keyboard))
                                    .collect();
                                if supported.is_empty() {
                                    continue;
                                }
                                ui.weak(section.title);
                                for item in supported {
                                    if ui.selectable_label(current == item, item.label()).clicked()
                                    {
                                        selected = Some(item);
                                    }
                                }
                                ui.add_space(4.0);
                            }
                        });
                    });
                });
            });
        });
    selected
}

/// The editor's scrolling central panel.
pub(super) fn editor_central_panel(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    content: impl FnOnce(&mut egui::Ui),
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
            left: 4,
            right: 0,
            top: 0,
            bottom: 0,
        }))
        .show(ui, |ui| {
            ui.push_id(&id_salt, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(&id_salt)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let content_width = (ui.available_width() - SCROLLBAR_GUTTER).max(100.0);
                        ui.set_max_width(content_width);
                        content(ui);
                    });
            });
        });
}

impl EditorState {
    /// Draws the edit key window.
    pub fn draw_window(
        &mut self,
        ctx: &egui::Context,
        keyboard: &Keyboard,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let Some(target) = self.target else {
            return;
        };

        // Save unsaved ZMK changes before closing the window.
        if self.closing && self.pending.is_none() && self.zmk_dirty {
            self.start_save(keyboard);
        }

        self.poll_pending_write(ctx, keyboard);

        if matches!(keyboard.write_support(), WriteSupport::Session) && !self.closing {
            self.ensure_zmk_session(keyboard);
        }

        let closing = self.closing;
        let title = if self.zmk_dirty {
            "Edit key (Unsaved changes)"
        } else {
            "Edit key"
        };
        let mut window = Window::new(title)
            .id(egui::Id::new("edit_key_window"))
            .resizable(true)
            .default_size(egui::vec2(440.0, 525.0))
            .min_size(egui::vec2(440.0, 525.0));
        let mut open = true;
        if !closing {
            window = window.open(&mut open);
        }
        window.show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
            }

            let is_enabled = self.overlay().is_none();
            ui.add_enabled_ui(is_enabled, |ui| {
                let target = self.draw_editor_header(ui, keyboard, target, style);

                let write_support = keyboard.write_support();
                match write_support {
                    WriteSupport::Immediate => {
                        ui.add_space(8.0);
                        self.draw_qmk_editor_body(ui, keyboard, target, style);
                    }
                    WriteSupport::Session => {
                        ui.add_space(8.0);
                        self.draw_zmk_editor_body(ui, keyboard, target, style);
                    }
                    WriteSupport::None => {
                        ui.add_space(8.0);
                        ui.weak("This key cannot be edited in this version.");
                    }
                }
            });

            self.draw_editor_overlay(ui);
        });

        if !open && self.request_close() {
            keyboard.end_edit_session();
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
            if target.action(keyboard).as_ref() != Some(&action) {
                self.apply_write(keyboard, target, action);
            }
        }
    }

    /// Draws the header row with layer switcher buttons spanning the full width.
    fn draw_editor_header(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        style: &crate::key_paint::KeyPaintStyle,
    ) -> EditTarget {
        let layer_infos = keyboard.layer_infos();
        let mut selected_layer = None;

        let layer_count = layer_infos.len().max(1);
        let item_spacing = ui.spacing().item_spacing.x;
        let total_spacing = (layer_count - 1) as f32 * item_spacing;
        let button_width = ((ui.available_width() - total_spacing) / layer_count as f32).max(24.0);

        ui.push_id("layer_switcher", |ui| {
            ui.horizontal(|ui| {
                for (i, info) in layer_infos.iter().enumerate() {
                    let label = info.short_name(i);
                    let is_selected = target.layer_index == i;
                    if layer_button(ui, &label, i, is_selected, button_width, style).clicked() {
                        selected_layer = Some(i);
                    }
                }
            });
        });

        if !self.closing {
            if let Some(new_layer) = selected_layer {
                if new_layer != target.layer_index {
                    let new_target = EditTarget {
                        layer_index: new_layer,
                        row: target.row,
                        col: target.col,
                    };
                    self.retarget(keyboard, new_target);
                    return new_target;
                }
            }
        }

        target
    }

    /// Draws a centered spinner and status text during connecting, saving, or failed operations.
    fn draw_editor_overlay(&mut self, ui: &mut egui::Ui) {
        let Some(overlay) = self.overlay() else {
            return;
        };

        let (msg, is_spinner, is_retry) = match overlay {
            EditorOverlay::Saving => ("Saving…", true, false),
            EditorOverlay::Connecting => ("Connecting…", true, false),
            EditorOverlay::Failed => ("Connection failed", false, true),
        };

        let window_rect = ui.max_rect();
        ui.scope_builder(egui::UiBuilder::new().max_rect(window_rect), |ui| {
            ui.vertical_centered(|ui| {
                let top_space = (window_rect.height() * 0.5 - 30.0).max(0.0);
                ui.add_space(top_space);
                if is_spinner {
                    ui.add(egui::Spinner::new().size(24.0));
                    ui.add_space(8.0);
                }
                ui.label(egui::RichText::new(msg).size(14.0).strong());
                if is_retry {
                    ui.add_space(8.0);
                    if ui.button("Retry").clicked() {
                        self.zmk_session = ZmkSessionState::Idle;
                        self.error = None;
                    }
                }
            });
        });
    }

    fn start_save(&mut self, keyboard: &Keyboard) {
        if self.pending.is_some() {
            return;
        }
        self.start_task(PendingKind::Save, keyboard.save_keymap());
    }

    /// Starts the ZMK Studio session connection if idle.
    fn ensure_zmk_session(&mut self, keyboard: &Keyboard) {
        if self.zmk_session != ZmkSessionState::Idle || self.pending.is_some() {
            return;
        }
        self.zmk_session = ZmkSessionState::Opening;
        self.start_task(PendingKind::Open, keyboard.open_edit_session());
    }

    /// Sends a write command to the device, or queues it if an operation is in progress.
    fn apply_write(&mut self, keyboard: &Keyboard, target: EditTarget, action: KeyAction) {
        if self.pending.is_some() {
            self.queued = Some((target, action));
            return;
        }
        let receiver = target.set_key(keyboard, action);
        self.start_task(PendingKind::Set, receiver);
    }

    /// Polls active background operations and processes queued write commands.
    fn poll_pending_write(&mut self, ctx: &egui::Context, keyboard: &Keyboard) {
        let Some(task) = &self.pending else {
            return;
        };
        match task.receiver.try_recv() {
            Ok(Ok(())) => {
                if let Some(task) = self.pending.take() {
                    self.complete_task(task.kind);
                }
                if let Some((target, action)) = self.queued.take() {
                    self.apply_write(keyboard, target, action);
                }
                if self.closing {
                    if self.zmk_dirty {
                        self.start_save(keyboard);
                    } else if self.pending.is_none() {
                        self.reset();
                        keyboard.end_edit_session();
                    }
                }
            }
            Ok(Err(e)) => {
                self.fail_task(e);
            }
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.fail_task("Connection lost");
            }
        }
    }
}

fn layer_button(
    ui: &mut egui::Ui,
    label: &str,
    layer_index: usize,
    selected: bool,
    width: f32,
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
        .min_size(egui::vec2(width, 22.0));

    let response = ui.push_id(layer_index, |ui| ui.add(button)).inner;
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
