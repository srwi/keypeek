use super::OverlayApp;
use crate::key_paint::{self, KeyDisplay};
use crate::keyboard::Keyboard;
use crate::layout_key::KeycodeKind;
use crate::settings::LegendMode;
use egui::Window;

impl OverlayApp {
    pub(super) fn draw_overlay_window(
        &mut self,
        ctx: &egui::Context,
        keyboard: &Keyboard,
        visible: bool,
    ) {
        let anchor_params = self.get_anchor_params();
        let mut window_open = visible;
        let size = self.settings.active.size as f32;
        // Pinned while the editor is targeting a specific layer; otherwise automatic (active).
        let pinned = self.editor.target.as_ref().map(|t| t.layer_index);
        // Keys can be clicked whenever either window is open (window is not clickthrough).
        let hit_test_enabled = self.is_any_window_open();

        // One shared painter for every key this frame; painting itself lives
        // in `key_paint` so pickers render identically.
        let style = self.paint_style(size);

        let overlay_response = Window::new("KeyPeek")
            .open(&mut window_open)
            .auto_sized()
            .interactable(hit_test_enabled)
            .anchor(anchor_params.0, anchor_params.1)
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .fade_out(true)
            .title_bar(false)
            .show(ctx, |ui| {
                let layout = keyboard.layout();
                let layout_size = layout.get_dimensions();
                let overlay_space =
                    ui.allocate_space(egui::vec2(layout_size.0 * size, layout_size.1 * size));
                let overlay_rect = overlay_space.1;
                let window_pos = overlay_rect.min;

                // Route pointer input through egui so a click on a key under an
                // overlapping settings window is not misread.
                let overlay_response = ui.interact(
                    overlay_rect,
                    ui.id().with("overlay_keys"),
                    egui::Sense::click(),
                );

                let mut hovered_key: Option<(usize, usize, usize)> = None;

                // Only walk the matrix for live modifier state when the preview can
                // actually use it; same reasoning as `is_key_pressed` elsewhere.
                // A pinned layer renders flat, so the live preview does not apply.
                let live_preview_active =
                    pinned.is_none() && self.settings.active.legend_mode == LegendMode::SingleLive;
                let shift_held = live_preview_active && keyboard.is_shift_held();
                let ralt_held = live_preview_active && keyboard.is_ralt_held();

                for key in &layout.keys {
                    let (effective_layer, is_background_key) = match pinned {
                        Some(layer) => (layer as u8, false),
                        None => keyboard.get_effective_key_layer(key.row, key.col),
                    };

                    // A pinned transparent binding (a slot with no label) renders as a
                    // dimmed empty key; an absent slot is a plain empty key.
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

                    let is_selected_for_edit = self
                        .editor
                        .target
                        .as_ref()
                        .is_some_and(|t| t.row == key.row && t.col == key.col);
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

                    // Only keys with an existing binding slot are clickable; a
                    // transparent slot counts, an absent one does not.
                    let clickable = hit_test_enabled
                        && keyboard
                            .get_action(effective_layer as usize, key.row, key.col)
                            .is_some();
                    // Hover tests the visible key face: `paint` shrinks the raw
                    // cell by its 0.06*unit margin before drawing.
                    let face = rect.shrink(0.06 * size);
                    let hovered = clickable
                        && overlay_response.hover_pos().is_some_and(|p| {
                            face.contains(key_paint::rotate_point(p, center, -angle))
                        });
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

                (hovered_key, overlay_response.clicked())
            });

        // Handle a click after the closure so editor state can be mutated.
        if hit_test_enabled {
            if let Some(response) = overlay_response.as_ref() {
                if let Some((hovered, clicked)) = response.inner {
                    // A closing editor is saving; it must not be retargeted.
                    if clicked && !self.editor.closing {
                        if let Some((row, col, target_layer)) = hovered {
                            self.editor.retarget(
                                keyboard,
                                crate::keymap_editor::EditTarget {
                                    layer_index: target_layer,
                                    row,
                                    col,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}

fn key_tooltip(key: &crate::layout_key::LayoutKey, transparent: bool) -> Option<String> {
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
