//! "Edit key" window. Displays and updates the key selected on the overlay.

pub use keypeek_core::keymap_editor::{candidate, catalog, draft, profile};
mod editor;
pub(crate) mod picker;

pub use draft::KeyDraft;
pub use picker::KEY_UNIT;
pub use profile::{EditorProfile, LayerTapTarget, SidebarSection};

use egui::Window;
use keypeek_core::{KeySpec, KeycodeKind};
use keypeek_protocol::{Keyboard, WriteSupport};
use std::sync::mpsc;

/// Target key position in the keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditTarget {
    pub layer_index: usize,
    pub row: usize,
    pub col: usize,
}

impl EditTarget {
    pub const fn new(layer_index: usize, row: usize, col: usize) -> Self {
        Self {
            layer_index,
            row,
            col,
        }
    }

    pub const fn with_layer(self, layer_index: usize) -> Self {
        Self {
            layer_index,
            ..self
        }
    }

    /// Returns the current key action at this target position.
    pub fn action(self, keyboard: &Keyboard) -> Option<KeySpec> {
        keyboard.get_action(self.layer_index, self.row, self.col)
    }

    /// Sends a write command for this target key.
    pub fn set_key(
        self,
        keyboard: &Keyboard,
        action: KeySpec,
    ) -> mpsc::Receiver<Result<(), String>> {
        keyboard.set_key(self.layer_index, self.row, self.col, action)
    }
}

/// Operation type of an active background task.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingKind {
    AcquireLock,
    Set,
    Save,
}

/// Lock state for protocols requiring an explicit hardware edit lock ahead of writes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditLockState {
    /// No active edit lock.
    Idle,
    /// Lock request is in flight.
    Acquiring,
    /// Edit lock is active and ready for writes.
    Locked,
    /// Acquiring edit lock failed.
    Failed,
}

/// Active background operation and its kind.
pub struct PendingTask {
    pub kind: PendingKind,
    pub receiver: mpsc::Receiver<Result<(), String>>,
}

/// Blocking overlay state for locking, saving, or failed operations.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum EditorOverlay {
    AcquiringLock,
    Saving,
    Failed,
}

/// Editor window state, owned by application coordinators (`DesktopOverlayApp` / `WebOverlayApp`).
pub struct EditorState {
    /// Active target key, or `None` when the window is closed.
    pub target: Option<EditTarget>,
    /// Active background operation.
    pub pending: Option<PendingTask>,
    /// Queued write operation to send when the current operation completes.
    pub queued: Option<(EditTarget, KeySpec)>,
    /// Error message to display in the window.
    pub error: Option<String>,
    /// Draft state for the active key editor.
    pub draft: KeyDraft,
    /// Indicates unsaved changes on the device.
    pub dirty: bool,
    /// State of the hardware edit lock.
    pub lock_state: EditLockState,
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
            draft: KeyDraft::default(),
            dirty: false,
            lock_state: EditLockState::Idle,
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

    /// Resets the editor state and releases the keyboard edit lock if a keyboard is provided.
    pub fn close(&mut self, keyboard: Option<&Keyboard>) {
        self.reset();
        if let Some(kbd) = keyboard {
            kbd.release_edit_lock();
        }
    }

