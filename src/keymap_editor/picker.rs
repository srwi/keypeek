//! A scrollable grid of key-shaped buttons shared by the QMK and ZMK editors.
//! The cells are painted by [`crate::key_paint`], so each candidate looks
//! exactly like it will on the live overlay.

use crate::key_paint::{self, KeyDisplay, KeyPaintStyle};
use crate::layout_key::LayoutKey;

/// Pixels per key-unit in picker grids; a miniature overlay key.
pub const KEY_UNIT: f32 = 51.0;
/// Gap between grid cells, matching the old button-grid rhythm.
const GAP: f32 = 6.0;

/// One selectable candidate in a picker grid.
pub struct Candidate {
    /// The firmware value the candidate writes.
    pub code: u32,
    /// The fully resolved key: labels, symbol, and kind (for coloring).
    pub key: LayoutKey,
    /// Rendered dimmed like an unset overlay slot (QMK `KC_TRANSPARENT`).
    pub transparent: bool,
}

impl Candidate {
    pub fn new(code: u32, key: LayoutKey) -> Self {
        Self {
            code,
            key,
            transparent: false,
        }
    }
}

/// A scrollable grid of miniature keys, one per candidate. The currently
/// assigned `selected` code is highlighted with the overlay's pressed look.
/// Clicking a key invokes `on_select` with the candidate's code.
///
/// No scroll area of its own: embed inside the pane that owns scrolling.
pub fn picker_grid_rows(
    ui: &mut egui::Ui,
    id_salt: &str,
    candidates: &[Candidate],
    selected: Option<u32>,
    style: &KeyPaintStyle,
    mut on_select: impl FnMut(u32),
) {
    let cols = ((ui.available_width() + GAP) / (KEY_UNIT + GAP))
        .floor()
        .max(1.0) as usize;
    let rows = candidates.len().div_ceil(cols);
    let grid_width =
        (cols as f32 * KEY_UNIT + (cols.saturating_sub(1)) as f32 * GAP).min(ui.available_width());
    let total_height = rows as f32 * KEY_UNIT + (rows.saturating_sub(1)) as f32 * GAP;

    let (_, space_rect) = ui.allocate_exact_size(
        egui::vec2(grid_width.max(KEY_UNIT), total_height),
        egui::Sense::hover(),
    );
    let origin = space_rect.rect.min;

    for (i, candidate) in candidates.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            origin
                + egui::vec2(
                    (i % cols) as f32 * (KEY_UNIT + GAP),
                    (i / cols) as f32 * (KEY_UNIT + GAP),
                ),
            egui::vec2(KEY_UNIT, KEY_UNIT),
        );
        let response = ui.interact(
            cell,
            ui.id().with((id_salt, "cell", i, candidate.code)),
            egui::Sense::click(),
        );

        // Selected uses the overlay's pressed treatment; transparent bindings
        // render ghosted, like unset slots on the overlay.
        let pressed = selected == Some(candidate.code);
        let colors = style
            .colors_for(0, candidate.key.kind, false, pressed)
            .ghosted_if(candidate.transparent);

        key_paint::paint(
            ui,
            cell,
            0.0,
            &KeyDisplay {
                key: &candidate.key,
                colors,
                hovered: response.hovered(),
                pressed,
                shift_held: false,
                ralt_held: false,
            },
            style,
        );

        if response.clicked() {
            on_select(candidate.code);
        }
    }
}
