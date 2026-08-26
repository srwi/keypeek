use super::state::{KeyColors, LabelGalleys};
use super::OverlayApp;
use crate::keyboard::Keyboard;
use crate::layout_key::{BorderStyle, KeycodeKind, LayoutKey};
use crate::settings::{LegendMode, ThemeColor, WindowPosition};
use egui::Window;

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

/// Sample a rounded-rect outline into a closed polyline, rotated about `center`
/// by `angle` so a dashed/dotted border matches a tilted key.
fn rounded_rect_outline(
    rect: egui::Rect,
    radius: f32,
    center: egui::Pos2,
    angle: f32,
) -> Vec<egui::Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let r = radius
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5)
        .max(0.0);
    const SEG: usize = 4; // straight segments approximating each rounded corner
                          // (corner arc center, start angle, end angle), traced clockwise (screen y-down).
    let corners = [
        (
            egui::pos2(rect.right() - r, rect.top() + r),
            -FRAC_PI_2,
            0.0,
        ),
        (
            egui::pos2(rect.right() - r, rect.bottom() - r),
            0.0,
            FRAC_PI_2,
        ),
        (
            egui::pos2(rect.left() + r, rect.bottom() - r),
            FRAC_PI_2,
            PI,
        ),
        (
            egui::pos2(rect.left() + r, rect.top() + r),
            PI,
            PI + FRAC_PI_2,
        ),
    ];
    let mut points = Vec::with_capacity(corners.len() * (SEG + 1) + 1);
    for (arc_center, a0, a1) in corners {
        for i in 0..=SEG {
            let t = a0 + (a1 - a0) * (i as f32 / SEG as f32);
            let p = egui::pos2(arc_center.x + r * t.cos(), arc_center.y + r * t.sin());
            points.push(rotate_point(p, center, angle));
        }
    }
    // Close the loop so the dashed/dotted line spans the final edge too.
    if let Some(&first) = points.first() {
        points.push(first);
    }
    points
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
    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_key_label_galleys(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        font: egui::FontId,
        color: egui::Color32,
        shift_held: bool,
        ralt_held: bool,
    ) -> LabelGalleys {
        let (symbol, text) =
            self.generate_tap_galleys(ui, key, rect, font, color, shift_held, ralt_held);
        let behavior = self.generate_strip_galley(ui, key.behavior.as_ref(), rect, color);
        let argument = self.generate_strip_galley(ui, key.argument.as_ref(), rect, color);
        LabelGalleys {
            symbol,
            text,
            behavior,
            argument,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_tap_galleys(
        &self,
        ui: &egui::Ui,
        key: &LayoutKey,
        rect: egui::Rect,
        font: egui::FontId,
        color: egui::Color32,
        shift_held: bool,
        ralt_held: bool,
    ) -> (
        Option<std::sync::Arc<egui::Galley>>,
        Option<std::sync::Arc<egui::Galley>>,
    ) {
        let size = self.settings.active.size as f32;
        let font_scale = self.settings.active.font_size_multiplier;
        let create_galley =
            |text: String, fid: egui::FontId| ui.painter().layout_no_wrap(text, fid, color);
        let max_width = rect.width() * 0.85;
        let legend_mode = self.settings.active.legend_mode;
        let single_legend = legend_mode != LegendMode::Stacked;

        // Live preview: while Shift/RAlt is physically held somewhere on the
        // keyboard, show what THIS key would produce under that modifier
        // instead of its plain tap. It shows flat, with no stacking
        // (single-legend mode never stacks).
        if legend_mode == LegendMode::SingleLive {
            let live = if shift_held && ralt_held {
                // Both held: prefer the Shift+RAlt result where the layout
                // defines one, otherwise fall back to the single-modifier
                // results.
                key.ralt_shifted.as_ref().or(key.shifted.as_ref())
            } else if shift_held {
                key.shifted.as_ref()
            } else if ralt_held {
                key.ralt.as_ref()
            } else {
                None
            };
            if let Some(text) = live {
                return (None, Some(create_galley(text.clone(), font)));
            }
        }

        // Default (dual-legend): stack the shifted character above the base character.
        if !single_legend {
            if let Some(shifted) = &key.shifted {
                let text = if key.tap.is_empty() {
                    shifted.clone()
                } else {
                    format!("{}\n{}", shifted, key.tap.full)
                };
                let mut job = egui::text::LayoutJob {
                    halign: egui::Align::Center,
                    ..Default::default()
                };
                job.append(&text, 0.0, egui::TextFormat::simple(font, color));
                return (None, Some(ui.painter().layout_job(job)));
            }
        }

        if let Some(symbol) = &key.symbol {
            let symbol_font = egui::FontId::proportional(0.33 * size * font_scale);
            let symbol_galley = create_galley(symbol.clone(), symbol_font);
            let gap = 0.06 * size;

            let candidates = [
                (!key.tap.is_empty()).then(|| key.tap.full.clone()),
                key.tap.short.clone(),
            ];
            for text in candidates.into_iter().flatten() {
                let text_galley = create_galley(text, font.clone());
                let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
                if total_width <= max_width {
                    return (Some(symbol_galley), Some(text_galley));
                }
            }

            return (Some(symbol_galley), None);
        }

        (
            None,
            self.fit_text_galley(
                ui,
                &key.tap.full,
                key.tap.short.as_deref(),
                font,
                color,
                egui::vec2(max_width, rect.height() * 0.85),
            ),
        )
    }

    /// `(font_size, strip_height)` for the legend strips, scaled to the tap font so
    /// the strips grow with the text rather than the key height.
    fn strip_metrics(&self) -> (f32, f32) {
        let size = self.settings.active.size as f32;
        let font_scale = self.settings.active.font_size_multiplier;
        // 0.55x the main tap font, so the legend stays a bit smaller.
        let font_size = 0.55 * 0.25 * size * font_scale;
        // Single text line is ~1.16x the font size; add a little vertical padding.
        let strip_height = font_size * 1.4;
        (font_size, strip_height)
    }

    /// Lay out an optional strip label (behavior name or argument) to fit its strip.
    fn generate_strip_galley(
        &self,
        ui: &egui::Ui,
        label: Option<&crate::layout_key::Label>,
        rect: egui::Rect,
        color: egui::Color32,
    ) -> Option<std::sync::Arc<egui::Galley>> {
        let label = label?;
        let max_width = rect.width() * 0.85;
        let (font_size, strip_height) = self.strip_metrics();
        let strip_font = egui::FontId::proportional(font_size);
        self.fit_text_galley(
            ui,
            &label.full,
            label.short.as_deref(),
            strip_font,
            color,
            egui::vec2(max_width, strip_height),
        )
    }

    /// Lay out `full` (or `short`) within `max_width`; when neither fits, either
    /// scale down (`auto_fit_before_ellipsis`) or truncate with an ellipsis.
    fn fit_text_galley(
        &self,
        ui: &egui::Ui,
        full: &str,
        short: Option<&str>,
        font: egui::FontId,
        color: egui::Color32,
        max: egui::Vec2,
    ) -> Option<std::sync::Arc<egui::Galley>> {
        let (max_width, max_height) = (max.x, max.y);
        let create_galley =
            |text: String, fid: egui::FontId| ui.painter().layout_no_wrap(text, fid, color);
        let fits_width = |galley: &std::sync::Arc<egui::Galley>| galley.rect.width() <= max_width;

        let full_galley = create_galley(full.to_string(), font.clone());
        if fits_width(&full_galley) {
            return Some(full_galley);
        }

        let mut truncated = if let Some(short) = short {
            let short_galley = create_galley(short.to_string(), font.clone());
            if fits_width(&short_galley) {
                return Some(short_galley);
            }
            short.to_string()
        } else {
            full.to_string()
        };

        if self.settings.active.auto_fit_before_ellipsis {
            let fit_text = short.unwrap_or(full).to_string();
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
            return Some(create_galley(
                fit_text,
                egui::FontId::proportional(font.size * scale),
            ));
        }

        while truncated.len() > 1 {
            truncated.pop();
            let truncated_with_ellipsis = format!("{}...", truncated);
            let truncated_galley = create_galley(truncated_with_ellipsis, font.clone());
            if fits_width(&truncated_galley) {
                return Some(truncated_galley);
            }
        }

        None
    }

    /// Paint one legend strip along a key edge. `top` selects which corners are
    /// rounded; `strip` is the un-rotated edge rect, rotated to the key's `angle`.
    #[allow(clippy::too_many_arguments)]
    fn paint_strip(
        &self,
        ui: &egui::Ui,
        strip: egui::Rect,
        galley: Option<std::sync::Arc<egui::Galley>>,
        top: bool,
        size: f32,
        center: egui::Pos2,
        angle: f32,
        background: egui::Color32,
        font_color: egui::Color32,
    ) {
        // RectShape rotates around its own center, so orbit the strip's center
        // around the key center first, then tilt it in place.
        let strip_rect =
            egui::Rect::from_center_size(rotate_point(strip.center(), center, angle), strip.size());
        let radius = (0.08 * size) as u8;
        let corners = if top {
            egui::CornerRadius {
                nw: radius,
                ne: radius,
                sw: 0,
                se: 0,
            }
        } else {
            egui::CornerRadius {
                nw: 0,
                ne: 0,
                sw: radius,
                se: radius,
            }
        };
        ui.painter().add(
            egui::epaint::RectShape::new(
                strip_rect,
                corners,
                background,
                egui::Stroke::NONE,
                egui::StrokeKind::Outside,
            )
            .with_angle(angle),
        );
        if let Some(galley) = galley {
            let pos = strip.center() - galley.rect.center().to_vec2();
            ui.painter().add(rotated_text_shape(
                pos,
                galley,
                font_color.gamma_multiply(0.7),
                center,
                angle,
            ));
        }
    }

    /// Paint a key's border, rotated to match the key. `Solid`/`None` use the native
    /// stroke; `Dashed`/`Dotted` trace the outline as styled segments.
    #[allow(clippy::too_many_arguments)]
    fn paint_key_border(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        corner_radius: f32,
        stroke: egui::Stroke,
        style: BorderStyle,
        size: f32,
        center: egui::Pos2,
        angle: f32,
    ) {
        match style {
            BorderStyle::None | BorderStyle::Solid => {
                ui.painter().add(
                    egui::epaint::RectShape::stroke(
                        rect,
                        corner_radius,
                        stroke,
                        egui::StrokeKind::Outside,
                    )
                    .with_angle(angle),
                );
            }
            BorderStyle::Dashed => {
                let points = rounded_rect_outline(rect, corner_radius, center, angle);
                for shape in egui::Shape::dashed_line(&points, stroke, 0.09 * size, 0.06 * size) {
                    ui.painter().add(shape);
                }
            }
        }
    }

    pub(super) fn get_keycode_color(
        &self,
        layer: u8,
        kind: KeycodeKind,
        desaturate: bool,
        pressed: bool,
    ) -> KeyColors {
        const DESATURATE_FACTOR: f32 = 0.7;

        const BLACK: egui::Color32 = egui::Color32::BLACK;

        let size = self.settings.active.size as f32;
        let layer_theme_color = self.settings.active.theme.layer_color(layer);
        let mut background_color = Self::to_egui_color(layer_theme_color);
        let mut font_color = Self::to_egui_color(self.settings.active.theme.font_color);

        if pressed {
            return KeyColors {
                fill: background_color.lerp_to_gamma(egui::Color32::WHITE, 0.2),
                border: background_color.lerp_to_gamma(egui::Color32::WHITE, 0.7),
                border_thickness: 0.03 * size,
                font: font_color.lerp_to_gamma(egui::Color32::WHITE, 0.4),
            };
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

        KeyColors {
            fill: background_color,
            border: border_color,
            border_thickness: 1.0,
            font: font_color,
        }
    }

    pub(super) fn to_egui_color(color: ThemeColor) -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(color.r, color.g, color.b, color.a)
    }

    pub(super) fn from_egui_color(color: egui::Color32) -> ThemeColor {
        ThemeColor::new(color.r(), color.g(), color.b(), color.a())
    }

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
        let font_scale = self.settings.active.font_size_multiplier;
        let pinned = self.ui.pinned_layer;

        let overlay_response = Window::new("KeyPeek")
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
                    let KeyColors {
                        fill: mut fill_color,
                        border: mut stroke_color,
                        mut border_thickness,
                        font: font_color,
                    } = self.get_keycode_color(
                        layout_key.layer_ref.unwrap_or(effective_layer),
                        first_layer_key_kind,
                        is_background_key,
                        pressed,
                    );

                    if transparent {
                        fill_color = fill_color.gamma_multiply(0.25);
                        stroke_color = fill_color;
                        border_thickness = 1.0;
                    }

                    let rect = egui::Rect::from_min_size(
                        egui::pos2(key.x * size, key.y * size) + window_pos.to_vec2(),
                        egui::vec2(key.w * size, key.h * size),
                    )
                    .shrink(0.06 * size);

                    let angle = key.r.to_radians();
                    let center = rect.center();
                    let corner_radius = 0.1 * size;

                    // Fill first; the border is drawn separately so layer keys can carry
                    // a styled outline. The pressed outline always wins; otherwise a layer
                    // key uses a heavier styled border hinting how its layer activates.
                    ui.painter().add(
                        egui::epaint::RectShape::filled(rect, corner_radius, fill_color)
                            .with_angle(angle),
                    );

                    let (border_style, border_width, border_color) =
                        if pressed || layout_key.border == BorderStyle::None {
                            (BorderStyle::Solid, border_thickness, stroke_color)
                        } else {
                            // Brighten the key's fill color for the outline so it stands out on-theme.
                            (
                                layout_key.border,
                                0.02 * size,
                                fill_color.lerp_to_gamma(egui::Color32::WHITE, 0.45),
                            )
                        };
                    self.paint_key_border(
                        ui,
                        rect,
                        corner_radius,
                        egui::Stroke::new(border_width, border_color),
                        border_style,
                        size,
                        center,
                        angle,
                    );

                    let font = egui::FontId::proportional(0.25 * size * font_scale);
                    let galleys = self.generate_key_label_galleys(
                        ui,
                        &layout_key,
                        rect,
                        font,
                        font_color,
                        shift_held,
                        ralt_held,
                    );

                    // Draw the legend strips: behavior on top, argument on bottom. They
                    // overlay the key's edges (the primary label stays centered) and are
                    // tied to the legend existing, not to whether the text fits, so an
                    // over-long legend never blanks out.
                    let strip_height = self.strip_metrics().1;
                    let has_behavior = layout_key.behavior.is_some();
                    let has_argument = layout_key.argument.is_some();

                    if has_behavior {
                        let strip = egui::Rect::from_min_max(
                            rect.left_top(),
                            egui::pos2(rect.right(), rect.top() + strip_height),
                        );
                        self.paint_strip(
                            ui,
                            strip,
                            galleys.behavior,
                            true,
                            size,
                            center,
                            angle,
                            stroke_color,
                            font_color,
                        );
                    }

                    if has_argument {
                        let strip = egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.bottom() - strip_height),
                            rect.max,
                        );
                        self.paint_strip(
                            ui,
                            strip,
                            galleys.argument,
                            false,
                            size,
                            center,
                            angle,
                            stroke_color,
                            font_color,
                        );
                    }

                    let draw_text = |pos, galley| {
                        ui.painter()
                            .add(rotated_text_shape(pos, galley, font_color, center, angle));
                    };
                    match (galleys.symbol, galleys.text) {
                        (Some(symbol_galley), Some(text_galley)) => {
                            let gap = 0.06 * size;
                            let total_width =
                                symbol_galley.rect.width() + gap + text_galley.rect.width();
                            let start_x = center.x - total_width * 0.5;

                            let text_pos_x = start_x + gap + symbol_galley.rect.width();
                            let text_pos =
                                egui::pos2(text_pos_x, center.y - text_galley.rect.center().y);
                            let sym_pos =
                                egui::pos2(start_x, center.y - symbol_galley.rect.center().y);
                            draw_text(sym_pos, symbol_galley);
                            draw_text(text_pos, text_galley);
                        }
                        (Some(symbol_galley), None) => {
                            let sym_pos = center - symbol_galley.rect.center().to_vec2();
                            draw_text(sym_pos, symbol_galley);
                        }
                        (None, Some(text_galley)) => {
                            let label_pos = center - text_galley.rect.center().to_vec2();
                            draw_text(label_pos, text_galley);
                        }
                        _ => {}
                    }
                }
            });

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
