//! Shared key picker controls. Draws candidate keys and modifier selectors using [`crate::key_paint`].

use crate::key_paint::{self, KeyDisplay, KeyPaintStyle};
use keypeek_core::{modifier_symbols, KeycodeKind, Label, LayoutKey};

/// Key unit size in picker grids in pixels.
pub const KEY_UNIT: f32 = 51.0;
/// Space between key cells in pixels.
const GAP: f32 = 6.0;

pub use keypeek_core::keymap_editor::candidate::{Candidate, CandidateGroup, SelectedKey};

/// Draws an interactive key button.
pub fn key_button(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: egui::Id,
    key: &LayoutKey,
    colors: crate::key_paint::KeyColors,
    pressed: bool,
    style: &KeyPaintStyle,
) -> egui::Response {
    let mut response = ui.interact(rect, id, egui::Sense::click());
    if let Some(tooltip) = key.tooltip_text() {
        response = response.on_hover_text_at_pointer(tooltip);
    }
    if ui.is_rect_visible(rect) {
        key_paint::paint(
            ui,
            rect,
            0.0,
            &KeyDisplay {
                key,
                colors,
                hovered: response.hovered(),
                pressed,
                shift_held: false,
                ralt_held: false,
            },
            style,
        );
    }
    response
}

/// Returns the number of columns of keys that fit in the given available width.
pub fn picker_grid_cols(available_width: f32) -> usize {
    ((available_width + GAP) / (KEY_UNIT + GAP))
        .floor()
        .max(1.0) as usize
}

/// Returns the total width spanned by the columns of keys in the picker grid for the given available width.
pub fn picker_grid_width(available_width: f32) -> f32 {
    let cols = picker_grid_cols(available_width);
    (cols as f32 * KEY_UNIT + (cols.saturating_sub(1)) as f32 * GAP).min(available_width)
}

/// Draws a grid of key candidates from references.
pub fn picker_grid_refs(
    ui: &mut egui::Ui,
    id_salt: &str,
    candidates: &[&Candidate],
    selected: Option<SelectedKey<'_>>,
    style: &KeyPaintStyle,
    mut on_select: impl FnMut(&Candidate),
) {
    if candidates.is_empty() {
        return;
    }
    let available_width = ui.available_width();
    let cols = picker_grid_cols(available_width);
    let rows = candidates.len().div_ceil(cols);
    let grid_width = picker_grid_width(available_width);
    let total_height = rows as f32 * KEY_UNIT + (rows.saturating_sub(1)) as f32 * GAP;

    let (_, space_rect) = ui.allocate_space(egui::vec2(grid_width.max(KEY_UNIT), total_height));
    let origin = space_rect.min;

    for (i, candidate) in candidates.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            origin
                + egui::vec2(
                    (i % cols) as f32 * (KEY_UNIT + GAP),
                    (i / cols) as f32 * (KEY_UNIT + GAP),
                ),
            egui::vec2(KEY_UNIT, KEY_UNIT),
        );

        if !ui.is_rect_visible(cell) {
            continue;
        }

        let pressed = selected.is_some_and(|s| s.action == &candidate.binding);
        let is_valid = selected.is_none_or(|s| s.valid);
        let mut colors = style
            .colors_for(
                candidate.key.layer_ref.unwrap_or(0),
                candidate.key.kind,
                false,
                pressed,
            )
            .ghosted_if(candidate.transparent);

        if pressed && !is_valid {
            colors = colors.with_invalid_selection();
        }

        let response = key_button(
            ui,
            cell,
            ui.id().with((id_salt, "cell", i)),
            &candidate.key,
            colors,
            pressed,
            style,
        );

        if response.clicked() {
            on_select(candidate);
        }
    }
}

/// Filters candidate references matching a predicate and pre-lowercased search query.
fn filter_candidates_lowercased<'a>(
    candidates: &'a [Candidate],
    lowercased_query: &str,
    filter: impl Fn(&Candidate) -> bool,
) -> Vec<&'a Candidate> {
    if lowercased_query.is_empty() {
        candidates.iter().filter(|c| filter(c)).collect()
    } else {
        candidates
            .iter()
            .filter(|c| filter(c) && c.matches_lowercased(lowercased_query))
            .collect()
    }
}

/// Filters candidate references matching a predicate and search query.
#[cfg(test)]
fn filter_candidates<'a>(
    candidates: &'a [Candidate],
    query: &str,
    filter: impl Fn(&Candidate) -> bool,
) -> Vec<&'a Candidate> {
    filter_candidates_lowercased(candidates, &query.trim().to_lowercase(), filter)
}

