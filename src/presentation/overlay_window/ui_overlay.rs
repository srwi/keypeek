use super::OverlayApp;
use crate::application::Keyboard;
use crate::key_paint::{self, KeyDisplay};
use crate::layout_key::KeycodeKind;
use crate::settings::LegendMode;
#[cfg(not(target_arch = "wasm32"))]
use egui::Window;

impl OverlayApp {
    pub(super) fn draw_overlay_keys(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        size: f32,
        hit_test_enabled: bool,
    ) {
        // Pinned while the editor is targeting a specific layer; otherwise automatic (active).
        let pinned = self.editor.pinned_layer();
        let style = self.paint_style(size);

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

            let is_selected_for_edit = self.editor.is_key_targeted(key.row, key.col);
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
                self.editor.retarget(
                    keyboard,
                    crate::keymap_editor::EditTarget::new(target_layer, row, col),
                );
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn draw_overlay_window(
        &mut self,
        ctx: &egui::Context,
        keyboard: &Keyboard,
        visible: bool,
    ) {
        let anchor_params = self.get_anchor_params();
        let mut window_open = visible;
        let hit_test_enabled = self.is_any_window_open();
        let size = self.settings.active.size as f32;

        Window::new("KeyPeek")
            .open(&mut window_open)
            .auto_sized()
            .interactable(hit_test_enabled)
            .anchor(anchor_params.0, anchor_params.1)
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .fade_out(true)
            .title_bar(false)
            .show(ctx, |ui| {
                self.draw_overlay_keys(ui, keyboard, size, hit_test_enabled);
            });
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn draw_overlay_canvas(&mut self, ui: &mut egui::Ui, keyboard: &Keyboard) {
        let (raw_w, raw_h) = keyboard.layout().get_dimensions();
        let layout_w = raw_w.max(1.0);
        let layout_h = raw_h.max(1.0);
        const PADDING: f32 = 32.0;
        const MAX_KEY_SIZE: f32 = 75.0;
        const MIN_KEY_SIZE: f32 = 18.0;

        let avail_size = ui.available_size();
        let avail_w = (avail_size.x - PADDING).max(MIN_KEY_SIZE);
        let avail_h = (avail_size.y - PADDING).max(MIN_KEY_SIZE);

        let key_size = (avail_w / layout_w)
            .min(avail_h / layout_h)
            .clamp(MIN_KEY_SIZE, MAX_KEY_SIZE);

        let overlay_h = layout_h * key_size;
        let v_space = ((avail_size.y - overlay_h) * 0.5).max(0.0);

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if v_space > 0.0 {
                    ui.add_space(v_space);
                }

                ui.vertical_centered(|ui| {
                    self.draw_overlay_keys(ui, keyboard, key_size, true);
                });
            });
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
