//! Key-shaped editor controls shared by the QMK and ZMK editors: a scrollable
//! grid of candidate keys and a row of modifier toggles. Everything is painted
//! by [`crate::key_paint`], so each key looks exactly like it will on the live
//! overlay.

use crate::key_action::KeyAction;
use crate::key_paint::{self, KeyDisplay, KeyPaintStyle};
use crate::layout_key::{modifier_symbols, KeycodeKind, Label, LayoutKey};

/// Pixels per key-unit in picker grids; a miniature overlay key.
pub const KEY_UNIT: f32 = 51.0;
/// Gap between grid cells, matching the old button-grid rhythm.
const GAP: f32 = 6.0;

/// One selectable candidate in a picker grid: the binding it stands for,
/// painted as the key it renders like on the overlay.
#[derive(Clone)]
pub struct Candidate {
    /// The firmware binding the candidate writes. Click-to-apply grids write
    /// it directly; staging pickers extract their parameter from it.
    pub binding: KeyAction,
    /// The fully resolved key: labels, symbol, and kind (for coloring).
    pub key: LayoutKey,
    /// Rendered dimmed like an unset overlay slot (QMK `KC_TRANSPARENT` / ZMK
    /// `Transparent`).
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
}

/// A scrollable grid of miniature keys, one per candidate. The currently
/// assigned `selected` binding is highlighted with the overlay's pressed look.
/// Clicking a key invokes `on_select` with the whole candidate.
///
/// No scroll area of its own: embed inside the pane that owns scrolling.
pub fn picker_grid_rows(
    ui: &mut egui::Ui,
    id_salt: &str,
    candidates: &[Candidate],
    selected: Option<&KeyAction>,
    style: &KeyPaintStyle,
    mut on_select: impl FnMut(&Candidate),
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
            ui.id().with((id_salt, "cell", i)),
            egui::Sense::click(),
        );

        // Selected uses the overlay's pressed treatment; transparent bindings
        // render ghosted, like unset slots on the overlay. Layer keys take
        // their target layer's fill, matching the overlay's coloring rule.
        let pressed = selected == Some(&candidate.binding);
        let colors = style
            .colors_for(
                candidate.key.layer_ref.unwrap_or(0),
                candidate.key.kind,
                false,
                pressed,
            )
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
            on_select(candidate);
        }
    }
}

/// A titled group of key candidates, rendered with a header like the usage
/// picker's category labels.
pub struct CandidateGroup {
    pub name: &'static str,
    pub candidates: Vec<Candidate>,
}

/// Titled groups of key grids, one after another — the layout shared by the
/// usage picker and both editors' layer pages. `selected(gi)` is the binding
/// highlighted within group `gi` (groups highlight differently, e.g. a staged
/// selection), and `on_select` receives the group index with the clicked
/// candidate.
pub fn candidate_groups_rows(
    ui: &mut egui::Ui,
    groups: &[CandidateGroup],
    selected: impl Fn(usize) -> Option<KeyAction>,
    style: &KeyPaintStyle,
    mut on_select: impl FnMut(usize, &Candidate),
) {
    for (gi, group) in groups.iter().enumerate() {
        ui.label(group.name);
        picker_grid_rows(
            ui,
            group.name,
            &group.candidates,
            selected(gi).as_ref(),
            style,
            |candidate| on_select(gi, candidate),
        );
        ui.add_space(6.0);
    }
}

/// [`candidate_groups_rows`] with every group framed in its own
/// [`crate::ui_widgets::titled_group`] outline — the layout for pages whose
/// groups are distinct keycode kinds (the layer pages and the direct-apply
/// command grids).
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

/// Paint one key-shaped chip and handle its click. `selected` uses the pressed
/// look, matching how the selected cell is highlighted in the picker grids.
fn key_chip(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    id: egui::Id,
    key: &LayoutKey,
    selected: bool,
    style: &KeyPaintStyle,
) -> egui::Response {
    let response = ui.interact(cell, id, egui::Sense::click());
    let colors = style.colors_for(0, KeycodeKind::Modifier, false, selected);
    key_paint::paint(
        ui,
        cell,
        0.0,
        &KeyDisplay {
            key,
            colors,
            hovered: response.hovered(),
            pressed: selected,
            shift_held: false,
            ralt_held: false,
        },
        style,
    );
    response
}