/// Renders filtered candidate groups, handling the global empty-query state and delegating
/// presentation of each non-empty group to `draw_group`.
fn render_candidate_groups(
    ui: &mut egui::Ui,
    groups: &[CandidateGroup],
    search_query: &str,
    filter: impl Fn(&Candidate) -> bool,
    mut draw_group: impl FnMut(&mut egui::Ui, usize, &'static str, &[&Candidate]),
) {
    let q = search_query.trim().to_lowercase();
    let mut rendered_any = false;
    for (gi, group) in groups.iter().enumerate() {
        let refs = filter_candidates_lowercased(&group.candidates, &q, &filter);
        if refs.is_empty() {
            continue;
        }
        rendered_any = true;
        draw_group(ui, gi, group.name, &refs);
    }

    if !rendered_any && !q.is_empty() {
        ui.weak("No matching keys");
    }
}

/// Draws multiple candidate groups inside an existing UI container,
/// filtering all groups by the search query.
pub fn multi_candidate_groups(
    ui: &mut egui::Ui,
    groups: &[CandidateGroup],
    search_query: &str,
    filter: impl Fn(&Candidate) -> bool,
    selected: Option<SelectedKey<'_>>,
    style: &KeyPaintStyle,
    mut on_select: impl FnMut(usize, &Candidate),
) {
    render_candidate_groups(ui, groups, search_query, filter, |ui, gi, name, refs| {
        ui.push_id((gi, name), |ui| {
            ui.label(name);
            picker_grid_refs(ui, name, refs, selected, style, |c| on_select(gi, c));
            ui.add_space(6.0);
        });
    });
}

/// Draws a search input with width matching the left pane.
pub fn search_bar(ui: &mut egui::Ui, query: &mut String) -> egui::Response {
    let width = ui.available_width();
    let response = ui.add_sized(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::TextEdit::singleline(query).hint_text("Search keys..."),
    );
    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        query.clear();
    }
    response
}

/// Draws candidate groups in framed group boxes, filtering all groups by the search query.
pub fn framed_candidate_groups(
    ui: &mut egui::Ui,
    groups: &[CandidateGroup],
    search_query: &str,
    filter: impl Fn(&Candidate) -> bool,
    selected: Option<SelectedKey<'_>>,
    style: &KeyPaintStyle,
    mut on_select: impl FnMut(usize, &Candidate),
) {
    render_candidate_groups(ui, groups, search_query, filter, |ui, gi, name, refs| {
        crate::ui_widgets::titled_group(ui, name, |ui| {
            picker_grid_refs(ui, name, refs, selected, style, |candidate| {
                on_select(gi, candidate)
            });
        });
    });
}

/// Draws a modifier key button.
fn key_chip(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    id: egui::Id,
    key: &LayoutKey,
    selected: bool,
    valid: bool,
    style: &KeyPaintStyle,
) -> egui::Response {
    let mut colors = style.colors_for(0, KeycodeKind::Modifier, false, selected);
    if selected && !valid {
        colors = colors.with_invalid_selection();
    }
    key_button(ui, cell, id, key, colors, selected, style)
}

/// Precomputed definitions for the 8 standard modifier keys (4 Left, 4 Right).
fn modifier_chip_keys() -> &'static [LayoutKey; 8] {
    static CHIPS: std::sync::OnceLock<[LayoutKey; 8]> = std::sync::OnceLock::new();
    CHIPS.get_or_init(|| {
        use modifier_symbols::{MOD_ALT, MOD_CTRL, MOD_GUI, MOD_SHIFT};
        let mods = [&MOD_CTRL, &MOD_SHIFT, &MOD_ALT, &MOD_GUI];
        std::array::from_fn(|i| {
            let mut key = modifier_symbols::modifier_key(mods[i % 4], 0);
            key.argument = Some(if i < 4 {
                Label::with_short("Left", "L")
            } else {
                Label::with_short("Right", "R")
            });
            key
        })
    })
}

