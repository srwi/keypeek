//! Persistent "Edit key" window: its content follows the most recently clicked
//! key on a pinned layer. The per-firmware editing UI lands in later stages;
//! this module currently shows the target and its current binding.

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use egui::Window;
use std::sync::mpsc;

/// Which key the editor window is currently targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditTarget {
    pub layer_index: usize,
    pub row: usize,
    pub col: usize,
}

/// Editor window state, owned by `OverlayApp`.
pub struct EditorState {
    /// `None` = window closed.
    pub target: Option<EditTarget>,
    /// An in-flight write; polled each frame (added in the write stages).
    pub pending: Option<mpsc::Receiver<Result<(), String>>>,
    /// Last write/read error shown in the window.
    pub error: Option<String>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            target: None,
            pending: None,
            error: None,
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
    /// Draws the persistent "Edit key" window for the current target. The body
    /// is just the header and an error slot until the write stages arrive.
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

        let mut open = true;
        Window::new("Edit key")
            .open(&mut open)
            .resizable(false)
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

                if let Some(error) = &self.editor.error {
                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
                }
            });

        if !open {
            self.editor.target = None;
            self.editor.pending = None;
            self.editor.error = None;
        }
    }
}
