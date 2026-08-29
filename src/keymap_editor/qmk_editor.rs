//! QMK/VIA editor content: left-panel sections for the basic/media pickers,
//! layer keys, the four modifier encodings (combo, one-shot, mod-tap,
//! layer-tap), and a raw-hex "Any" fallback. Writes apply immediately (VIA
//! behavior — no save button).

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use crate::qmk_keycode_labels::constants::*;
use crate::qmk_keycode_labels::get_layout_key;

use super::picker::{
    framed_candidate_groups_rows, modifier_toggle_row, picker_grid_rows, titled_group, Candidate,
    Hand, KEY_UNIT,
};
use super::qmk_catalog::{qmk_candidate, LayerKind};
use super::EditTarget;

/// The editor's left-panel sections, one entry each. The four modifier
/// encodings are separate sections rather than a dropdown mode, so each is
/// reachable in one click; the section itself selects the encoding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Basic,
    Media,
    Layers,
    /// `(mods << 8) | kc` — `LSFT(kc)`-style modified keycodes.
    Combo,
    /// `OSM(mods)`.
    OneShot,
    /// `MT(mods, kc)`.
    ModTap,
    /// `LT(layer, kc)`.
    LayerTap,
    Any,
}

impl Section {
    const ALL: [Section; 8] = [
        Section::Basic,
        Section::Media,
        Section::Layers,
        Section::Combo,
        Section::OneShot,
        Section::ModTap,
        Section::LayerTap,
        Section::Any,
    ];
    fn label(&self) -> &'static str {
        match self {
            Section::Basic => "Basic",
            Section::Media => "Media",
            Section::Layers => "Layers",
            Section::Combo => "Mod Combo",
            Section::OneShot => "One-Shot Mod",
            Section::ModTap => "Mod-Tap",
            Section::LayerTap => "Layer-Tap",
            Section::Any => "Any",
        }
    }
}

/// The editor's editable fields, rebuilt on each retarget.
#[derive(Clone)]
pub struct QmkDraft {
    pub section: Section,
    pub mods: u16,
    pub right: bool,
    pub base_code: u16,
    pub mod_tap_layer: usize,
    pub hex: String,
}

impl Default for QmkDraft {
    fn default() -> Self {
        Self {
            section: Section::Basic,
            mods: 0,
            right: false,
            base_code: 0,
            mod_tap_layer: 0,
            hex: String::new(),
        }
    }
}

impl QmkDraft {
    /// Pre-fills the draft from a QMK keycode, choosing the section and fields
    /// that best describe it. Unknown/plain codes land in the *Any* section.
    pub fn from_keycode(code: u16) -> Self {
        decode(code)
    }

    fn mod_value(&self) -> u16 {
        (self.mods & 0x0F) | if self.right { MOD_RIGHT_FLAG } else { 0 }
    }
}

// ── Encoders ────────────────────────────────────────────────────────────────

/// `LSFT(kc)`-style combo: `(mods << 8) | keycode`. Needs at least one mod and
/// an 8-bit tap key.
fn encode_combo(mods: u16, keycode: u16) -> Option<u16> {
    if mods & 0x1F == 0 || keycode > 0xFF {
        return None;
    }
    let code = (mods << 8) | keycode;
    QK_MODS.contains(&code).then_some(code)
}

fn encode_one_shot_mod(mods: u16) -> Option<u16> {
    if mods & 0x1F == 0 {
        return None;
    }
    Some(QK_ONE_SHOT_MOD.start + (mods & 0x1F))
}

fn encode_mod_tap(mods: u16, keycode: u16) -> Option<u16> {
    if mods & 0x1F == 0 || keycode > 0xFF {
        return None;
    }
    Some(QK_MOD_TAP.start + (mods << 8) + keycode)
}

/// `LT(layer, kc)`: the layer is only 4 bits (0–15).
fn encode_layer_tap(layer: usize, keycode: u16) -> Option<u16> {
    if layer > 15 || keycode > 0xFF {
        return None;
    }
    Some(QK_LAYER_TAP.start + ((layer as u16) << 8) + keycode)
}

// ── Decoder (pre-fill the draft from the current binding) ───────────────────

