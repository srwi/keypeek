use super::state::LabelGalleys;
use super::OverlayApp;
use crate::keyboard::Keyboard;
use crate::layout_key::{KeycodeKind, LayoutKey};
use crate::settings::ThemeColor;
use eframe::egui::{self, Window};

fn rotate_point(point: egui::Pos2, origin: egui::Pos2, angle_rad: f32) -> egui::Pos2 {
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let dx = point.x - origin.x;
    let dy = point.y - origin.y;
    egui::pos2(
        origin.x + dx * cos_a - dy * sin_a,
        origin.y + dx * sin_a + dy * cos_a,
    )
}

impl OverlayApp {
    fn add_rotated_shape(
        painter: &egui::Painter,
        mut shape: egui::Shape,
        angle: f32,
        origin: egui::Pos2,
    ) {
        if angle != 0.0 {
            match &mut shape {
                egui::Shape::Rect(rect_shape) => {
                    let new_center = rotate_point(rect_shape.rect.center(), origin, angle);
                    rect_shape.rect =
                        egui::Rect::from_center_size(new_center, rect_shape.rect.size());
                    rect_shape.angle += angle;
                }
                egui::Shape::Text(text_shape) => {
                    text_shape.pos = rotate_point(text_shape.pos, origin, angle);
                    text_shape.angle += angle;
                }
                _ => {
                    eprintln!(
                        "Warning: Unhandled shape type in add_rotated_shape: {:?}",
                        shape
                    );
                }
            }
        }
        painter.add(shape);
    }
    pub(super) fn generate_key_label_galleys(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        font: egui::FontId,
        color: egui::Color32,
    ) -> LabelGalleys {
        let size = self.settings.active.size as f32;
        let font_scale = self.settings.active.font_size_multiplier;
        let create_galley = |text: String, fid: egui::FontId| {
            let mut job = egui::text::LayoutJob::default();
            job.append(
                &text,
                0.0,
                egui::text::TextFormat {
                    font_id: fid,
                    color,
                    ..Default::default()
                },
            );
            job.halign = egui::Align::Center;
            ui.painter().layout_job(job)
        };
        let fits_width =
            |galley: &std::sync::Arc<egui::Galley>, max: f32| galley.rect.width() <= max;
        let max_width = rect.width() * 0.85;

        let mut galleys = if let Some(symbol) = &key.symbol {
            let symbol_font = egui::FontId::proportional(0.33 * size * font_scale);
            let symbol_galley = create_galley(symbol.clone(), symbol_font);

            if !key.tap.is_empty() {
                let text_galley = create_galley(key.tap.full.clone(), font.clone());
                let gap = 0.06 * size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    LabelGalleys {
                        symbol: Some(symbol_galley),
                        text: Some(text_galley),
                        hold: None,
                    }
                } else {
                    if let Some(short) = &key.tap.short {
                        let text_galley = create_galley(short.clone(), font.clone());
                        let gap = 0.06 * size;
                        let total_width =
                            symbol_galley.rect.width() + gap + text_galley.rect.width();
                        if total_width <= max_width {
                            LabelGalleys {
                                symbol: Some(symbol_galley),
                                text: Some(text_galley),
                                hold: None,
                            }
                        } else {
                            LabelGalleys {
                                symbol: Some(symbol_galley),
                                text: None,
                                hold: None,
                            }
                        }
                    } else {
                        LabelGalleys {
                            symbol: Some(symbol_galley),
                            text: None,
                            hold: None,
                        }
                    }
                }
            } else {
                LabelGalleys {
                    symbol: Some(symbol_galley),
                    text: None,
                    hold: None,
                }
            }
        } else {
            let full_galley = create_galley(key.tap.full.clone(), font.clone());
            if fits_width(&full_galley, max_width) {
                LabelGalleys {
                    symbol: None,
                    text: Some(full_galley),
                    hold: None,
                }
            } else {
                let mut truncated = if let Some(short) = &key.tap.short {
                    let short_galley = create_galley(short.clone(), font.clone());
                    if fits_width(&short_galley, max_width) {
                        return LabelGalleys {
                            symbol: None,
                            text: Some(short_galley),
                            hold: None,
                        };
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
                    LabelGalleys {
                        symbol: None,
                        text: Some(fitted_text),
                        hold: None,
                    }
                } else {
                    let mut text_galley = None;
                    while truncated.len() > 1 {
                        truncated.pop();
                        let truncated_with_ellipsis = format!("{}...", truncated);
                        let truncated_galley = create_galley(truncated_with_ellipsis, font.clone());
                        if fits_width(&truncated_galley, max_width) {
                            text_galley = Some(truncated_galley);
                            break;
                        }
                    }
                    LabelGalleys {
                        symbol: None,
                        text: text_galley,
                        hold: None,
                    }
                }
            }
        };

        if let Some(hold) = &key.hold {
            let hold_font = egui::FontId::proportional(0.20 * size * font_scale);
            let hold_galley = create_galley(hold.full.clone(), hold_font.clone());
            if fits_width(&hold_galley, max_width) {
                galleys.hold = Some(hold_galley);
            } else if let Some(short) = &hold.short {
                let short_galley = create_galley(short.clone(), hold_font);
                if fits_width(&short_galley, max_width) {
                    galleys.hold = Some(short_galley);
                }
            }
        }

        galleys
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

        if kind == KeycodeKind::Special {
            background_color = background_color.lerp_to_gamma(BLACK, 0.6);
        } else if kind == KeycodeKind::Modifier {
            background_color = background_color.lerp_to_gamma(BLACK, 0.3);
        }

        if pressed {
            return (
                background_color.lerp_to_gamma(egui::Color32::WHITE, 0.25),
                background_color.lerp_to_gamma(egui::Color32::WHITE, 0.8),
                0.08 * size,
                font_color.lerp_to_gamma(egui::Color32::WHITE, 0.5),
            );
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

                    let (fill_color, stroke_color, border_thickness, font_color) = self
                        .get_keycode_color(
                            layout_key.layer_ref.unwrap_or(effective_layer),
                            layout_key.kind,
                            is_background_key,
                            keyboard.is_key_pressed(key.row, key.col),
                        );

                    let rect = egui::Rect::from_min_size(
                        egui::pos2(key.x * size, key.y * size) + window_pos.to_vec2(),
                        egui::vec2(key.w * size, key.h * size),
                    )
                    .shrink(0.06 * size);

                    let angle = key.r.to_radians();
                    let origin = rect.center();

                    Self::add_rotated_shape(
                        ui.painter(),
                        egui::Shape::Rect(egui::epaint::RectShape::new(
                            rect,
                            0.1 * size,
                            fill_color,
                            egui::Stroke::new(border_thickness, stroke_color),
                            egui::StrokeKind::Outside,
                        )),
                        angle,
                        origin,
                    );

                    let font = egui::FontId::proportional(0.25 * size * font_scale);
                    let galleys =
                        self.generate_key_label_galleys(ui, &layout_key, rect, font, font_color);

                    let main_label_rect = if galleys.hold.is_some() {
                        egui::Rect::from_min_max(
                            rect.left_top(),
                            egui::pos2(rect.right(), rect.bottom() - rect.height() * 0.22),
                        )
                    } else {
                        rect
                    };

                    if let Some(hold_galley) = galleys.hold {
                        let hold_area_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.bottom() - rect.height() * 0.22),
                            rect.max,
                        );
                        let r = ((0.08 * size).clamp(0.0, 255.0)) as u8;

                        Self::add_rotated_shape(
                            ui.painter(),
                            egui::Shape::Rect(egui::epaint::RectShape::new(
                                hold_area_rect,
                                egui::CornerRadius {
                                    nw: 0,
                                    ne: 0,
                                    sw: r,
                                    se: r,
                                },
                                fill_color.lerp_to_gamma(egui::Color32::BLACK, 0.15),
                                egui::Stroke::NONE,
                                egui::StrokeKind::Outside,
                            )),
                            angle,
                            origin,
                        );

                        let hold_pos = hold_area_rect.center() - hold_galley.rect.center().to_vec2();
                        Self::add_rotated_shape(
                            ui.painter(),
                            egui::Shape::Text(egui::epaint::TextShape::new(
                                hold_pos,
                                hold_galley,
                                font_color.gamma_multiply(0.7),
                            )),
                            angle,
                            origin,
                        );
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
                            Self::add_rotated_shape(
                                ui.painter(),
                                egui::Shape::Text(egui::epaint::TextShape::new(
                                    sym_pos,
                                    symbol_galley,
                                    font_color,
                                )),
                                angle,
                                origin,
                            );
                            Self::add_rotated_shape(
                                ui.painter(),
                                egui::Shape::Text(egui::epaint::TextShape::new(
                                    text_pos,
                                    text_galley,
                                    font_color,
                                )),
                                angle,
                                origin,
                            );
                        }
                        (Some(symbol_galley), None) => {
                            let sym_pos =
                                main_label_rect.center() - symbol_galley.rect.center().to_vec2();
                            Self::add_rotated_shape(
                                ui.painter(),
                                egui::Shape::Text(egui::epaint::TextShape::new(
                                    sym_pos,
                                    symbol_galley,
                                    font_color,
                                )),
                                angle,
                                origin,
                            );
                        }
                        (None, Some(text_galley)) => {
                            let label_pos =
                                main_label_rect.center() - text_galley.rect.center().to_vec2();
                            Self::add_rotated_shape(
                                ui.painter(),
                                egui::Shape::Text(egui::epaint::TextShape::new(
                                    label_pos,
                                    text_galley,
                                    font_color,
                                )),
                                angle,
                                origin,
                            );
                        }
                        _ => {}
                    }
                }
            });
    }
}
