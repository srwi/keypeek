use super::state::LabelGalleys;
use super::OverlayApp;
use crate::keyboard::Keyboard;
use crate::layout_key::{KeycodeKind, LayoutKey};
use crate::settings::ThemeColor;
use eframe::egui::{self, Window};

impl OverlayApp {
    pub(super) fn generate_key_label_galleys(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        font: egui::FontId,
        color: egui::Color32,
    ) -> LabelGalleys {
        let size = self.settings.active.size as f32;
        let create_galley =
            |text: String, fid: egui::FontId| ui.painter().layout_no_wrap(text, fid, color);
        let fits_width =
            |galley: &std::sync::Arc<egui::Galley>, max: f32| galley.rect.width() <= max;
        let max_width = rect.width() * 0.85;

        if let Some(symbol) = &key.symbol {
            let symbol_font = egui::FontId::proportional(0.33 * size);
            let symbol_galley = create_galley(symbol.clone(), symbol_font);

            if !key.tap.is_empty() {
                let text_galley = create_galley(key.tap.full.clone(), font.clone());
                let gap = 0.06 * size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return LabelGalleys {
                        symbol: Some(symbol_galley),
                        text: Some(text_galley),
                    };
                }
            }

            if let Some(short) = &key.tap.short {
                let text_galley = create_galley(short.clone(), font.clone());
                let gap = 0.06 * size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return LabelGalleys {
                        symbol: Some(symbol_galley),
                        text: Some(text_galley),
                    };
                }
            }

            return LabelGalleys {
                symbol: Some(symbol_galley),
                text: None,
            };
        }

        let full_galley = create_galley(key.tap.full.clone(), font.clone());
        if fits_width(&full_galley, max_width) {
            return LabelGalleys {
                symbol: None,
                text: Some(full_galley),
            };
        }

        let mut truncated = if let Some(short) = &key.tap.short {
            let short_galley = create_galley(short.clone(), font.clone());
            if fits_width(&short_galley, max_width) {
                return LabelGalleys {
                    symbol: None,
                    text: Some(short_galley),
                };
            }
            short.clone()
        } else {
            key.tap.full.clone()
        };

        while truncated.len() > 1 {
            truncated.pop();
            let truncated_with_ellipsis = format!("{}...", truncated);
            let truncated_galley = create_galley(truncated_with_ellipsis, font.clone());
            if fits_width(&truncated_galley, max_width) {
                return LabelGalleys {
                    symbol: None,
                    text: Some(truncated_galley),
                };
            }
        }

        LabelGalleys {
            symbol: None,
            text: None,
        }
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

        Window::new("QMK Layout Helper")
            .open(&mut window_open)
            .auto_sized()
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

                    ui.painter().rect(
                        rect,
                        0.1 * size,
                        fill_color,
                        egui::Stroke::new(border_thickness, stroke_color),
                        egui::StrokeKind::Outside,
                    );

                    let font = egui::FontId::proportional(0.25 * size);
                    match self.generate_key_label_galleys(ui, &layout_key, rect, font, font_color) {
                        LabelGalleys {
                            symbol: Some(symbol_galley),
                            text: Some(text_galley),
                        } => {
                            let gap = 0.06 * size;
                            let total_width =
                                symbol_galley.rect.width() + gap + text_galley.rect.width();
                            let start_x = rect.center().x - total_width * 0.5;

                            let text_pos_x = start_x + gap + symbol_galley.rect.width();
                            let text_pos = egui::pos2(
                                text_pos_x,
                                rect.center().y - text_galley.rect.center().y,
                            );
                            let sym_pos = egui::pos2(
                                start_x,
                                rect.center().y - symbol_galley.rect.center().y,
                            );
                            ui.painter().galley(sym_pos, symbol_galley, font_color);
                            ui.painter().galley(text_pos, text_galley, font_color);
                        }
                        LabelGalleys {
                            symbol: Some(symbol_galley),
                            text: None,
                        } => {
                            let sym_pos = rect.center() - symbol_galley.rect.center().to_vec2();
                            ui.painter().galley(sym_pos, symbol_galley, font_color);
                        }
                        LabelGalleys {
                            symbol: None,
                            text: Some(text_galley),
                        } => {
                            let label_pos = rect.center() - text_galley.rect.center().to_vec2();
                            ui.painter().galley(label_pos, text_galley, font_color);
                        }
                        _ => {}
                    }
                }
            });
    }
}
