//! Platform-agnostic keyboard overlay rendering component.

use crate::key_paint::{self, KeyDisplay, KeyPaintStyle};
use crate::keymap_editor::{EditTarget, EditorState};
use crate::settings::{LegendMode, Settings};
use keypeek_core::{KeycodeKind, LayoutKey};
use keypeek_protocol::Keyboard;

/// Reusable view component that renders keyboard layer geometry and keys.
pub struct OverlayView<'a> {
    pub keyboard: &'a Keyboard,
    pub editor: &'a mut EditorState,
    pub settings: &'a Settings,
    pub size: f32,
    pub hit_test_enabled: bool,
}

impl<'a> OverlayView<'a> {
    pub fn new(
        keyboard: &'a Keyboard,
        editor: &'a mut EditorState,
        settings: &'a Settings,
        size: f32,
        hit_test_enabled: bool,
    ) -> Self {
        Self {
            keyboard,
            editor,
            settings,
            size,
            hit_test_enabled,
        }
    }

    /// Renders the overlay keys into the provided UI.
    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        draw_overlay_keys(
            ui,
            self.keyboard,
            self.editor,
            self.settings,
            self.size,
            self.hit_test_enabled,
        )
    }
}

/// Draws keyboard keys with active styles, layer highlighting, and hit-testing.
pub fn draw_overlay_keys(
    ui: &mut egui::Ui,
    keyboard: &Keyboard,
    editor: &mut EditorState,
    settings: &Settings,
    size: f32,
    hit_test_enabled: bool,
) -> egui::Response {
    // Pinned while editor targets a specific layer; otherwise follows active layer.
    let pinned = editor.pinned_layer();
    let style = KeyPaintStyle::from_settings(settings).with_unit(size);

    let layout = keyboard.layout();
    let layout_size = layout.get_dimensions();
    let overlay_space = ui.allocate_space(egui::vec2(layout_size.0 * size, layout_size.1 * size));
    let overlay_rect = overlay_space.1;
    let window_pos = overlay_rect.min;

    // Route pointer input through egui to prevent misreading clicks under settings.
    let overlay_response = ui.interact(
        overlay_rect,
        ui.id().with("overlay_keys"),
        egui::Sense::click(),
    );

    let mut hovered_key: Option<(usize, usize, usize)> = None;

    // Modifier state only applies to live preview on non-pinned layers.
    let live_preview_active = pinned.is_none() && settings.legend_mode == LegendMode::SingleLive;
    let shift_held = live_preview_active && keyboard.is_shift_held();
    let ralt_held = live_preview_active && keyboard.is_ralt_held();

    for key in &layout.keys {
        let (effective_layer, is_background_key) = match pinned {
            Some(layer) => (layer as u8, false),
            None => keyboard.get_effective_key_layer(key.row, key.col),
        };

        // Pinned transparent slot renders dimmed empty; absent slot is plain empty.
        let transparent = pinned.is_some()
            && keyboard
                .get_action(effective_layer as usize, key.row, key.col)
                .is_some()
            && keyboard
                .get_key(effective_layer as usize, key.row, key.col)
                .is_none();

        let layout_key = keyboard
            .get_key(effective_layer as usize, key.row, key.col)
            .unwrap_or_default();

        let first_layer_key_kind = keyboard
            .get_key(0, key.row, key.col)
            .map(|k| k.kind)
            .unwrap_or(KeycodeKind::Basic);

        let is_selected_for_edit = editor.is_key_targeted(key.row, key.col);
        let pressed = keyboard.is_key_pressed(key.row, key.col) || is_selected_for_edit;
        let mut colors = style.colors_for(
            layout_key.layer_ref.unwrap_or(effective_layer),
            first_layer_key_kind,
            is_background_key,
            pressed,
        );

        if transparent {
            colors = colors.ghosted();
        }

        let rect = egui::Rect::from_min_size(
            egui::pos2(key.x * size, key.y * size) + window_pos.to_vec2(),
            egui::vec2(key.w * size, key.h * size),
        );

        let angle = key.r.to_radians();
        let center = rect.center();

        // Only keys with a binding slot can be clicked.
        let clickable = hit_test_enabled
            && keyboard
                .get_action(effective_layer as usize, key.row, key.col)
                .is_some();
        // Test visible face (shrunk by 0.06 unit margin).
        let face = rect.shrink(0.06 * size);
        let hovered = clickable
            && overlay_response
                .hover_pos()
                .is_some_and(|p| face.contains(key_paint::rotate_point(p, center, -angle)));
        if hovered {
            hovered_key = Some((key.row, key.col, effective_layer as usize));
            if let Some(tooltip) = key_tooltip(&layout_key, transparent) {
                show_pointer_tooltip(
                    ui,
                    ui.id().with(("overlay_key_tooltip", key.row, key.col)),
                    &tooltip,
                );
            }
        }

        key_paint::paint(
            ui,
            rect,
            angle,
            &KeyDisplay {
                key: &layout_key,
                colors,
                hovered,
                pressed,
                shift_held,
                ralt_held,
            },
            &style,
        );
    }

    if hit_test_enabled && overlay_response.clicked() {
        if let Some((row, col, target_layer)) = hovered_key {
            editor.retarget(keyboard, EditTarget::new(target_layer, row, col));
        }
    }

    overlay_response
}

fn key_tooltip(key: &LayoutKey, transparent: bool) -> Option<String> {
    if transparent {
        Some("Transparent".to_string())
    } else {
        key.tooltip_text()
    }
}

fn show_pointer_tooltip(ui: &egui::Ui, id: egui::Id, text: &str) {
    egui::Tooltip::always_open(
        ui.ctx().clone(),
        ui.layer_id(),
        id,
        egui::PopupAnchor::Pointer,
    )
    .gap(12.0)
    .show(|ui| {
        ui.add(egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend));
    });
}
