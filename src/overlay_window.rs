use crate::keyboard::Keyboard;
use crate::layout_key::{KeycodeKind, LayoutKey};
use crate::settings::WindowPosition;

use eframe::egui::{self, Align2, Window};
use std::time::Instant;

struct LabelGalleys {
    symbol: Option<std::sync::Arc<egui::Galley>>,
    text: Option<std::sync::Arc<egui::Galley>>,
}

pub struct Overlay {
    keyboard: Keyboard,
    size: f32,
    margin: u32,
    position: WindowPosition,
}

impl Overlay {
    pub fn new(keyboard: Keyboard, size: i32, margin: u32, position: WindowPosition) -> Self {
        Self {
            keyboard,
            margin,
            position,
            size: size as f32,
        }
    }

    fn generate_key_label_galleys(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        font: egui::FontId,
        color: egui::Color32,
    ) -> LabelGalleys {
        let create_galley =
            |text: String, fid: egui::FontId| ui.painter().layout_no_wrap(text, fid, color);
        let fits_width =
            |galley: &std::sync::Arc<egui::Galley>, max: f32| galley.rect.width() <= max;
        let max_width = rect.width() * 0.85;

        if let Some(symbol) = &key.symbol {
            let symbol_font = egui::FontId::proportional(0.33 * self.size);
            let symbol_galley = create_galley(symbol.clone(), symbol_font);

            // Try to fit symbol + tap (full) label
            if !key.tap.is_empty() {
                let text_galley = create_galley(key.tap.full.clone(), font.clone());
                let gap = 0.06 * self.size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return LabelGalleys {
                        symbol: Some(symbol_galley),
                        text: Some(text_galley),
                    };
                }
            }

            // Try to fit symbol + short label
            if let Some(short) = &key.tap.short {
                let text_galley = create_galley(short.clone(), font.clone());
                let gap = 0.06 * self.size;
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return LabelGalleys {
                        symbol: Some(symbol_galley),
                        text: Some(text_galley),
                    };
                }
            }

            // Symbol only
            return LabelGalleys {
                symbol: Some(symbol_galley),
                text: None,
            };
        }

        // Try fitting tap (full) label
        let full_galley = create_galley(key.tap.full.clone(), font.clone());
        if fits_width(&full_galley, max_width) {
            return LabelGalleys {
                symbol: None,
                text: Some(full_galley),
            };
        }

        // Try fitting short label or truncated tap label
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

    fn get_anchor_params(&self) -> (Align2, egui::Vec2) {
        match self.position {
            WindowPosition::TopLeft => (
                Align2::LEFT_TOP,
                egui::vec2(self.margin as f32, self.margin as f32),
            ),
            WindowPosition::TopRight => (
                Align2::RIGHT_TOP,
                egui::vec2(-(self.margin as f32), self.margin as f32),
            ),
            WindowPosition::BottomLeft => (
                Align2::LEFT_BOTTOM,
                egui::vec2(self.margin as f32, -(self.margin as f32)),
            ),
            WindowPosition::BottomRight => (
                Align2::RIGHT_BOTTOM,
                egui::vec2(-(self.margin as f32), -(self.margin as f32)),
            ),
            WindowPosition::Bottom => (
                Align2::CENTER_BOTTOM,
                egui::vec2(0.0, -(self.margin as f32)),
            ),
            WindowPosition::Top => (Align2::CENTER_TOP, egui::vec2(0.0, self.margin as f32)),
        }
    }

    pub fn get_keycode_color(
        &self,
        layer: u8,
        kind: KeycodeKind,
        desaturate: bool,
        pressed: bool,
    ) -> (egui::Color32, egui::Color32, f32, egui::Color32) {
        const ALPHA: u8 = 239;
        const DESATURATE_FACTOR: f32 = 0.7;

        const BLACK: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, ALPHA);
        const LAYER_0: egui::Color32 = egui::Color32::from_rgba_premultiplied(83, 83, 83, ALPHA);
        const LAYER_1: egui::Color32 = egui::Color32::from_rgba_premultiplied(80, 140, 115, ALPHA);
        const LAYER_2: egui::Color32 = egui::Color32::from_rgba_premultiplied(100, 115, 150, ALPHA);
        const LAYER_3: egui::Color32 = egui::Color32::from_rgba_premultiplied(140, 110, 150, ALPHA);
        const LAYER_4: egui::Color32 = egui::Color32::from_rgba_premultiplied(95, 121, 127, ALPHA);
        const LAYER_5: egui::Color32 = egui::Color32::from_rgba_premultiplied(147, 137, 110, ALPHA);
        const LAYER_N: egui::Color32 = egui::Color32::from_rgba_premultiplied(127, 127, 127, ALPHA);

        let mut background_color = match layer {
            0 => LAYER_0,
            1 => LAYER_1,
            2 => LAYER_2,
            3 => LAYER_3,
            4 => LAYER_4,
            5 => LAYER_5,
            _ => LAYER_N,
        };

        if pressed {
            return (
                background_color.lerp_to_gamma(egui::Color32::WHITE, 0.2),
                background_color.lerp_to_gamma(egui::Color32::WHITE, 0.7),
                0.03 * self.size,
                egui::Color32::WHITE,
            );
        }

        if kind == KeycodeKind::Special {
            background_color = background_color.lerp_to_gamma(BLACK, 0.6);
        } else if kind == KeycodeKind::Modifier {
            background_color = background_color.lerp_to_gamma(BLACK, 0.3);
        }

        let mut border_color = background_color.lerp_to_gamma(BLACK, 0.2);

        // Never desaturate layer 0
        if desaturate && layer != 0 {
            background_color = background_color.lerp_to_gamma(LAYER_0, DESATURATE_FACTOR);
            border_color = border_color.lerp_to_gamma(LAYER_0, DESATURATE_FACTOR);
        }

        let font_color = if desaturate {
            egui::Color32::WHITE.gamma_multiply(1.0 - DESATURATE_FACTOR)
        } else {
            egui::Color32::WHITE
        };

        (background_color, border_color, 1.0, font_color)
    }
}

