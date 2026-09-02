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
    /// Precomputed lowercase search tokens.
    search_haystack: String,
}

impl Candidate {
    pub fn new(binding: KeyAction, key: LayoutKey) -> Self {
        let search_haystack = build_search_haystack(&binding, &key);
        Self {
            binding,
            key,
            transparent: false,
            search_haystack,
        }
    }

    /// Sets whether this candidate represents a transparent key slot.
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Checks whether this candidate matches the search query.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        self.matches_lowercased(&q)
    }

    /// Fast search check against pre-lowercased query token without allocations.
    pub fn matches_lowercased(&self, lowercase_query: &str) -> bool {
        self.search_haystack.contains(lowercase_query)
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
            None => Self::new(
                binding,
                LayoutKey {
                    tap: Label::with_short("Transparent", egui_phosphor::regular::CARET_DOWN),
                    ..Default::default()
                },
            )
            .with_transparent(true),
            Some(key) => Self::new(binding, key),
        }
    }
}

fn push_token(haystack: &mut String, s: &str) {
    for c in s.chars().flat_map(char::to_lowercase) {
        haystack.push(c);
    }
    haystack.push(' ');
}

fn push_opt_token(haystack: &mut String, opt: &Option<String>) {
    if let Some(s) = opt {
        push_token(haystack, s);
    }
}

fn push_lbl_token(haystack: &mut String, lbl: &Option<crate::layout_key::Label>) {
    if let Some(l) = lbl {
        push_token(haystack, &l.full);
        push_opt_token(haystack, &l.short);
    }
}

fn build_search_haystack(binding: &KeyAction, key: &LayoutKey) -> String {
    let mut haystack = String::with_capacity(64);

    push_token(&mut haystack, &key.tap.full);
    push_opt_token(&mut haystack, &key.tap.short);
    push_opt_token(&mut haystack, &key.shifted);
    push_opt_token(&mut haystack, &key.ralt);
    push_opt_token(&mut haystack, &key.ralt_shifted);
    push_opt_token(&mut haystack, &key.tooltip_text());
    push_lbl_token(&mut haystack, &key.behavior);
    push_lbl_token(&mut haystack, &key.argument);

    match binding {
        KeyAction::Qmk(code) => {
            if let Ok(kc) = qmk_via_api::keycodes::Keycode::try_from(*code) {
                push_token(&mut haystack, kc.as_ref());
            }
            use std::fmt::Write;
            let _ = write!(&mut haystack, "{:04x} ", code);
        }
        KeyAction::Zmk(behavior) => match behavior {
            zmk_studio_api::Behavior::KeyPress(usage)
            | zmk_studio_api::Behavior::KeyToggle(usage)
            | zmk_studio_api::Behavior::StickyKey(usage) => {
                if let Ok(kc) = zmk_studio_api::Keycode::try_from(usage.to_hid_usage()) {
                    push_token(&mut haystack, kc.as_ref());
                    push_token(&mut haystack, kc.to_name());
                }
                use std::fmt::Write;
                let _ = write!(&mut haystack, "{:02x} ", usage.id());
            }
            _ => {}
        },
    }

    haystack
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

/// Named group of candidate keys.
pub struct CandidateGroup {
    pub name: &'static str,
    pub candidates: Vec<Candidate>,
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
    let q = search_query.trim().to_lowercase();
    let refs = filter_candidates_lowercased(&group.candidates, &q, &filter);
    crate::ui_widgets::titled_group(ui, group.name, |ui| {
        if refs.is_empty() && !q.is_empty() {
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

/// Draws a compact hand (L/R) selector chip fitting within a single key unit.
fn hand_selector(ui: &mut egui::Ui, rect: egui::Rect, id_salt: &str, right: &mut bool) -> bool {
    let mut changed = false;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .id_salt((id_salt, "hand")),
        |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 2.0);
            ui.vertical_centered(|ui| {
                ui.weak("Hand");
                ui.horizontal(|ui| {
                    let btn_size = egui::vec2((rect.width() - 3.0) / 2.0, ui.available_height());
                    for (is_r, label, tip) in [
                        (false, "L", "Left-hand modifiers (LCTL, LSFT, LALT, LGUI)"),
                        (true, "R", "Right-hand modifiers (RCTL, RSFT, RALT, RGUI)"),
                    ] {
                        if ui
                            .add_sized(btn_size, egui::Button::new(label).selected(*right == is_r))
                            .on_hover_text(tip)
                            .clicked()
                            && *right != is_r
                        {
                            *right = is_r;
                            changed = true;
                        }
                    }
                });
            });
        },
    );
    changed
}

/// Draws a 4-modifier toggle row alongside a Hand (L/R) selector.
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

    let total_cells = 5.0;
    let row_width = total_cells * KEY_UNIT + (total_cells - 1.0) * GAP;
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

    let hand_rect = egui::Rect::from_min_size(
        origin + egui::vec2(defs.len() as f32 * (KEY_UNIT + GAP), 0.0),
        egui::vec2(KEY_UNIT, KEY_UNIT),
    );
    if hand_selector(ui, hand_rect, id_salt, right) {
        changed = true;
    }

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

    #[test]
    fn modifier_toggle_row_fits_five_keys_width() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            let mut mods = 0u16;
            let mut right = false;
            let style = KeyPaintStyle::from_settings(&crate::settings::Settings::default());
            let inner_response = ui.allocate_ui(egui::vec2(0.0, 0.0), |ui| {
                modifier_toggle_row(ui, "test", &mut mods, &mut right, true, &style);
            });
            let expected_width = 5.0 * KEY_UNIT + 4.0 * GAP;
            assert_eq!(expected_width, 279.0);
            assert_eq!(inner_response.response.rect.width(), expected_width);
            assert_eq!(inner_response.response.rect.height(), KEY_UNIT);
        });
    }

    #[test]
    fn modifier_toggle_row_toggle_hand() {
        let ctx = egui::Context::default();
        let style = KeyPaintStyle::from_settings(&crate::settings::Settings::default());

        let mut mods = 0u16;
        let mut right = false;

        let mut r_pos = egui::Pos2::ZERO;
        let _ = ctx.run_ui(Default::default(), |ui| {
            let inner_response = ui.allocate_ui(egui::vec2(0.0, 0.0), |ui| {
                modifier_toggle_row(ui, "test", &mut mods, &mut right, true, &style);
            });
            let origin = inner_response.response.rect.min;
            let btn_gap = 3.0;
            let btn_width = (KEY_UNIT - btn_gap) / 2.0;
            let label_height = 14.0;
            let gap_y = 2.0;
            r_pos = origin
                + egui::vec2(
                    4.0 * (KEY_UNIT + GAP) + btn_width + btn_gap + btn_width / 2.0,
                    label_height + gap_y + (KEY_UNIT - label_height - gap_y) / 2.0,
                );
        });

        let mut raw_input = egui::RawInput::default();
        raw_input.events.push(egui::Event::PointerMoved(r_pos));
        raw_input.events.push(egui::Event::PointerButton {
            pos: r_pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        });
        raw_input.events.push(egui::Event::PointerButton {
            pos: r_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        });

        let mut changed = false;
        let _ = ctx.run_ui(raw_input, |ui| {
            ui.allocate_ui(egui::vec2(0.0, 0.0), |ui| {
                changed = modifier_toggle_row(ui, "test", &mut mods, &mut right, true, &style);
            });
        });

        assert!(right);
        assert!(changed);
    }
}