/// Draws an 8-key modifier toggle grid (4 Left, 4 Right).
pub fn modifier_toggle_grid(
    ui: &mut egui::Ui,
    id_salt: &str,
    mods: u8,
    valid: bool,
    style: &KeyPaintStyle,
    mut on_toggle: impl FnMut(u8),
) {
    let row_width = 4.0 * KEY_UNIT + 3.0 * GAP;
    let total_height = 2.0 * KEY_UNIT + GAP;
    let (_, space_rect) = ui.allocate_space(egui::vec2(row_width, total_height));

    for (idx, chip) in modifier_chip_keys().iter().enumerate() {
        let col = (idx % 4) as f32;
        let row = (idx / 4) as f32;
        let mask = 1 << idx;
        let cell = egui::Rect::from_min_size(
            space_rect.min + egui::vec2(col * (KEY_UNIT + GAP), row * (KEY_UNIT + GAP)),
            egui::vec2(KEY_UNIT, KEY_UNIT),
        );
        let response = key_chip(
            ui,
            cell,
            ui.id().with((id_salt, "mod", mask)),
            chip,
            mods & mask != 0,
            valid,
            style,
        );
        if response.clicked() {
            on_toggle(mask);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keypeek_core::{HidKey, KeySpec, Modifiers};
    use keypeek_protocol::key_presenter::StandardKeyPresenter;

    fn test_candidate(binding: KeySpec) -> Candidate {
        Candidate::from_action(binding, &StandardKeyPresenter, &[])
    }

    #[test]
    fn candidate_matches_query() {
        let space = test_candidate(KeySpec::KeyPress {
            key: HidKey::keyboard(0x2c),
            modifiers: Modifiers::default(),
        })
        .with_search_token("custom_token");

        assert!(space.matches_query(""));
        assert!(space.matches_query("space"));
        assert!(space.matches_query("SPACE"));
        assert!(space.matches_query("custom_token"));
        assert!(!space.matches_query("enter"));

        let digit_1 = test_candidate(KeySpec::KeyPress {
            key: HidKey::keyboard(0x1e),
            modifiers: Modifiers::default(),
        });
        assert!(digit_1.matches_query("1"));
        assert!(digit_1.matches_query("!"));

        let enter = test_candidate(KeySpec::KeyPress {
            key: HidKey::keyboard(0x28),
            modifiers: Modifiers::default(),
        });
        assert!(enter.matches_query("enter"));
        assert!(enter.matches_query(egui_phosphor::regular::ARROW_ELBOW_DOWN_LEFT));

        let a_key = test_candidate(KeySpec::KeyPress {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers::default(),
        });
        assert!(a_key.matches_query("  "));
        assert!(a_key.matches_query("0004"));
        assert!(!a_key.matches_query("9999"));
    }

    #[test]
    fn filter_candidates_filters_by_query() {
        let candidates = vec![
            test_candidate(KeySpec::KeyPress {
                key: HidKey::keyboard(0x29),
                modifiers: Modifiers::default(),
            }),
            test_candidate(KeySpec::KeyPress {
                key: HidKey::keyboard(0x28),
                modifiers: Modifiers::default(),
            }),
            test_candidate(KeySpec::KeyPress {
                key: HidKey::keyboard(0x2c),
                modifiers: Modifiers::default(),
            }),
        ];
        let empty_filter = filter_candidates(&candidates, "", |_| true);
        assert_eq!(empty_filter.len(), 3);

        let esc_filter = filter_candidates(&candidates, "esc", |_| true);
        assert_eq!(esc_filter.len(), 1);

        let none_filter = filter_candidates(&candidates, "zzzzz", |_| true);
        assert_eq!(none_filter.len(), 0);
    }

    #[test]
    fn picker_grid_width_matches_columns() {
        // 1 column: 51.0
        assert_eq!(picker_grid_width(55.0), 51.0);
        // 2 columns: 51.0 * 2 + 6.0 = 108.0
        assert_eq!(picker_grid_width(110.0), 108.0);
        // 10 columns: 51.0 * 10 + 6.0 * 9 = 564.0
        assert_eq!(picker_grid_width(580.0), 564.0);
    }

    #[test]
    fn modifier_toggle_grid_dimensions_and_toggle() {
        let ctx = egui::Context::default();
        let style = KeyPaintStyle::from_settings(&crate::settings::Settings::default());

        let expected_width = 4.0 * KEY_UNIT + 3.0 * GAP;
        let expected_height = 2.0 * KEY_UNIT + GAP;

        let mut toggled = None;
        let output = ctx.run_ui(Default::default(), |ui| {
            let inner_response = ui.allocate_ui(egui::vec2(0.0, 0.0), |ui| {
                modifier_toggle_grid(ui, "test", 0, true, &style, |mask| {
                    toggled = Some(mask);
                });
            });
            assert_eq!(inner_response.response.rect.width(), expected_width);
            assert_eq!(inner_response.response.rect.height(), expected_height);
        });
        output.drop_without_applying_deltas();
    }
}
