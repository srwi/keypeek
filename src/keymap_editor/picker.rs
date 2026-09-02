//! Shared key picker controls. Draws candidate keys and modifier selectors using [`crate::key_paint`].

use crate::key_action::KeyAction;
use crate::key_paint::{self, KeyDisplay, KeyPaintStyle};
use crate::layout_key::{modifier_symbols, KeycodeKind, Label, LayoutKey};

/// Key unit size in picker grids in pixels.
pub const KEY_UNIT: f32 = 51.0;
/// Space between key cells in pixels.
const GAP: f32 = 6.0;

/// Candidate key binding displayed in a picker grid.
#[derive(Clone)]
pub struct Candidate {
    /// Firmware key binding.
    pub binding: KeyAction,
    /// Visual key representation.
    pub key: LayoutKey,
    /// Indicates a transparent key slot.
    pub transparent: bool,
}

impl Candidate {
    pub fn new(binding: KeyAction, key: LayoutKey) -> Self {
        Self {
            binding,
            key,
            transparent: false,
        }
    }

    /// Checks whether this candidate matches the search query.
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }

        let matches = |s: &str| s.to_lowercase().contains(&q);
        let matches_opt = |opt: &Option<String>| opt.as_deref().is_some_and(matches);
        let matches_lbl = |lbl: &Option<crate::layout_key::Label>| {
            lbl.as_ref()
                .is_some_and(|l| matches(&l.full) || matches_opt(&l.short))
        };

        if matches(&self.key.tap.full)
            || matches_opt(&self.key.tap.short)
            || matches_opt(&self.key.shifted)
            || matches_opt(&self.key.ralt)
            || matches_opt(&self.key.ralt_shifted)
            || self.key.tooltip_text().as_deref().is_some_and(matches)
            || matches_lbl(&self.key.behavior)
            || matches_lbl(&self.key.argument)
        {
            return true;
        }

        match &self.binding {
            KeyAction::Qmk(code) => {
                if let Ok(kc) = qmk_via_api::keycodes::Keycode::try_from(*code) {
                    if matches(kc.as_ref()) {
                        return true;
                    }
                }
                format!("{:04x}", code).contains(&q)
            }
            KeyAction::Zmk(behavior) => match behavior {
                zmk_studio_api::Behavior::KeyPress(usage)
                | zmk_studio_api::Behavior::KeyToggle(usage)
                | zmk_studio_api::Behavior::StickyKey(usage) => {
                    if let Ok(kc) = zmk_studio_api::Keycode::try_from(usage.to_hid_usage()) {
                        if matches(kc.as_ref()) || matches(kc.to_name()) {
                            return true;
                        }
                    }
                    format!("{:02x}", usage.id()).contains(&q)
                }
                _ => false,
            },
        }
    }

    /// Creates a candidate for any key action, providing consistent display
    /// labels for transparent and none slots across protocols.
    pub fn from_action(binding: KeyAction, layer_names: &[String]) -> Self {
        let is_none = match &binding {
            KeyAction::Qmk(code) => *code == qmk_via_api::keycodes::Keycode::KC_NO as u16,
            KeyAction::Zmk(b) => *b == zmk_studio_api::Behavior::None,
        };
        if is_none {
            return Self::new(
                binding,
                LayoutKey {
                    tap: Label::new("None"),
                    ..Default::default()
                },
            );
        }

        match binding.resolve_label(layer_names) {
            None => Self {
                binding,
                key: LayoutKey {
                    tap: Label::with_short("Transparent", egui_phosphor::regular::CARET_DOWN),
                    ..Default::default()
                },
                transparent: true,
            },
            Some(key) => Self::new(binding, key),
        }
    }
}

/// Selected key state and validity indicator for picker grids.
#[derive(Clone, Copy)]
pub struct SelectedKey<'a> {
    pub action: &'a KeyAction,
    pub valid: bool,
}

impl<'a> SelectedKey<'a> {
    pub fn valid(action: &'a KeyAction) -> Self {
        Self {
            action,
            valid: true,
        }
    }

    pub fn new(action: &'a KeyAction, valid: bool) -> Self {
        Self { action, valid }
    }
}

impl<'a> From<&'a KeyAction> for SelectedKey<'a> {
    fn from(action: &'a KeyAction) -> Self {
        Self::valid(action)
    }
}

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
    response
}

