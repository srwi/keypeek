//! A scrollable grid of key-shaped buttons shared by the QMK and ZMK editors.

/// One selectable candidate in a picker grid.
pub struct Candidate {
    /// The firmware value the candidate writes.
    pub code: u32,
    /// The button text (the resolved label, already special-cased where the
    /// raw label is unusable as button text).
    pub text: String,
}

/// A scrollable grid of small buttons, one per candidate. The currently
/// assigned `selected` code is highlighted. Clicking a button invokes
/// `on_select` with the candidate's code.
pub fn picker_grid(
    ui: &mut egui::Ui,
    id_salt: &str,
    candidates: &[Candidate],
    selected: Option<u32>,
    on_select: impl FnMut(u32),
) {
    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            picker_grid_rows(ui, id_salt, candidates, selected, on_select);
        });
}

/// The grid inside `picker_grid`, without its own scroll area (for embedding
/// inside an outer scroll region). `id_salt` must be unique per grid in the
/// same window so egui does not report duplicate widget ids.
pub fn picker_grid_rows(
    ui: &mut egui::Ui,
    id_salt: &str,
    candidates: &[Candidate],
    selected: Option<u32>,
    mut on_select: impl FnMut(u32),
) {
    let button_size = egui::vec2(
        ui.spacing().interact_size.y * 2.4,
        ui.spacing().interact_size.y,
    );
    let spacing = ui.spacing().item_spacing.x;
    let cols = ((ui.available_width() + spacing) / (button_size.x + spacing))
        .floor()
        .max(1.0) as usize;

    egui::Grid::new(ui.id().with(id_salt))
        .spacing([spacing, 6.0])
        .show(ui, |ui| {
            for (i, candidate) in candidates.iter().enumerate() {
                let is_selected = selected == Some(candidate.code);
                let button = ui.add_sized(
                    button_size,
                    egui::Button::new(egui::RichText::new(&candidate.text).small().strong())
                        .selected(is_selected),
                );
                if button.clicked() {
                    on_select(candidate.code);
                }
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
            if candidates.len() % cols != 0 {
                ui.end_row();
            }
        });
}

/// The button text for a QMK keycode: `KC_TRANSPARENT` resolves to no label
/// and `KC_NO` to an empty one, so both get explicit text.
pub fn qmk_candidate_text(code: u16) -> String {
    use crate::qmk_keycode_labels::get_layout_key;
    use qmk_via_api::keycodes::Keycode;

    if code == Keycode::KC_TRANSPARENT as u16 {
        return "\u{25bd} Trans".to_string();
    }
    if code == Keycode::KC_NO as u16 {
        return "None".to_string();
    }
    match get_layout_key(code) {
        Some(key) if !key.tap.full.is_empty() => key.tap.full.clone(),
        Some(key) => key
            .symbol
            .clone()
            .unwrap_or_else(|| format!("0x{code:04X}")),
        None => format!("0x{code:04X}"),
    }
}