/// Which hand variant a modifier chip stands for. Rendered as a tag in the
/// chip's bottom argument strip rather than a separate row label, so chip rows
/// align with the pane's left edge.
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

/// A modifier chip key: the shared modifier glyph/name with an optional hand
/// tag in the bottom argument strip.
fn modifier_chip_key(name: &modifier_symbols::ModName, hand: Option<Hand>) -> LayoutKey {
    let mut key = modifier_symbols::modifier_key(name, 0);
    key.argument = hand.map(Hand::tag);
    key
}

/// The four QMK modifier types as key-shaped toggle chips, sharing the
/// overlay's modifier look: the same platform glyphs (`modifier_symbols`) and
/// the same darkened modifier colors. Selected bits use the pressed treatment,
/// matching how the selected cell is highlighted in the picker grids. Each
/// chip is tagged with the currently selected hand in its bottom strip, since
/// QMK's encoding applies one hand to the whole set.
///
/// `mods` is the low modifier nibble in HID bit order (Ctrl 0x01 … Gui 0x08).
/// `on_toggle` receives the clicked bit so the caller can flip it.
pub fn modifier_toggle_row(
    ui: &mut egui::Ui,
    id_salt: &str,
    mods: u16,
    hand: Hand,
    style: &KeyPaintStyle,
    mut on_toggle: impl FnMut(u16),
) {
    use modifier_symbols::{MOD_ALT, MOD_CTRL, MOD_GUI, MOD_SHIFT};

    let defs = [
        (0x01, &MOD_CTRL),
        (0x02, &MOD_SHIFT),
        (0x04, &MOD_ALT),
        (0x08, &MOD_GUI),
    ];

    // One allocated row rect with per-cell interact rects, like the grids.
    let cells = defs.len() as f32;
    let row_width = cells * KEY_UNIT + (cells - 1.0) * GAP;
    let (_, space_rect) =
        ui.allocate_exact_size(egui::vec2(row_width, KEY_UNIT), egui::Sense::hover());
    let origin = space_rect.rect.min;

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
            mods & mask != 0,
            style,
        );

        if response.clicked() {
            on_toggle(*mask);
        }
    }
}

/// The eight HID modifier bits as two key-shaped toggle chip rows (left hand,
/// right hand), freely combinable — unlike the single-hand [`modifier_toggle_row`],
/// this matches binding formats that store each modifier bit independently
/// (ZMK's usage modifier byte, LCTL 0x01 … RGUI 0x80). `mods` is that full
/// mask; `on_toggle` receives the clicked bit. Each chip is tagged with its
/// hand in the bottom argument strip, so no row labels are needed.
pub fn modifier_toggle_grid(
    ui: &mut egui::Ui,
    id_salt: &str,
    mods: u8,
    style: &KeyPaintStyle,
    mut on_toggle: impl FnMut(u8),
) {
    use modifier_symbols::{MOD_ALT, MOD_CTRL, MOD_GUI, MOD_SHIFT};

    let names = [&MOD_CTRL, &MOD_SHIFT, &MOD_ALT, &MOD_GUI];
    let full_names = ["Control", "Shift", "Alt", "GUI"];
    for (row, hand) in [(0u8, Hand::Left), (1, Hand::Right)] {
        ui.horizontal(|ui| {
            for (i, name) in names.iter().enumerate() {
                let mask = 1 << (row * 4 + i as u8);
                let key = modifier_chip_key(name, Some(hand));
                let (_, cell) =
                    ui.allocate_exact_size(egui::vec2(KEY_UNIT, KEY_UNIT), egui::Sense::hover());
                let response = key_chip(
                    ui,
                    cell.rect,
                    ui.id().with((id_salt, "mod", mask)),
                    &key,
                    mods & mask != 0,
                    style,
                );
                if response.clicked() {
                    on_toggle(mask);
                }
                response.on_hover_text(format!(
                    "{} {full} modifier",
                    if hand == Hand::Left { "Left" } else { "Right" },
                    full = full_names[i]
                ));
            }
        });
    }
}