/// Returns the total width spanned by the columns of keys in the picker grid for the given available width.
pub fn picker_grid_width(available_width: f32) -> f32 {
    let cols = ((available_width + GAP) / (KEY_UNIT + GAP))
        .floor()
        .max(1.0) as usize;
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
    let cols = ((ui.available_width() + GAP) / (KEY_UNIT + GAP))
        .floor()
        .max(1.0) as usize;
    let rows = candidates.len().div_ceil(cols);
    let grid_width = picker_grid_width(ui.available_width());
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

/// Named group of candidate keys.
pub struct CandidateGroup {
    pub name: &'static str,
    pub candidates: Vec<Candidate>,
}

/// Filters candidate references matching a predicate and search query.
fn filter_candidates<'a>(
    candidates: &'a [Candidate],
    query: &str,
    filter: impl Fn(&Candidate) -> bool,
) -> Vec<&'a Candidate> {
    candidates
        .iter()
        .filter(|c| filter(c) && c.matches_query(query))
        .collect()
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
    let mut rendered_any = false;
    for (gi, group) in groups.iter().enumerate() {
        let refs = filter_candidates(&group.candidates, search_query, &filter);
        if refs.is_empty() {
            continue;
        }
        rendered_any = true;
        draw_group(ui, gi, group.name, &refs);
    }

    if !rendered_any && !search_query.trim().is_empty() {
        ui.weak("No matching keys");
    }
}

/// Draws a titled group containing candidate keys, filtering by the search query.
pub fn titled_candidate_group(
    ui: &mut egui::Ui,
    group: &CandidateGroup,
    search_query: &str,
    filter: impl Fn(&Candidate) -> bool,
    selected: Option<SelectedKey<'_>>,
    style: &KeyPaintStyle,
    on_select: impl FnMut(&Candidate),
) {
    crate::ui_widgets::titled_group(ui, group.name, |ui| {
        let refs = filter_candidates(&group.candidates, search_query, &filter);
        if refs.is_empty() && !search_query.trim().is_empty() {
            ui.weak("No matching keys");
        } else {
            picker_grid_refs(ui, group.name, &refs, selected, style, on_select);
        }
    });
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

/// Hand variant for a modifier key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
}

impl Hand {
    fn tag(self) -> Label {
        match self {
            Hand::Left => Label::with_short("Left", "L"),
            Hand::Right => Label::with_short("Right", "R"),
        }
    }
}

/// Creates a modifier key definition with an optional hand label.
fn modifier_chip_key(name: &modifier_symbols::ModName, hand: Option<Hand>) -> LayoutKey {
    let mut key = modifier_symbols::modifier_key(name, 0);
    key.argument = hand.map(Hand::tag);
    key
}