fn decode(code: u16) -> QmkDraft {
    let mut draft = QmkDraft::default();

    for kind in LayerKind::ALL {
        if kind.range().contains(&code) {
            draft.section = Section::Layers;
            return draft;
        }
    }

    if QK_MODS.contains(&code) {
        draft.section = Section::Combo;
        draft.mods = (code >> 8) & 0x0F;
        draft.right = (code >> 8) & MOD_RIGHT_FLAG != 0;
        draft.base_code = code & 0xFF;
        return draft;
    }
    if QK_MOD_TAP.contains(&code) {
        let remainder = code - QK_MOD_TAP.start;
        let mod_value = (remainder >> 8) & 0x1F;
        draft.section = Section::ModTap;
        draft.mods = mod_value & 0x0F;
        draft.right = mod_value & MOD_RIGHT_FLAG != 0;
        draft.base_code = remainder & 0xFF;
        return draft;
    }
    if QK_ONE_SHOT_MOD.contains(&code) {
        let mod_value = (code - QK_ONE_SHOT_MOD.start) & 0x1F;
        draft.section = Section::OneShot;
        draft.mods = mod_value & 0x0F;
        draft.right = mod_value & MOD_RIGHT_FLAG != 0;
        return draft;
    }
    if QK_LAYER_TAP.contains(&code) {
        let remainder = code - QK_LAYER_TAP.start;
        draft.section = Section::LayerTap;
        draft.mod_tap_layer = (remainder >> 8) as usize;
        draft.base_code = remainder & 0xFF;
        return draft;
    }

    draft.section = Section::Any;
    draft.hex = format!("{:04X}", code);
    draft
}

// ── Editor body ─────────────────────────────────────────────────────────────

