//! Shared key rendering engine for the overlay and keycode pickers.

use crate::layout_key::{BorderStyle, KeycodeKind, LayoutKey};
use crate::settings::{LegendMode, Settings, ThemeColor, ThemeSettings};

/// Color palette for painting a key.
#[derive(Clone, Copy)]
pub struct KeyColors {
    pub fill: egui::Color32,
    pub border: egui::Color32,
    pub border_thickness: f32,
    pub font: egui::Color32,
}

impl KeyColors {
    /// Returns dimmed colors for transparent and unbound keys.
    pub fn ghosted(mut self) -> Self {
        self.fill = self.fill.gamma_multiply(0.25);
        self.border = self.fill;
        self.border_thickness = 1.0;
        self
    }

    /// Returns ghosted colors if `condition` is true.
    pub fn ghosted_if(self, condition: bool) -> Self {
        if condition {
            self.ghosted()
        } else {
            self
        }
    }

    /// Returns the highlight border color for hover and selection outlines.
    pub fn highlight_border(&self) -> egui::Color32 {
        self.fill.lerp_to_gamma(egui::Color32::WHITE, 0.45)
    }
}

/// Rendering style settings for keys.
pub struct KeyPaintStyle {
    /// Key unit size in pixels.
    pub unit: f32,
    /// Text size multiplier.
    pub font_scale: f32,
    /// Single-legend display mode.
    pub single_legend: bool,
    /// Dynamic legend update mode for held modifiers.
    pub live_legends: bool,
    /// Scale text down to fit before truncating.
    pub auto_fit_before_ellipsis: bool,
    /// Color theme settings.
    pub theme: ThemeSettings,
}