/// The four QMK modifier types as key-shaped toggle chips, sharing the
/// overlay's modifier look, alongside a vertically centered Hand (L/R) selector.
/// Selected bits use the pressed treatment, matching how the selected cell is
/// highlighted in the picker grids. Returns `true` if any modifier bit or hand
/// selection changed.
pub fn modifier_toggle_row(
    ui: &mut egui::Ui,
    id_salt: &str,
    mods: &mut u16,
    right: &mut bool,
    valid: bool,
    style: &KeyPaintStyle,
) -> bool {
    use modifier_symbols::{MOD_ALT, MOD_CTRL, MOD_GUI, MOD_SHIFT};

    let hand = if *right { Hand::Right } else { Hand::Left };
    let defs = [
        (0x01, &MOD_CTRL),
        (0x02, &MOD_SHIFT),
        (0x04, &MOD_ALT),
        (0x08, &MOD_GUI),
    ];

    let mut changed = false;

    ui.horizontal(|ui| {
        let cells = defs.len() as f32;
        let row_width = cells * KEY_UNIT + (cells - 1.0) * GAP;
        let (_, space_rect) = ui.allocate_space(egui::vec2(row_width, KEY_UNIT));
        let origin = space_rect.min;

        for (i, (mask, name)) in defs.iter().enumerate() {
            let cell = egui::Rect::from_min_size(
                origin + egui::vec2(i as f32 * (KEY_UNIT + GAP), 0.0),
                egui::vec2(KEY_UNIT, KEY_UNIT),
            );
            let key = modifier_chip_key(name, Some(hand));
            let response = key_chip(
                ui,
                cell,
                ui.id().with((id_salt, "mod", i)),
                &key,
                *mods & mask != 0,
                valid,
                style,
            );

            if response.clicked() {
                *mods ^= *mask;
                changed = true;
            }
        }

        ui.add_space(8.0);
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.weak("Hand");
            if ui
                .add(egui::Button::new("L").small().selected(!*right))
                .clicked()
            {
                *right = false;
                changed = true;
            }
            if ui
                .add(egui::Button::new("R").small().selected(*right))
                .on_hover_text("Right-hand modifiers (RCTL, RSFT, RALT, RGUI)")
                .clicked()
            {
                *right = true;
                changed = true;
            }
        });
    });

    changed
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
    use modifier_symbols::{MOD_ALT, MOD_CTRL, MOD_GUI, MOD_SHIFT};

    let names = [&MOD_CTRL, &MOD_SHIFT, &MOD_ALT, &MOD_GUI];
    let cells = names.len() as f32;
    let row_width = cells * KEY_UNIT + (cells - 1.0) * GAP;
    let total_height = 2.0 * KEY_UNIT + GAP;

    let (_, space_rect) = ui.allocate_space(egui::vec2(row_width, total_height));
    let origin = space_rect.min;

    for (row, hand) in [(0u8, Hand::Left), (1, Hand::Right)] {
        for (i, name) in names.iter().enumerate() {
            let mask = 1 << (row * 4 + i as u8);
            let cell = egui::Rect::from_min_size(
                origin + egui::vec2(i as f32 * (KEY_UNIT + GAP), row as f32 * (KEY_UNIT + GAP)),
                egui::vec2(KEY_UNIT, KEY_UNIT),
            );
            let key = modifier_chip_key(name, Some(hand));
            let response = key_chip(
                ui,
                cell,
                ui.id().with((id_salt, "mod", mask)),
                &key,
                mods & mask != 0,
                valid,
                style,
            );
            if response.clicked() {
                on_toggle(mask);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qmk_via_api::keycodes::Keycode;

    #[test]
    fn candidate_matches_query_by_label_and_short() {
        let candidate = Candidate::from_action(KeyAction::Qmk(Keycode::KC_ESCAPE as u16), &[]);
        assert!(candidate.matches_query(""));
        assert!(candidate.matches_query("esc"));
        assert!(candidate.matches_query("ESC"));
        assert!(candidate.matches_query("escape"));
        assert!(!candidate.matches_query("enter"));

        let enter = Candidate::from_action(KeyAction::Qmk(Keycode::KC_ENTER as u16), &[]);
        assert!(enter.matches_query("enter"));
        assert!(enter.matches_query("ENT"));
        assert!(!enter.matches_query("space"));
    }

    #[test]
    fn candidate_matches_query_by_shifted_and_symbol() {
        let digit_1 = Candidate::from_action(KeyAction::Qmk(Keycode::KC_1 as u16), &[]);
        assert!(digit_1.matches_query("1"));
        assert!(digit_1.matches_query("!"));

        let mute = Candidate::from_action(KeyAction::Qmk(Keycode::KC_AUDIO_MUTE as u16), &[]);
        assert!(mute.matches_query("mute"));
        assert!(mute.matches_query("audio"));
    }

    #[test]
    fn candidate_matches_zmk_behavior() {
        let space = Candidate::from_action(
            KeyAction::Zmk(zmk_studio_api::Behavior::KeyPress(
                zmk_studio_api::HidUsage::from_parts(zmk_studio_api::HID_USAGE_KEYBOARD, 0x2C, 0),
            )),
            &[],
        );
        assert!(space.matches_query("space"));
        assert!(space.matches_query("spc"));

        let play = Candidate::from_action(
            KeyAction::Zmk(zmk_studio_api::Behavior::KeyPress(
                zmk_studio_api::HidUsage::from_encoded(
                    zmk_studio_api::Keycode::C_PLAY.to_hid_usage(),
                ),
            )),
            &[],
        );
        assert!(play.matches_query("play"));
    }

    #[test]
    fn candidate_matches_hex_and_whitespace_query() {
        let a_key = Candidate::from_action(KeyAction::Qmk(Keycode::KC_A as u16), &[]);
        assert!(a_key.matches_query("  "));
        assert!(a_key.matches_query("0004"));
        assert!(!a_key.matches_query("9999"));
    }

    #[test]
    fn filter_candidates_filters_by_query() {
        let candidates = vec![
            Candidate::from_action(KeyAction::Qmk(Keycode::KC_ESCAPE as u16), &[]),
            Candidate::from_action(KeyAction::Qmk(Keycode::KC_ENTER as u16), &[]),
            Candidate::from_action(KeyAction::Qmk(Keycode::KC_SPACE as u16), &[]),
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
}