impl crate::overlay_window::OverlayApp {
    pub(super) fn draw_qmk_editor_body(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut QmkDraft,
    ) {
        // Two-pane layout: the category list on the left drives whatever the
        // right pane shows (a key grid for Basic/Media, parameter forms for
        // the modifier sections, Layers, and Any).
        super::editor_panes(
            ui,
            "qmk_sections",
            100.0,
            draft,
            |ui, draft| {
                for section in Section::ALL {
                    ui.selectable_value(&mut draft.section, section, section.label());
                }
            },
            |ui, draft| match draft.section {
                Section::Basic | Section::Media => {
                    let group = super::qmk_catalog::categories()
                        .iter()
                        .find(|g| g.name == draft.section.label())
                        .expect("a candidate group per keycode section");
                    let style = self.paint_style(KEY_UNIT);
                    titled_group(ui, group.name, |ui| {
                        picker_grid_rows(
                            ui,
                            group.name,
                            &group.candidates,
                            keyboard
                                .get_action(target.layer_index, target.row, target.col)
                                .as_ref(),
                            &style,
                            |candidate| {
                                self.apply_write(keyboard, target, candidate.binding.clone());
                            },
                        );
                    });
                }
                Section::Layers => {
                    // One page of grouped layer keys, one per real layer; see
                    // draw_qmk_layer_page.
                    self.draw_qmk_layer_page(ui, keyboard, target);
                }
                Section::Combo | Section::OneShot | Section::ModTap | Section::LayerTap => {
                    self.draw_qmk_mods_page(ui, keyboard, target, draft)
                }
                Section::Any => {
                    let code = u16::from_str_radix(&draft.hex, 16);
                    // The preview and field hint describe the group's keycode,
                    // so they live inside the outline; only Apply sits outside.
                    titled_group(ui, "Keycode", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("0x");
                            let mut text = draft.hex.clone();
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut text)
                                    .desired_width(80.0)
                                    .char_limit(4),
                            );
                            if response.changed() {
                                text.retain(|c| c.is_ascii_hexdigit());
                                draft.hex = text;
                            }
                        });
                        match code {
                            Ok(code) => {
                                let label = get_layout_key(code)
                                    .map(|k| k.tap.full.clone())
                                    .unwrap_or_else(|| format!("0x{code:04X}"));
                                ui.weak(format!("Preview: {label}"));
                            }
                            Err(_) => {
                                ui.weak("Enter a 1–4 digit hex keycode");
                            }
                        }
                    });
                    if let Ok(code) = code {
                        if ui.button("Apply").clicked() {
                            self.apply_write(keyboard, target, KeyAction::Qmk(code));
                        }
                    }
                }
            },
        );
    }

    /// The layer page: every layer keycode kind as one framed group with one
    /// key per real layer, grouped like the ZMK layer page. A QMK layer keycode
    /// is fully determined by its kind and layer, so clicking applies directly
    /// and nothing needs staging. QMK keymaps carry no layer names, so keys
    /// show their index.
    fn draw_qmk_layer_page(&mut self, ui: &mut egui::Ui, keyboard: &Keyboard, target: EditTarget) {
        let layer_count = keyboard.layer_infos().len();
        let groups = super::qmk_catalog::layer_groups(layer_count);
        let selected = keyboard.get_action(target.layer_index, target.row, target.col);
        let style = self.paint_style(KEY_UNIT);
        framed_candidate_groups_rows(
            ui,
            &groups,
            |_| selected.clone(),
            &style,
            |_, candidate| {
                self.apply_write(keyboard, target, candidate.binding.clone());
            },
        );
    }

    /// The four modifier sections share one page body: each distinct encoding
    /// argument is framed in its own titled group — the modifier set (with its
    /// hand) for the three mod-carrying encodings, the layer for Layer-Tap, and
    /// the tap/base key where the encoding takes one. Everything stages into
    /// the draft and applies through the button below the groups.
    fn draw_qmk_mods_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut QmkDraft,
    ) {
        // Modifier toggles look like the overlay's modifier keys; selected bits
        // use the pressed look, matching the selected picker cell. Each chip's
        // bottom strip carries the currently selected hand, so the chips always
        // show which hand variant they would encode. Hand selection qualifies
        // the modifier set, so it lives inside the same group.
        let hand = if draft.right { Hand::Right } else { Hand::Left };
        match draft.section {
            Section::LayerTap => {
                // The layer radio from the layer page, one key per real layer;
                // clicking stages the layer, the tap picker below completes
                // the binding.
                let group = super::qmk_catalog::layer_tap_group(
                    keyboard.layer_infos().len(),
                    draft.base_code,
                );
                let staged = KeyAction::Qmk(
                    QK_LAYER_TAP.start
                        + ((draft.mod_tap_layer.min(15) as u16) << 8)
                        + draft.base_code,
                );
                let style = self.paint_style(KEY_UNIT);
                titled_group(ui, "Layer", |ui| {
                    picker_grid_rows(
                        ui,
                        "qmk_lt_layer",
                        &group.candidates,
                        Some(&staged),
                        &style,
                        |candidate| {
                            if let KeyAction::Qmk(code) = &candidate.binding {
                                draft.mod_tap_layer = ((code - QK_LAYER_TAP.start) >> 8) as usize;
                            }
                        },
                    );
                });
            }
            _ => {
                let mod_style = self.paint_style(KEY_UNIT);
                titled_group(ui, "Modifiers", |ui| {
                    ui.horizontal(|ui| {
                        modifier_toggle_row(ui, "qmk_mods", draft.mods, hand, &mod_style, |mask| {
                            draft.mods ^= mask;
                        });
                        ui.weak("Hand");
                        if ui
                            .add(egui::Button::new("L").small().selected(!draft.right))
                            .clicked()
                        {
                            draft.right = false;
                        }
                        if ui
                            .add(egui::Button::new("R").small().selected(draft.right))
                            .on_hover_text("Right-hand modifiers (RCTL, RSFT, RALT, RGUI)")
                            .clicked()
                        {
                            draft.right = true;
                        }
                    });
                });
            }
        }

        match draft.section {
            Section::LayerTap => {
                titled_group(ui, "Tap key (8-bit basic only)", |ui| {
                    self.draw_base_picker(ui, draft);
                });
            }
            Section::Combo | Section::ModTap => {
                titled_group(ui, "Tap/base key (8-bit basic only)", |ui| {
                    self.draw_base_picker(ui, draft);
                });
            }
            Section::OneShot => {}
            // Only the four modifier sections reach this page.
            _ => {}
        }

        let code = match draft.section {
            Section::Combo => encode_combo(draft.mod_value(), draft.base_code),
            Section::OneShot => encode_one_shot_mod(draft.mod_value()),
            Section::ModTap => encode_mod_tap(draft.mod_value(), draft.base_code),
            Section::LayerTap => encode_layer_tap(draft.mod_tap_layer, draft.base_code),
            _ => None,
        };

        if ui.button("Apply").clicked() {
            if let Some(code) = code {
                self.apply_write(keyboard, target, KeyAction::Qmk(code));
            }
        }
    }

    fn draw_base_picker(&mut self, ui: &mut egui::Ui, draft: &mut QmkDraft) {
        let candidates: Vec<Candidate> = (0x00u16..=0xFF)
            .filter(|&code| get_layout_key(code).is_some())
            .map(qmk_candidate)
            .collect();
        let selected = KeyAction::Qmk(draft.base_code);
        let style = self.paint_style(KEY_UNIT);
        picker_grid_rows(
            ui,
            "qmk_base",
            &candidates,
            Some(&selected),
            &style,
            |candidate| {
                if let KeyAction::Qmk(code) = &candidate.binding {
                    draft.base_code = *code;
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap_editor::qmk_catalog::encode_layer;

    fn assert_round_trips(draft: QmkDraft, code: u16) {
        // The encoded code must decode back into the same section.
        let decoded = decode(code);
        assert_eq!(decoded.section, draft.section, "code 0x{code:04X} section");
        // And the layout must resolve it (matches the decoders in
        // layer.rs / advanced.rs that drive the overlay).
        assert!(
            get_layout_key(code).is_some(),
            "0x{code:04X} should resolve"
        );
    }

    #[test]
    fn layer_keys_encode_and_round_trip() {
        for (kind, layer, expected_start) in [
            (LayerKind::Mo, 3, QK_MOMENTARY.start),
            (LayerKind::Tg, 1, QK_TOGGLE_LAYER.start),
            (LayerKind::To, 0, QK_TO.start),
            (LayerKind::Osl, 2, QK_ONE_SHOT_LAYER.start),
            (LayerKind::Tt, 5, QK_LAYER_TAP_TOGGLE.start),
            (LayerKind::Df, 4, QK_DEF_LAYER.start),
        ] {
            let code = encode_layer(kind, layer).unwrap();
            assert_eq!(code, expected_start + layer as u16);
            let mut draft = QmkDraft::default();
            draft.section = Section::Layers;
            assert_round_trips(draft, code);
        }
        assert!(encode_layer(LayerKind::Mo, 32).is_none());
    }

    #[test]
    fn mod_combo_round_trips() {
        let code = encode_combo(MOD_LSFT, 0x2A).unwrap(); // LSFT(Backspace)
        assert_eq!(code, (MOD_LSFT << 8) | 0x2A);
        let mut draft = QmkDraft::default();
        draft.section = Section::Combo;
        assert_round_trips(draft, code);
        assert_eq!(decode(code).base_code, 0x2A);
        assert!(decode(code).mods & MOD_LSFT != 0);

        // No modifier -> not a valid QK_MODS code.
        assert!(encode_combo(0, 0x2A).is_none());
        // Tap key must fit in 8 bits.
        assert!(encode_combo(MOD_LSFT, 0x100).is_none());
    }

    #[test]
    fn one_shot_mod_round_trips() {
        let code = encode_one_shot_mod(MOD_LCTL | MOD_LSFT).unwrap();
        assert_eq!(code, QK_ONE_SHOT_MOD.start + (MOD_LCTL | MOD_LSFT));
        let mut draft = QmkDraft::default();
        draft.section = Section::OneShot;
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mods, MOD_LCTL | MOD_LSFT);
        assert!(encode_one_shot_mod(0).is_none());
    }

    #[test]
    fn mod_tap_round_trips() {
        let code = encode_mod_tap(MOD_LSFT | MOD_LALT, 0x04).unwrap(); // MT(LSFT|LALT, A)
        let mut draft = QmkDraft::default();
        draft.section = Section::ModTap;
        assert_round_trips(draft, code);
        assert_eq!(decode(code).base_code, 0x04);
        assert_eq!(decode(code).mods, MOD_LSFT | MOD_LALT);
        assert!(encode_mod_tap(0, 0x04).is_none());
        assert!(encode_mod_tap(MOD_LSFT, 0x100).is_none());
    }

    #[test]
    fn layer_tap_round_trips() {
        let code = encode_layer_tap(2, 0x1C).unwrap(); // LT(2, Enter)
        let mut draft = QmkDraft::default();
        draft.section = Section::LayerTap;
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mod_tap_layer, 2);
        assert_eq!(decode(code).base_code, 0x1C);
        assert!(encode_layer_tap(16, 0x1C).is_none());
        assert!(encode_layer_tap(2, 0x100).is_none());
    }
}
