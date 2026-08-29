use super::OverlayApp;
use crate::key_paint::{self, KeyDisplay};
use crate::keyboard::Keyboard;
use crate::layout_key::KeycodeKind;
use crate::settings::{LegendMode, WindowPosition};
use egui::Window;

impl OverlayApp {
    /// Returns the overlay window's rect when it was shown, so the layer picker
    /// can sit adjacent to it.
    pub(super) fn draw_overlay_window(
        &mut self,
        ctx: &egui::Context,
        keyboard: &Keyboard,
        visible: bool,
    ) -> Option<egui::Rect> {
        let anchor_params = self.get_anchor_params();
        let mut window_open = visible;
        let size = self.settings.active.size as f32;
        let pinned = self.ui.pinned_layer;
        // Keys can only be clicked on a pinned layer while settings are open.
        let hit_test_enabled = self.ui.settings_visible && pinned.is_some();

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
                let layout_size = keyboard.layout.get_dimensions();
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

                let mut hovered_key: Option<(usize, usize)> = None;

                // Only walk the matrix for live modifier state when the preview can
                // actually use it; same reasoning as `is_key_pressed` elsewhere.
                // A pinned layer renders flat, so the live preview does not apply.
                let live_preview_active =
                    pinned.is_none() && self.settings.active.legend_mode == LegendMode::SingleLive;
                let shift_held = live_preview_active && keyboard.is_shift_held();
                let ralt_held = live_preview_active && keyboard.is_ralt_held();

                for key in &keyboard.layout.keys {
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

                    let pressed = keyboard.is_key_pressed(key.row, key.col);
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
                        hovered_key = Some((key.row, key.col));
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
                        if let Some((row, col)) = hovered {
                            self.editor.target = Some(crate::keymap_editor::EditTarget {
                                layer_index: pinned.unwrap(),
                                row,
                                col,
                            });
                            self.editor.error = None;
                            // Rebuild the draft from the newly targeted binding.
                            match keyboard.get_action(pinned.unwrap(), row, col) {
                                Some(crate::key_action::KeyAction::Qmk(code)) => {
                                    self.editor.qmk_draft =
                                        crate::keymap_editor::QmkDraft::from_keycode(code);
                                }
                                Some(crate::key_action::KeyAction::Zmk(behavior)) => {
                                    self.editor.zmk_draft =
                                        crate::keymap_editor::ZmkDraft::from_behavior(&behavior);
                                }
                                _ => {
                                    self.editor.qmk_draft = Default::default();
                                    self.editor.zmk_draft = Default::default();
                                }
                            }
                        }
                    }
                }
            }
        }

        overlay_response.map(|response| response.response.rect)
    }

    /// The dropdown next to the overlay that picks which layer it shows. Only
    /// present in settings mode; "Active" restores the live view.
    pub(super) fn draw_layer_picker(
        &mut self,
        ctx: &egui::Context,
        keyboard: &Keyboard,
        overlay_rect: egui::Rect,
    ) {
        let layer_infos = keyboard.layer_infos();

        // The layer count can change across a reconnect; drop a stale pin.
        if let Some(layer) = self.ui.pinned_layer {
            if layer >= layer_infos.len() {
                self.ui.pinned_layer = None;
            }
        }

        // Sit the picker above a bottom-anchored overlay, below it otherwise.
        let bottom_anchored = matches!(
            self.settings.active.position,
            WindowPosition::BottomLeft | WindowPosition::BottomRight | WindowPosition::Bottom
        );
        let gap = 8.0;
        let (pivot, pos) = if bottom_anchored {
            (
                egui::Align2::CENTER_BOTTOM,
                egui::pos2(overlay_rect.center().x, overlay_rect.top() - gap),
            )
        } else {
            (
                egui::Align2::CENTER_TOP,
                egui::pos2(overlay_rect.center().x, overlay_rect.bottom() + gap),
            )
        };

        let mut selected = self.ui.pinned_layer;
        Window::new("layer_picker")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .pivot(pivot)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                let selected_text = selected
                    .map(|i| format!("{i}: {}", layer_display_name(&layer_infos, i)))
                    .unwrap_or_else(|| "Active".to_string());
                egui::ComboBox::from_id_salt("layer_picker_combo")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected, None, "Active");
                        for (i, _) in layer_infos.iter().enumerate() {
                            let label = format!("{i}: {}", layer_display_name(&layer_infos, i));
                            ui.selectable_value(&mut selected, Some(i), label);
                        }
                    });
            });
        self.ui.pinned_layer = selected;
    }
}

fn layer_display_name(layer_infos: &[crate::key_action::LayerInfo], index: usize) -> String {
    layer_infos
        .get(index)
        .and_then(|info| info.name.clone())
        .unwrap_or_else(|| format!("Layer {index}"))
}
