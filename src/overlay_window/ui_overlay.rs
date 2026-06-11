use super::state::LabelGalleys;
use super::OverlayApp;
use crate::keyboard::Keyboard;
use crate::layout_key::{KeycodeKind, LayoutKey};
use crate::settings::ThemeColor;
use eframe::egui::{self, Window};

/// Rotate `point` clockwise around `origin` by `angle_rad` (screen space, y-down).
fn rotate_point(point: egui::Pos2, origin: egui::Pos2, angle_rad: f32) -> egui::Pos2 {
    if angle_rad == 0.0 {
        return point;
    }
    let (sin_a, cos_a) = angle_rad.sin_cos();
    let dx = point.x - origin.x;
    let dy = point.y - origin.y;
    egui::pos2(
        origin.x + dx * cos_a - dy * sin_a,
        origin.y + dx * sin_a + dy * cos_a,
    )
}

/// Build a text shape tilted as if the key were rotated by `angle` around `center`:
/// the anchor moves along the arc and the glyphs tilt by the same angle.
fn rotated_text_shape(
    pos: egui::Pos2,
    galley: std::sync::Arc<egui::Galley>,
    color: egui::Color32,
    center: egui::Pos2,
    angle: f32,
) -> egui::Shape {
    egui::Shape::Text(
        egui::epaint::TextShape::new(rotate_point(pos, center, angle), galley, color)
            .with_angle(angle),
    )
}

impl OverlayApp {
    pub(super) fn generate_key_label_galleys(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        font: egui::FontId,
        color: egui::Color32,
    ) -> LabelGalleys {
        let (symbol, text) = self.generate_tap_galleys(ui, key, rect, font, color);
        let hold = self.generate_hold_galley(ui, key, rect, color);
        LabelGalleys { symbol, text, hold }
    }

    fn generate_tap_galleys(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        font: egui::FontId,
        color: egui::Color32,
    ) -> (
        Option<std::sync::Arc<egui::Galley>>,
        Option<std::sync::Arc<egui::Galley>>,
    ) {
        let size = self.settings.active.size as f32;
        let font_scale = self.settings.active.font_size_multiplier;
        let create_galley =
            |text: String, fid: egui::FontId| ui.painter().layout_no_wrap(text, fid, color);
        let fits_width =
            |galley: &std::sync::Arc<egui::Galley>, max: f32| galley.rect.width() <= max;
        let max_width = rect.width() * 0.85;

        // Stack the shifted character above the base character.
        if let Some(shifted) = &key.shifted {
            let text = if key.tap.is_empty() {
                shifted.clone()
            } else {
                format!("{}\n{}", shifted, key.tap.full)
            };
            return (None, Some(create_galley(text, font)));
        }

        if let Some(symbol) = &key.symbol {
            let symbol_font = egui::FontId::proportional(0.33 * size * font_scale);
            let symbol_galley = create_galley(symbol.clone(), symbol_font);

            if !key.tap.is_empty() {
                let text_galley = create_galley(key.tap.full.clone(), font.clone());
                let gap = 0.06 * size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return (Some(symbol_galley), Some(text_galley));
                }
            }

            if let Some(short) = &key.tap.short {
                let text_galley = create_galley(short.clone(), font.clone());
                let gap = 0.06 * size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return (Some(symbol_galley), Some(text_galley));
                }
            }