    /// Returns the active blocking overlay state, if any.
    pub fn overlay(&self) -> Option<EditorOverlay> {
        if self.closing {
            Some(EditorOverlay::Saving)
        } else if self.lock_state == EditLockState::Acquiring {
            Some(EditorOverlay::AcquiringLock)
        } else if self.lock_state == EditLockState::Failed {
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

    /// Updates lock or dirty flags upon successful completion of a background operation.
    pub fn complete_task(&mut self, kind: PendingKind) {
        match kind {
            PendingKind::AcquireLock => self.lock_state = EditLockState::Locked,
            PendingKind::Set => self.dirty = true,
            PendingKind::Save => self.dirty = false,
        }
    }

    /// Clears the active background task and records an error, cancelling queued and closing states.
    pub fn fail_task(&mut self, error: impl Into<String>) {
        if let Some(task) = self.pending.take() {
            if task.kind == PendingKind::AcquireLock {
                self.lock_state = EditLockState::Failed;
            }
        }
        self.queued = None;
        self.closing = false;
        self.error = Some(error.into());
    }

    /// Requests to close the window. Saves unsaved changes first. Returns `true` if closed immediately.
    pub fn request_close(&mut self) -> bool {
        self.error = None;
        if self.dirty {
            self.closing = true;
            false
        } else {
            self.reset();
            true
        }
    }

    /// Returns `true` if the editor window is open.
    pub fn is_open(&self) -> bool {
        self.target.is_some()
    }

    /// Target key layer index, if set.
    pub fn pinned_layer(&self) -> Option<usize> {
        self.target.as_ref().map(|t| t.layer_index)
    }

    /// Returns the currently active layer index (either targeted layer or hardware active layer).
    pub fn active_layer(&self, keyboard: &Keyboard) -> usize {
        self.pinned_layer()
            .unwrap_or_else(|| keyboard.active_layer())
    }

    /// Switches the layer of the targeted key, if an edit target is active.
    pub fn select_layer(&mut self, keyboard: &Keyboard, layer_index: usize) {
        if let Some(target) = self.target {
            if target.layer_index != layer_index {
                self.retarget(keyboard, target.with_layer(layer_index));
            }
        }
    }

    /// Returns `true` if the target key matches the matrix position.
    pub fn is_key_targeted(&self, row: usize, col: usize) -> bool {
        self.target
            .as_ref()
            .is_some_and(|t| t.row == row && t.col == col)
    }

    /// Sets the target key and loads its current binding into the draft.
    pub fn retarget(&mut self, keyboard: &Keyboard, target: EditTarget) {
        if self.closing {
            return;
        }

        let is_already_open = self.is_open();
        self.target = Some(target);
        self.error = None;
        if !is_already_open {
            self.search_query.clear();
        }
        if self.lock_state == EditLockState::Failed {
            self.lock_state = EditLockState::Idle;
        }
        let action = target.action(keyboard);
        if is_already_open {
            self.draft = KeyDraft::for_section(self.draft.section, action.as_ref());
        } else if let Some(action) = action {
            self.draft = KeyDraft::from_spec(&action);
        } else {
            self.draft = Default::default();
        }
    }
}

impl EditorState {
    /// Prepares editor state for rendering: flushes pending saves, polls writes, and ensures lock.
    fn prepare_frame(&mut self, ctx: &egui::Context, keyboard: &Keyboard) {
        if self.closing && self.pending.is_none() && self.dirty {
            self.start_save(keyboard);
        }

        self.poll_pending_write(ctx, keyboard);

        if matches!(keyboard.write_support(), WriteSupport::Staged) && !self.closing {
            self.ensure_lock(keyboard);
        }
    }

    /// Renders the core editor body, optionally displaying the layer switcher header.
    pub fn draw_editor_content(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        style: &crate::key_paint::KeyPaintStyle,
        target: EditTarget,
        show_layer_switcher: bool,
    ) {
        if let Some(error) = &self.error {
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
        }

        let is_enabled = self.overlay().is_none();
        ui.add_enabled_ui(is_enabled, |ui| {
            let target = if show_layer_switcher {
                self.draw_editor_header(ui, keyboard, target, style)
            } else {
                target
            };

            match keyboard.write_support() {
                WriteSupport::None => {
                    ui.add_space(8.0);
                    ui.weak("This key cannot be edited in this version.");
                }
                WriteSupport::Immediate | WriteSupport::Staged => {
                    ui.add_space(8.0);
                    self.draw_editor_body(ui, keyboard, profile, target, style);
                }
            }
        });

        self.draw_editor_overlay(ui);
    }

    /// Draws the edit key window (used on desktop).
    pub fn draw_window(
        &mut self,
        ctx: &egui::Context,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let Some(target) = self.target else {
            return;
        };

        self.prepare_frame(ctx, keyboard);

        let closing = self.closing;
        let mut window = Window::new(self.title())
            .id(egui::Id::new("edit_key_window"))
            .resizable(true)
            .default_size(egui::vec2(440.0, 525.0))
            .min_size(egui::vec2(440.0, 525.0));

        let mut open = true;
        if !closing {
            window = window.open(&mut open);
        }
        window.show(ctx, |ui| {
            self.draw_editor_content(ui, keyboard, profile, style, target, true);
        });

        if !open {
            self.handle_close_request(Some(keyboard));
        }
    }

    /// Draws the edit key sidebar (used on WASM).
    pub fn draw_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let Some(target) = self.target else {
            return;
        };

        self.prepare_frame(ui.ctx(), keyboard);

        egui::Panel::left(egui::Id::new("edit_key_sidebar"))
            .resizable(true)
            .default_size(450.0)
            .min_size(340.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                self.draw_editor_content(ui, keyboard, profile, style, target, false);
            });
    }