impl KeyPaintStyle {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            unit: settings.size as f32,
            font_scale: settings.font_size_multiplier,
            single_legend: settings.legend_mode != LegendMode::Stacked,
            live_legends: settings.legend_mode == LegendMode::SingleLive,
            auto_fit_before_ellipsis: settings.auto_fit_before_ellipsis,
            theme: settings.theme.clone(),
        }
    }

    /// Sets the key unit size in pixels.
    pub fn with_unit(mut self, unit: f32) -> Self {
        self.unit = unit;
        self
    }

    /// Calculates font size and height for behavior and argument strips.
    fn strip_metrics(&self) -> (f32, f32) {
        let font_size = 0.55 * 0.25 * self.unit * self.font_scale;
        let strip_height = font_size * 1.4;
        (font_size, strip_height)
    }

    /// Calculates colors for a key based on its layer, kind, and pressed state.
    pub fn colors_for(
        &self,
        layer: u8,
        kind: KeycodeKind,
        desaturate: bool,
        pressed: bool,
    ) -> KeyColors {
        const DESATURATE_FACTOR: f32 = 0.7;

        const BLACK: egui::Color32 = egui::Color32::BLACK;

        let unit = self.unit;
        let layer_theme_color = self.theme.layer_color(layer);
        let mut background_color = to_egui_color(layer_theme_color);
        let mut font_color = to_egui_color(self.theme.font_color);

        if pressed {
            return KeyColors {
                fill: background_color.lerp_to_gamma(egui::Color32::WHITE, 0.2),
                border: background_color.lerp_to_gamma(egui::Color32::WHITE, 0.7),
                border_thickness: 0.03 * unit,
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
            let layer0_color = to_egui_color(self.theme.layer_colors[0]);
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
}

/// Display state for a single key.
pub struct KeyDisplay<'a> {
    pub key: &'a LayoutKey,
    pub colors: KeyColors,
    pub hovered: bool,
    /// Key is pressed.
    pub pressed: bool,
    pub shift_held: bool,
    pub ralt_held: bool,
}

/// Paints a key into the given rectangle with rotation.
pub fn paint(
    ui: &egui::Ui,
    rect: egui::Rect,
    angle: f32,
    display: &KeyDisplay<'_>,
    style: &KeyPaintStyle,
) {
    let unit = style.unit;
    let rect = rect.shrink(0.06 * unit);
    let center = rect.center();
    let corner_radius = 0.1 * unit;
    let key = display.key;

    ui.painter().add(
        egui::epaint::RectShape::filled(rect, corner_radius, display.colors.fill).with_angle(angle),
    );

    let (border_style, border_width, mut border_color) =
        if display.pressed || key.border == BorderStyle::None {
            (
                BorderStyle::Solid,
                display.colors.border_thickness,
                display.colors.border,
            )
        } else {
            (key.border, 0.02 * unit, display.colors.highlight_border())
        };
    if display.hovered {
        border_color = display.colors.highlight_border();
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    paint_border(
        ui,
        rect,
        corner_radius,
        egui::Stroke::new(border_width, border_color),
        border_style,
        unit,
        center,
        angle,
    );

    let font = egui::FontId::proportional(0.25 * unit * style.font_scale);
    let galleys = generate_label_galleys(ui, display, rect, font, style);

    // Draw the legend strips: behavior on top, argument on bottom. They overlay
    // the key's edges (the primary label stays centered) and are tied to the
    // legend existing, not to whether the text fits, so an over-long legend
    // never blanks out.
    let strip_height = style.strip_metrics().1;
    let has_behavior = key.behavior.is_some();
    let has_argument = key.argument.is_some();

    if has_behavior {
        let strip = egui::Rect::from_min_max(
            rect.left_top(),
            egui::pos2(rect.right(), rect.top() + strip_height),
        );
        paint_strip(
            ui,
            strip,
            galleys.behavior,
            true,
            unit,
            center,
            angle,
            display.colors.border,
            display.colors.font,
        );
    }

    if has_argument {
        let strip = egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - strip_height),
            rect.max,
        );
        paint_strip(
            ui,
            strip,
            galleys.argument,
            false,
            unit,
            center,
            angle,
            display.colors.border,
            display.colors.font,
        );
    }

    let draw_text = |pos, galley| {
        ui.painter().add(rotated_text_shape(
            pos,
            galley,
            display.colors.font,
            center,
            angle,
        ));
    };
    match (galleys.symbol, galleys.text) {
        (Some(symbol_galley), Some(text_galley)) => {
            let gap = 0.06 * unit;
            let total_width = symbol_galley.rect.width() + gap + text_galley.rect.width();
            let start_x = center.x - total_width * 0.5;

            let text_pos_x = start_x + gap + symbol_galley.rect.width();
            let text_pos = egui::pos2(text_pos_x, center.y - text_galley.rect.center().y);
            let sym_pos = egui::pos2(start_x, center.y - symbol_galley.rect.center().y);
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

struct LabelGalleys {
    symbol: Option<std::sync::Arc<egui::Galley>>,
    text: Option<std::sync::Arc<egui::Galley>>,
    behavior: Option<std::sync::Arc<egui::Galley>>,
    argument: Option<std::sync::Arc<egui::Galley>>,
}

/// Generates text galleys for all key legends.
fn generate_label_galleys(
    ui: &egui::Ui,
    display: &KeyDisplay<'_>,
    rect: egui::Rect,
    font: egui::FontId,
    style: &KeyPaintStyle,
) -> LabelGalleys {
    let (symbol, text) = generate_tap_galleys(ui, display, rect, font, style);
    let color = display.colors.font;
    let behavior = generate_strip_galley(ui, display.key.behavior.as_ref(), rect, color, style);
    let argument = generate_strip_galley(ui, display.key.argument.as_ref(), rect, color, style);
    LabelGalleys {
        symbol,
        text,
        behavior,
        argument,
    }
}

fn generate_tap_galleys(
    ui: &egui::Ui,
    display: &KeyDisplay<'_>,
    rect: egui::Rect,
    font: egui::FontId,
    style: &KeyPaintStyle,
) -> (
    Option<std::sync::Arc<egui::Galley>>,
    Option<std::sync::Arc<egui::Galley>>,
) {
    let key = display.key;
    let color = display.colors.font;
    let unit = style.unit;
    let font_scale = style.font_scale;
    let create_galley =
        |text: String, fid: egui::FontId| ui.painter().layout_no_wrap(text, fid, color);
    let max_width = rect.width() * 0.85;
    let single_legend = style.single_legend;

    // Display active modifier result in live-legend mode.
    if style.live_legends {
        let live = if display.shift_held && display.ralt_held {
            key.ralt_shifted.as_ref().or(key.shifted.as_ref())
        } else if display.shift_held {
            key.shifted.as_ref()
        } else if display.ralt_held {
            key.ralt.as_ref()
        } else {
            None
        };
        if let Some(text) = live {
            return (None, Some(create_galley(text.clone(), font)));
        }
    }

    // Stack shifted character above base character in dual-legend mode.
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
        let symbol_font = egui::FontId::proportional(0.33 * unit * font_scale);
        let symbol_galley = create_galley(symbol.clone(), symbol_font);
        let gap = 0.06 * unit;

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
        fit_text_galley(
            ui,
            &key.tap.full,
            key.tap.short.as_deref(),
            font,
            color,
            egui::vec2(max_width, rect.height() * 0.85),
            style,
        ),
    )
}

/// Generates a text galley for a behavior or argument strip.
fn generate_strip_galley(
    ui: &egui::Ui,
    label: Option<&crate::layout_key::Label>,
    rect: egui::Rect,
    color: egui::Color32,
    style: &KeyPaintStyle,
) -> Option<std::sync::Arc<egui::Galley>> {
    let label = label?;
    let max_width = rect.width() * 0.85;
    let (font_size, strip_height) = style.strip_metrics();
    let strip_font = egui::FontId::proportional(font_size);
    fit_text_galley(
        ui,
        &label.full,
        label.short.as_deref(),
        strip_font,
        color,
        egui::vec2(max_width, strip_height),
        style,
    )
}

/// Formats text into a galley fitted to the maximum dimensions.
fn fit_text_galley(
    ui: &egui::Ui,
    full: &str,
    short: Option<&str>,
    font: egui::FontId,
    color: egui::Color32,
    max: egui::Vec2,
    style: &KeyPaintStyle,
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

    if style.auto_fit_before_ellipsis {
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

/// Rotates a point clockwise around an origin by the specified angle in radians.
pub fn rotate_point(point: egui::Pos2, origin: egui::Pos2, angle_rad: f32) -> egui::Pos2 {
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

/// Generates a rotated rounded rectangle outline path.
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
    const SEG: usize = 4;
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
    if let Some(&first) = points.first() {
        points.push(first);
    }
    points
}

/// Creates a rotated text shape.
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

/// Paints a behavior or argument strip along a key edge.
#[allow(clippy::too_many_arguments)]
fn paint_strip(
    ui: &egui::Ui,
    strip: egui::Rect,
    galley: Option<std::sync::Arc<egui::Galley>>,
    top: bool,
    unit: f32,
    center: egui::Pos2,
    angle: f32,
    background: egui::Color32,
    font_color: egui::Color32,
) {
    let strip_rect =
        egui::Rect::from_center_size(rotate_point(strip.center(), center, angle), strip.size());
    let radius = (0.08 * unit) as u8;
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

/// Paints a key border with rotation.
#[allow(clippy::too_many_arguments)]
fn paint_border(
    ui: &egui::Ui,
    rect: egui::Rect,
    corner_radius: f32,
    stroke: egui::Stroke,
    style: BorderStyle,
    unit: f32,
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
            for shape in egui::Shape::dashed_line(&points, stroke, 0.09 * unit, 0.06 * unit) {
                ui.painter().add(shape);
            }
        }
    }
}

pub fn to_egui_color(color: ThemeColor) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(color.r, color.g, color.b, color.a)
}

pub fn from_egui_color(color: egui::Color32) -> ThemeColor {
    ThemeColor::new(color.r(), color.g(), color.b(), color.a())
}