            return (Some(symbol_galley), None);
        }

        let full_galley = create_galley(key.tap.full.clone(), font.clone());
        if fits_width(&full_galley, max_width) {
            return (None, Some(full_galley));
        }

        let mut truncated = if let Some(short) = &key.tap.short {
            let short_galley = create_galley(short.clone(), font.clone());
            if fits_width(&short_galley, max_width) {
                return (None, Some(short_galley));
            }
            short.clone()
        } else {
            key.tap.full.clone()
        };

        if self.settings.active.auto_fit_before_ellipsis {
            let max_height = rect.height() * 0.85;
            let fit_text = key.tap.short.as_ref().unwrap_or(&key.tap.full).clone();
            let fit_galley = create_galley(fit_text.clone(), font.clone());
            let width_scale = if fit_galley.rect.width() > 0.0 {
                max_width / fit_galley.rect.width()
            } else {
                1.0
            };
            let height_scale = if fit_galley.rect.height() > 0.0 {
                max_height / fit_galley.rect.height()
            } else {
                1.0
            };
            let scale = width_scale.min(height_scale).min(1.0);
            let fitted_text =
                create_galley(fit_text, egui::FontId::proportional(font.size * scale));
            return (None, Some(fitted_text));
        }

        while truncated.len() > 1 {
            truncated.pop();
            let truncated_with_ellipsis = format!("{}...", truncated);
            let truncated_galley = create_galley(truncated_with_ellipsis, font.clone());
            if fits_width(&truncated_galley, max_width) {
                return (None, Some(truncated_galley));
            }
        }

        (None, None)
    }

    fn generate_hold_galley(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        color: egui::Color32,
    ) -> Option<std::sync::Arc<egui::Galley>> {
        let hold = key.hold.as_ref()?;
        let size = self.settings.active.size as f32;
        let font_scale = self.settings.active.font_size_multiplier;
        let max_width = rect.width() * 0.85;
        let hold_font = egui::FontId::proportional(0.20 * size * font_scale);
        let create_galley =
            |text: String, fid: egui::FontId| ui.painter().layout_no_wrap(text, fid, color);

        let galley = create_galley(hold.full.clone(), hold_font.clone());
        if galley.rect.width() <= max_width {
            return Some(galley);
        }
        if let Some(short) = &hold.short {
            let short_galley = create_galley(short.clone(), hold_font);
            if short_galley.rect.width() <= max_width {
                return Some(short_galley);
            }
        }
        None
    }

    pub(super) fn get_keycode_color(
        &self,
        layer: u8,
        kind: KeycodeKind,
        desaturate: bool,
        pressed: bool,
    ) -> (egui::Color32, egui::Color32, f32, egui::Color32) {
        const DESATURATE_FACTOR: f32 = 0.7;

        const BLACK: egui::Color32 = egui::Color32::BLACK;

        let size = self.settings.active.size as f32;
        let layer_theme_color = self.settings.active.theme.layer_color(layer);
        let mut background_color = Self::to_egui_color(layer_theme_color);
        let mut font_color = Self::to_egui_color(self.settings.active.theme.font_color);

        if pressed {
            return (
                background_color.lerp_to_gamma(egui::Color32::WHITE, 0.2),
                background_color.lerp_to_gamma(egui::Color32::WHITE, 0.7),
                0.03 * size,
                font_color.lerp_to_gamma(egui::Color32::WHITE, 0.4),
            );
        }

        if kind == KeycodeKind::Special {
            background_color = background_color.lerp_to_gamma(BLACK, 0.6);
        } else if kind == KeycodeKind::Modifier {
            background_color = background_color.lerp_to_gamma(BLACK, 0.3);
        }

        let mut border_color = background_color.lerp_to_gamma(BLACK, 0.2);
        if desaturate && layer != 0 {
            let layer0_color = Self::to_egui_color(self.settings.active.theme.layer_colors[0]);
            background_color = background_color.lerp_to_gamma(layer0_color, DESATURATE_FACTOR);
            border_color = border_color.lerp_to_gamma(layer0_color, DESATURATE_FACTOR);
            font_color = font_color.gamma_multiply(1.0 - DESATURATE_FACTOR);
        }

        (background_color, border_color, 1.0, font_color)
    }

    pub(super) fn to_egui_color(color: ThemeColor) -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(color.r, color.g, color.b, color.a)
    }

    pub(super) fn from_egui_color(color: egui::Color32) -> ThemeColor {
        ThemeColor::new(color.r(), color.g(), color.b(), color.a())
    }

    pub(super) fn theme_color_entry(ui: &mut egui::Ui, label: &str, color: &mut ThemeColor) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut display_color = Self::to_egui_color(*color);
                if ui.color_edit_button_srgba(&mut display_color).changed() {
                    *color = Self::from_egui_color(display_color);
                }
            });
        });
        ui.add_space(4.0);
    }

    pub(super) fn draw_overlay_window(&self, ctx: &egui::Context, keyboard: &Keyboard) {
        let anchor_params = self.get_anchor_params();
        let mut window_open = true;
        let size = self.settings.active.size as f32;
        let font_scale = self.settings.active.font_size_multiplier;

        Window::new("KeyPeek")
            .open(&mut window_open)
            .auto_sized()
            .interactable(false)
            .anchor(anchor_params.0, anchor_params.1)
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .fade_out(true)
            .title_bar(false)
            .show(ctx, |ui| {
                let layout_size = keyboard.layout.get_dimensions();
                ui.allocate_space(egui::vec2(layout_size.0 * size, layout_size.1 * size));
                let window_pos = ui.min_rect().min;

                for key in &keyboard.layout.keys {
                    let (effective_layer, is_background_key) =
                        keyboard.get_effective_key_layer(key.row, key.col);

                    let layout_key = keyboard
                        .get_key(effective_layer as usize, key.row, key.col)
                        .unwrap_or_default();

                    let first_layer_key_kind = keyboard
                        .get_key(0, key.row, key.col)
                        .map(|k| k.kind)
                        .unwrap_or(KeycodeKind::Basic);

                    let (fill_color, stroke_color, border_thickness, font_color) = self
                        .get_keycode_color(
                            layout_key.layer_ref.unwrap_or(effective_layer),
                            first_layer_key_kind,
                            is_background_key,
                            keyboard.is_key_pressed(key.row, key.col),
                        );

                    let rect = egui::Rect::from_min_size(
                        egui::pos2(key.x * size, key.y * size) + window_pos.to_vec2(),
                        egui::vec2(key.w * size, key.h * size),
                    )
                    .shrink(0.06 * size);

                    let angle = key.r.to_radians();
                    let center = rect.center();

                    ui.painter().add(
                        egui::epaint::RectShape::new(
                            rect,
                            0.1 * size,
                            fill_color,
                            egui::Stroke::new(border_thickness, stroke_color),
                            egui::StrokeKind::Outside,
                        )
                        .with_angle(angle),
                    );

                    let font = egui::FontId::proportional(0.25 * size * font_scale);
                    let galleys =
                        self.generate_key_label_galleys(ui, &layout_key, rect, font, font_color);

                    // When a hold label is present, reserve a strip along the bottom edge for it
                    // and center the primary label in the remaining area above.
                    let hold_height = rect.height() * 0.22;
                    let main_label_rect = if galleys.hold.is_some() {
                        egui::Rect::from_min_max(
                            rect.left_top(),
                            egui::pos2(rect.right(), rect.bottom() - hold_height),
                        )
                    } else {
                        rect
                    };

                    if let Some(hold_galley) = galleys.hold {
                        let strip = egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.bottom() - hold_height),
                            rect.max,
                        );
                        // RectShape rotates around its own center, so orbit the strip's center
                        // around the key center first, then tilt it in place.
                        let strip_rect = egui::Rect::from_center_size(
                            rotate_point(strip.center(), center, angle),
                            strip.size(),
                        );
                        let radius = (0.08 * size) as u8;
                        ui.painter().add(
                            egui::epaint::RectShape::new(
                                strip_rect,
                                egui::CornerRadius {
                                    nw: 0,
                                    ne: 0,
                                    sw: radius,
                                    se: radius,
                                },
                                fill_color.lerp_to_gamma(egui::Color32::BLACK, 0.15),
                                egui::Stroke::NONE,
                                egui::StrokeKind::Outside,
                            )
                            .with_angle(angle),
                        );
                        let hold_pos = strip.center() - hold_galley.rect.center().to_vec2();
                        ui.painter().add(rotated_text_shape(
                            hold_pos,
                            hold_galley,
                            font_color.gamma_multiply(0.7),
                            center,
                            angle,
                        ));
                    }

                    match (galleys.symbol, galleys.text) {
                        (Some(symbol_galley), Some(text_galley)) => {
                            let gap = 0.06 * size;
                            let total_width =
                                symbol_galley.rect.width() + gap + text_galley.rect.width();
                            let start_x = main_label_rect.center().x - total_width * 0.5;

                            let text_pos_x = start_x + gap + symbol_galley.rect.width();
                            let text_pos = egui::pos2(
                                text_pos_x,
                                main_label_rect.center().y - text_galley.rect.center().y,
                            );
                            let sym_pos = egui::pos2(
                                start_x,
                                main_label_rect.center().y - symbol_galley.rect.center().y,
                            );
                            ui.painter().add(rotated_text_shape(
                                sym_pos,
                                symbol_galley,
                                font_color,
                                center,
                                angle,
                            ));
                            ui.painter().add(rotated_text_shape(
                                text_pos,
                                text_galley,
                                font_color,
                                center,
                                angle,
                            ));
                        }
                        (Some(symbol_galley), None) => {
                            let sym_pos =
                                main_label_rect.center() - symbol_galley.rect.center().to_vec2();
                            ui.painter().add(rotated_text_shape(
                                sym_pos,
                                symbol_galley,
                                font_color,
                                center,
                                angle,
                            ));
                        }
                        (None, Some(text_galley)) => {
                            let label_pos =
                                main_label_rect.center() - text_galley.rect.center().to_vec2();
                            ui.painter().add(rotated_text_shape(
                                label_pos,
                                text_galley,
                                font_color,
                                center,
                                angle,
                            ));
                        }
                        _ => {}
                    }
                }
            });
    }
}