impl eframe::App for Overlay {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));

        let mut window_open = match self.keyboard.time_to_hide_overlay.lock().unwrap().as_ref() {
            Some(time_to_hide) => Instant::now() < *time_to_hide,
            None => true,
        };

        let anchor_params = self.get_anchor_params();

        Window::new("QMK Layout Helper")
            .open(&mut window_open)
            .auto_sized()
            .anchor(anchor_params.0, anchor_params.1)
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .fade_out(true)
            .title_bar(false)
            .show(ctx, |ui| {
                // Allow auto_sized window to shrink to fit content
                let layout_size = self.keyboard.layout.get_dimensions();
                ui.allocate_space(egui::vec2(
                    layout_size.0 * self.size,
                    layout_size.1 * self.size,
                ));

                let window_pos = ui.min_rect().min;

                for key in &self.keyboard.layout.keys {
                    let (effective_layer, is_background_key) =
                        self.keyboard.get_effective_key_layer(key.row, key.col);

                    // Get the LayoutKey directly (already contains display labels)
                    let layout_key = self
                        .keyboard
                        .get_key(effective_layer as usize, key.row, key.col)
                        .unwrap_or_default();

                    // Get layer 0 key's kind for coloring
                    let first_layer_key_kind = self
                        .keyboard
                        .get_key(0, key.row, key.col)
                        .map(|k| k.kind)
                        .unwrap_or(KeycodeKind::Basic);

                    let (fill_color, stroke_color, border_thickness, font_color) = self
                        .get_keycode_color(
                            layout_key.layer_ref.unwrap_or(effective_layer),
                            first_layer_key_kind,
                            is_background_key,
                            self.keyboard.is_key_pressed(key.row, key.col),
                        );

                    // Draw key background
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(key.x * self.size, key.y * self.size) + window_pos.to_vec2(),
                        egui::vec2(key.w * self.size, key.h * self.size),
                    )
                    .shrink(0.06 * self.size);
                    ui.painter().rect(
                        rect,
                        0.1 * self.size,
                        fill_color,
                        egui::Stroke::new(border_thickness, stroke_color),
                        egui::StrokeKind::Outside,
                    );

                    // Draw key label and optional symbol
                    let font = egui::FontId::proportional(0.25 * self.size);
                    match self.generate_key_label_galleys(
                        ui,
                        &layout_key,
                        rect,
                        font.clone(),
                        font_color,
                    ) {
                        LabelGalleys {
                            symbol: Some(symbol_galley),
                            text: Some(text_galley),
                        } => {
                            let gap = 0.06 * self.size;
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

        ctx.request_repaint();
    }
}
