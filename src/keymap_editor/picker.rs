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

/// Draws a grid of key candidates from references.
pub fn picker_grid_refs(
    ui: &mut egui::Ui,
    id_salt: &str,
    candidates: &[&Candidate],
    selected: Option<&KeyAction>,
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
    let grid_width =
        (cols as f32 * KEY_UNIT + (cols.saturating_sub(1)) as f32 * GAP).min(ui.available_width());
    let total_height = rows as f32 * KEY_UNIT + (rows.saturating_sub(1)) as f32 * GAP;

    let (_, space_rect) =
        ui.allocate_space(egui::vec2(grid_width.max(KEY_UNIT), total_height));
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

        let pressed = selected == Some(&candidate.binding);
        let colors = style
            .colors_for(
                candidate.key.layer_ref.unwrap_or(0),
                candidate.key.kind,
                false,
                pressed,
            )
            .ghosted_if(candidate.transparent);

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

/// Draws a grid of key candidates.
pub fn picker_grid_rows(
    ui: &mut egui::Ui,
    id_salt: &str,
    candidates: &[Candidate],
    selected: Option<&KeyAction>,
    style: &KeyPaintStyle,
    on_select: impl FnMut(&Candidate),
) {
    let refs: Vec<&Candidate> = candidates.iter().collect();
    picker_grid_refs(ui, id_salt, &refs, selected, style, on_select);
}

/// Draws a grid of key candidates filtered by a predicate.
pub fn picker_grid_filtered(
    ui: &mut egui::Ui,
    id_salt: &str,
    candidates: &[Candidate],
    filter: impl Fn(&Candidate) -> bool,
    selected: Option<&KeyAction>,
    style: &KeyPaintStyle,
    on_select: impl FnMut(&Candidate),
) {
    let refs: Vec<&Candidate> = candidates.iter().filter(|c| filter(c)).collect();
    picker_grid_refs(ui, id_salt, &refs, selected, style, on_select);
}

/// Named group of candidate keys.
pub struct CandidateGroup {
    pub name: &'static str,
    pub candidates: Vec<Candidate>,
}

/// Draws labeled groups of candidate keys.
pub fn candidate_groups_rows(
    ui: &mut egui::Ui,
    groups: &[CandidateGroup],
    filter: impl Fn(&Candidate) -> bool,
    selected: impl Fn(usize) -> Option<KeyAction>,
    style: &KeyPaintStyle,
    mut on_select: impl FnMut(usize, &Candidate),
) {
    for (gi, group) in groups.iter().enumerate() {
        let refs: Vec<&Candidate> = group.candidates.iter().filter(|c| filter(c)).collect();
        if refs.is_empty() {
            continue;
        }
        ui.label(group.name);
        picker_grid_refs(
            ui,
            group.name,
            &refs,
            selected(gi).as_ref(),
            style,
            |candidate| on_select(gi, candidate),
        );
        ui.add_space(6.0);
    }
}

/// Draws candidate groups in framed group boxes.
pub fn framed_candidate_groups_rows(
    ui: &mut egui::Ui,
    groups: &[CandidateGroup],
    selected: impl Fn(usize) -> Option<KeyAction>,
    style: &KeyPaintStyle,
    mut on_select: impl FnMut(usize, &Candidate),
) {
    for (gi, group) in groups.iter().enumerate() {
        crate::ui_widgets::titled_group(ui, group.name, |ui| {
            picker_grid_rows(
                ui,
                group.name,
                &group.candidates,
                selected(gi).as_ref(),
                style,
                |candidate| on_select(gi, candidate),
            );
        });
    }
}

/// Draws a modifier key button.
fn key_chip(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    id: egui::Id,
    key: &LayoutKey,
    selected: bool,
    style: &KeyPaintStyle,
) -> egui::Response {
    let colors = style.colors_for(0, KeycodeKind::Modifier, false, selected);
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
            if ui.add(egui::Button::new("L").small().selected(!*right)).clicked() {
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
                origin
                    + egui::vec2(
                        i as f32 * (KEY_UNIT + GAP),
                        row as f32 * (KEY_UNIT + GAP),
                    ),
                egui::vec2(KEY_UNIT, KEY_UNIT),
            );
            let key = modifier_chip_key(name, Some(hand));
            let response = key_chip(
                ui,
                cell,
                ui.id().with((id_salt, "mod", mask)),
                &key,
                mods & mask != 0,
                style,
            );
            if response.clicked() {
                on_toggle(mask);
            }
        }
    }
}