    /// Title string reflecting whether there are unsaved pending changes.
    pub fn title(&self) -> &'static str {
        if self.dirty {
            "Edit key (Unsaved changes)"
        } else {
            "Edit key"
        }
    }

    /// Requests to close the editor, releasing edit lock if closed immediately.
    /// Returns `true` if closed immediately.
    pub fn handle_close_request(&mut self, keyboard: Option<&Keyboard>) -> bool {
        if self.request_close() {
            if let Some(kbd) = keyboard {
                kbd.release_edit_lock();
            }
            true
        } else {
            false
        }
    }

    /// Writes an action if it differs from current binding.
    pub(super) fn commit_action(
        &mut self,
        keyboard: &Keyboard,
        target: EditTarget,
        action: KeySpec,
    ) {
        if target.action(keyboard).as_ref() != Some(&action) {
            self.apply_write(keyboard, target, action);
        }
    }

    /// Writes a staged binding if complete and different from current key.
    pub(super) fn commit_staged(
        &mut self,
        keyboard: &Keyboard,
        target: EditTarget,
        staged: Option<KeySpec>,
    ) {
        if let Some(action) = staged {
            self.commit_action(keyboard, target, action);
        }
    }

    /// Draws layer switcher buttons across the top row.
    fn draw_editor_header(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        style: &crate::key_paint::KeyPaintStyle,
    ) -> EditTarget {
        let clicked = ui
            .push_id("layer_switcher", |ui| {
                draw_layer_switcher(ui, keyboard, target.layer_index, None, style)
            })
            .inner;

        if let Some(new_layer) = clicked {
            self.select_layer(keyboard, new_layer);
            return self.target.unwrap_or(target);
        }

        target
    }

    /// Draws a centered spinner and status text during connecting, saving, or failed operations.
    fn draw_editor_overlay(&mut self, ui: &mut egui::Ui) {
        let Some(overlay) = self.overlay() else {
            return;
        };

        let (msg, is_spinner, is_retry) = match overlay {
            EditorOverlay::Saving => ("Saving...", true, false),
            EditorOverlay::AcquiringLock => ("Connecting...", true, false),
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
                        self.lock_state = EditLockState::Idle;
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

    /// Acquires the hardware edit lock when idle.
    fn ensure_lock(&mut self, keyboard: &Keyboard) {
        if self.lock_state != EditLockState::Idle || self.pending.is_some() {
            return;
        }
        self.lock_state = EditLockState::Acquiring;
        self.start_task(PendingKind::AcquireLock, keyboard.acquire_edit_lock());
    }

    /// Sends a write command, or queues it if a task is running.
    pub(super) fn apply_write(&mut self, keyboard: &Keyboard, target: EditTarget, action: KeySpec) {
        if self.pending.is_some() {
            self.queued = Some((target, action));
            return;
        }
        let receiver = target.set_key(keyboard, action);
        self.start_task(PendingKind::Set, receiver);
    }

    /// Polls background tasks and handles queued writes.
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
                    if self.dirty {
                        self.start_save(keyboard);
                    } else if self.pending.is_none() {
                        self.reset();
                        keyboard.release_edit_lock();
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

/// Draws layer switcher buttons in a horizontal row.
///
/// If `button_width` is `None`, buttons expand proportionally to fill available width.
/// Returns `Some(clicked_layer)` if a button was clicked.
pub fn draw_layer_switcher(
    ui: &mut egui::Ui,
    keyboard: &Keyboard,
    active_layer: usize,
    button_width: Option<f32>,
    style: &crate::key_paint::KeyPaintStyle,
) -> Option<usize> {
    let layer_infos = keyboard.layer_infos();
    let mut selected_layer = None;

    let layer_count = layer_infos.len().max(1);
    let item_spacing = ui.spacing().item_spacing.x;
    let total_spacing = (layer_count - 1) as f32 * item_spacing;
    let width = button_width
        .unwrap_or_else(|| ((ui.available_width() - total_spacing) / layer_count as f32).max(24.0));

    ui.horizontal(|ui| {
        for (i, info) in layer_infos.iter().enumerate() {
            let label = info.short_name(i);
            let is_selected = active_layer == i;
            if layer_button(ui, &label, i, is_selected, width, style).clicked() {
                selected_layer = Some(i);
            }
        }
    });

    selected_layer
}

/// Renders a styled layer selector button matching the active theme.
pub fn layer_button(
    ui: &mut egui::Ui,
    label: &str,
    layer_index: usize,
    selected: bool,
    width: f32,
    style: &crate::key_paint::KeyPaintStyle,
) -> egui::Response {
    let colors = style.colors_for(layer_index as u8, KeycodeKind::Modifier, false, selected);
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

#[cfg(test)]
mod tests {
    use super::*;
    use keypeek_core::keymap_editor::draft::EditorSection;
    use keypeek_protocol::test_utils::create_test_keyboard;

    #[test]
    fn retarget_on_closed_editor_selects_keycode_section() {
        let keyboard = create_test_keyboard();
        let mut editor = EditorState::new();
        assert!(!editor.is_open());

        // Target key is 'KC_Q' (KeyPress) -> Keyboard section
        let key_press_target = EditTarget {
            layer_index: 0,
            row: 0,
            col: 1,
        };
        editor.retarget(&keyboard, key_press_target);
        assert!(editor.is_open());
        assert_eq!(editor.draft.section, EditorSection::Keyboard);

        // Close editor
        editor.request_close();
        assert!(!editor.is_open());

        // Target key is 'MO(1)' (Layer activation) -> Layers section
        let layer_target = EditTarget {
            layer_index: 0,
            row: 3,
            col: 3,
        };
        editor.retarget(&keyboard, layer_target);
        assert!(editor.is_open());
        assert_eq!(editor.draft.section, EditorSection::Layers);
    }

    #[test]
    fn retarget_on_open_editor_preserves_active_section_and_search_query() {
        let keyboard = create_test_keyboard();
        let mut editor = EditorState::new();

        // 1. Initially closed: open on keypress key (selects Keyboard section)
        let key_press_target = EditTarget {
            layer_index: 0,
            row: 0,
            col: 1,
        };
        editor.retarget(&keyboard, key_press_target);
        assert_eq!(editor.draft.section, EditorSection::Keyboard);

        // Set a search query
        editor.search_query = "play".to_string();

        // 2. Editor is open: retarget to a layer key 'MO(1)'
        let layer_target = EditTarget {
            layer_index: 0,
            row: 3,
            col: 3,
        };
        editor.retarget(&keyboard, layer_target);
        // Active section should remain Keyboard, NOT switch to Layers
        assert_eq!(editor.draft.section, EditorSection::Keyboard);
        // Search query should be preserved
        assert_eq!(editor.search_query, "play");

        // 3. User manually navigates to Audio section
        editor.draft.section = EditorSection::Audio;
        editor.search_query = "mute".to_string();

        // Retarget back to a keypress key
        editor.retarget(&keyboard, key_press_target);
        // Active section should remain Audio
        assert_eq!(editor.draft.section, EditorSection::Audio);
        assert_eq!(editor.search_query, "mute");

        // 4. Closing the editor and reopening on layer_target selects Layers
        assert!(editor.request_close());
        assert!(!editor.is_open());
        editor.retarget(&keyboard, layer_target);
        assert_eq!(editor.draft.section, EditorSection::Layers);
        assert!(editor.search_query.is_empty());
    }

    #[test]
    fn retarget_while_closing_is_ignored() {
        let keyboard = create_test_keyboard();
        let mut editor = EditorState::new();
        let target1 = EditTarget::new(0, 0, 1);
        editor.retarget(&keyboard, target1);
        assert_eq!(editor.target, Some(target1));

        editor.closing = true;
        let target2 = EditTarget::new(0, 3, 3);
        editor.retarget(&keyboard, target2);
        assert_eq!(editor.target, Some(target1));
    }

    #[test]
    fn test_pinned_layer() {
        let mut editor = EditorState::new();
        assert_eq!(editor.pinned_layer(), None);

        editor.target = Some(EditTarget::new(2, 1, 3));
        assert_eq!(editor.pinned_layer(), Some(2));
    }

    #[test]
    fn test_is_key_targeted() {
        let mut editor = EditorState::new();
        assert!(!editor.is_key_targeted(1, 3));

        editor.target = Some(EditTarget::new(2, 1, 3));
        assert!(editor.is_key_targeted(1, 3));
        assert!(!editor.is_key_targeted(1, 4));
        assert!(!editor.is_key_targeted(2, 3));
    }

    #[test]
    fn test_edit_target_helpers() {
        let target = EditTarget::new(0, 1, 2);
        assert_eq!(target.layer_index, 0);
        assert_eq!(target.row, 1);
        assert_eq!(target.col, 2);

        let new_target = target.with_layer(3);
        assert_eq!(new_target.layer_index, 3);
        assert_eq!(new_target.row, 1);
        assert_eq!(new_target.col, 2);
    }

    #[test]
    fn test_select_layer() {
        let keyboard = create_test_keyboard();
        let mut editor = EditorState::new();
        let target = EditTarget::new(0, 0, 1);
        editor.retarget(&keyboard, target);
        assert_eq!(editor.active_layer(&keyboard), 0);

        editor.select_layer(&keyboard, 2);
        assert_eq!(editor.target.unwrap().layer_index, 2);
        assert_eq!(editor.active_layer(&keyboard), 2);
    }
}
