//! QMK/VIA editor content: left-panel sections for the basic/media pickers,
//! layer keys, the four modifier encodings (combo, one-shot, mod-tap,
//! layer-tap), and a raw-hex "Any" fallback. Writes apply immediately (VIA
//! behavior — no save button).

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use crate::qmk_keycode_labels::constants::*;
use crate::qmk_keycode_labels::get_layout_key;

use super::picker::{
    framed_candidate_groups_rows, modifier_toggle_row, picker_grid_rows, Candidate, Hand, KEY_UNIT,
};
use super::qmk_catalog::{qmk_candidate, LayerKind};
use super::EditTarget;
use crate::ui_widgets::titled_group;

/// The editor's left-panel sections, one entry each. The four modifier
/// encodings are separate sections rather than a dropdown mode, so each is
/// reachable in one click; the section itself selects the encoding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Basic,
    Media,
    Special,
    Backlight,
    Rgblight,
    RgbMatrix,
    Audio,
    Custom,
    Layers,
    /// `(mods << 8) | kc` — `LSFT(kc)`-style modified keycodes.
    Combo,
    /// `OSM(mods)`.
    OneShot,
    /// `MT(mods, kc)`.
    ModTap,
    /// `LT(layer, kc)`.
    LayerTap,
    /// `LM(layer, mods)`.
    LayerMod,
    Any,
}

impl Section {
    const ALL: [Section; 15] = [
        Section::Basic,
        Section::Media,
        Section::Special,
        Section::Backlight,
        Section::Rgblight,
        Section::RgbMatrix,
        Section::Audio,
        Section::Custom,
        Section::Layers,
        Section::Combo,
        Section::OneShot,
        Section::ModTap,
        Section::LayerTap,
        Section::LayerMod,
        Section::Any,
    ];
    fn label(&self) -> &'static str {
        match self {
            Section::Basic => "Basic",
            Section::Media => "Media",
            Section::Special => "Special",
            Section::Backlight => "Backlight",
            Section::Rgblight => "RGB Underglow",
            Section::RgbMatrix => "RGB Matrix",
            Section::Audio => "Audio",
            Section::Custom => "Custom",
            Section::Layers => "Layers",
            Section::Combo => "Mod Combo",
            Section::OneShot => "One-Shot Mod",
            Section::ModTap => "Mod-Tap",
            Section::LayerTap => "Layer-Tap",
            Section::LayerMod => "Layer-Mod",
            Section::Any => "Any",
        }
    }

    fn from_label(label: &str) -> Option<Section> {
        Self::ALL.iter().copied().find(|s| s.label() == label)
    }

    fn is_supported(&self, keyboard: &Keyboard) -> bool {
        super::qmk_catalog::categories()
            .iter()
            .find(|g| g.name == self.label())
            .map_or(true, |group| {
                group
                    .candidates
                    .iter()
                    .any(|c| keyboard.is_action_supported(&c.binding))
            })
    }

    fn has_layer(&self) -> bool {
        matches!(self, Section::LayerTap | Section::LayerMod)
    }

    fn has_mods(&self) -> bool {
        matches!(
            self,
            Section::Combo | Section::OneShot | Section::ModTap | Section::LayerMod
        )
    }

    fn has_tap_key(&self) -> bool {
        matches!(self, Section::Combo | Section::ModTap | Section::LayerTap)
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
    /// Whether the user has interacted with the draft's parameter controls
    /// since it was built. Only a touched draft can be mid-selection (and so
    /// ghosted as invalid in the header); a fresh draft mirrors the current
    /// binding.
    pub touched: bool,
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
            touched: false,
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

    /// The keycode the draft currently describes, if every argument is
    /// meaningfully present: modifier encodings need at least one modifier,
    /// and a tap/base key of `KC_NO` (0x00) counts as "not picked yet". A
    /// valid draft applies instantly, so it never lingers staged.
    pub(super) fn staged(&self) -> Option<u16> {
        match self.section {
            Section::Combo if self.base_code != 0 => encode_combo(self.mod_value(), self.base_code),
            Section::ModTap if self.base_code != 0 => {
                encode_mod_tap(self.mod_value(), self.base_code)
            }
            Section::LayerTap if self.base_code != 0 => {
                encode_layer_tap(self.mod_tap_layer.min(15), self.base_code)
            }
            Section::LayerMod => encode_layer_mod(self.mod_tap_layer.min(15), self.mod_value()),
            Section::OneShot => encode_one_shot_mod(self.mod_value()),
            Section::Any => u16::from_str_radix(&self.hex, 16).ok(),
            _ => None,
        }
    }

    /// The ghost binding shown in the header while the user is mid-selection
    /// and the draft is not yet a valid binding: the closest meaningful key —
    /// the picked tap/base key, or the layer of an incomplete layer-tap.
    /// Modifier-less one-shot and unparseable hex have no key to preview and
    /// ghost as an empty key.
    pub(super) fn ghost_action(&self) -> Option<KeyAction> {
        if !self.touched || self.staged().is_some() {
            return None;
        }
        let code = match self.section {
            Section::Combo | Section::ModTap => self.base_code,
            Section::LayerTap | Section::LayerMod => {
                QK_MOMENTARY.start + self.mod_tap_layer.min(15) as u16
            }
            Section::OneShot | Section::Any => 0,
            _ => return None,
        };
        Some(KeyAction::Qmk(code))
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

/// `LM(layer, mods)`: the layer is 4 bits (0–15) and mods is 5 bits (1–31).
fn encode_layer_mod(layer: usize, mods: u16) -> Option<u16> {
    if layer > 15 || mods & 0x1F == 0 {
        return None;
    }
    Some(QK_LAYER_MOD.start + ((layer as u16) << 5) + (mods & 0x1F))
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
    if QK_LAYER_MOD.contains(&code) {
        let remainder = code - QK_LAYER_MOD.start;
        draft.section = Section::LayerMod;
        draft.mod_tap_layer = (remainder >> 5) as usize;
        let mod_value = remainder & 0x1F;
        draft.mods = mod_value & 0x0F;
        draft.right = mod_value & MOD_RIGHT_FLAG != 0;
        return draft;
    }

    for group in super::qmk_catalog::categories() {
        if group
            .candidates
            .iter()
            .any(|c| c.binding == KeyAction::Qmk(code))
        {
            if let Some(section) = Section::from_label(group.name) {
                draft.section = section;
                return draft;
            }
        }
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
        // right pane shows (a key grid for catalog categories, parameter forms
        // for the modifier sections, Layers, and Any).
        super::editor_panes(
            ui,
            "qmk_sections",
            100.0,
            draft,
            |ui, draft| {
                for section in Section::ALL {
                    if !section.is_supported(keyboard) {
                        continue;
                    }
                    let response =
                        ui.selectable_value(&mut draft.section, section, section.label());
                    // Switching sections starts a fresh selection.
                    if response.changed() {
                        draft.touched = false;
                    }
                }
            },
            |ui, draft| match draft.section {
                Section::Layers => {
                    // One page of grouped layer keys, one per real layer; see
                    // draw_qmk_layer_page.
                    self.draw_qmk_layer_page(ui, keyboard, target);
                }
                Section::Combo
                | Section::OneShot
                | Section::ModTap
                | Section::LayerTap
                | Section::LayerMod => self.draw_qmk_mods_page(ui, keyboard, target, draft),
                Section::Any => {
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
                                // Every parsed prefix applies at once; the
                                // final value sticks as the last write.
                                self.commit_qmk_draft(keyboard, target, draft);
                            }
                        });
                        if u16::from_str_radix(&draft.hex, 16).is_err() {
                            ui.weak("Enter a 1–4 digit hex keycode");
                        }
                    });
                }
                _ => {
                    if let Some(group) = super::qmk_catalog::categories()
                        .iter()
                        .find(|g| g.name == draft.section.label())
                    {
                        let filtered_candidates: Vec<super::picker::Candidate> = group
                            .candidates
                            .iter()
                            .filter(|c| keyboard.is_action_supported(&c.binding))
                            .cloned()
                            .collect();
                        let style = self.paint_style(KEY_UNIT);
                        titled_group(ui, group.name, |ui| {
                            picker_grid_rows(
                                ui,
                                group.name,
                                &filtered_candidates,
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

    /// The five modifier sections share one page body: each distinct encoding
    /// argument is framed in its own titled group — the modifier set (with its
    /// hand) for the mod-carrying encodings, the layer for Layer-Tap and Layer-Mod,
    /// and the tap/base key where the encoding takes one. Every interaction stages
    /// into the draft and a complete binding applies instantly; an incomplete
    /// one ghosts the header slot until it becomes valid.
    fn draw_qmk_mods_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut QmkDraft,
    ) {
        if draft.section.has_layer() {
            let (group, staged_code) = match draft.section {
                Section::LayerTap => (
                    super::qmk_catalog::layer_tap_group(
                        keyboard.layer_infos().len(),
                        draft.base_code,
                    ),
                    QK_LAYER_TAP.start
                        + ((draft.mod_tap_layer.min(15) as u16) << 8)
                        + draft.base_code,
                ),
                Section::LayerMod => (
                    super::qmk_catalog::layer_mod_group(
                        keyboard.layer_infos().len(),
                        draft.mod_value(),
                    ),
                    QK_LAYER_MOD.start
                        + ((draft.mod_tap_layer.min(15) as u16) << 5)
                        + (draft.mod_value() & 0x1F),
                ),
                _ => unreachable!(),
            };
            let style = self.paint_style(KEY_UNIT);
            titled_group(ui, "Layer", |ui| {
                picker_grid_rows(
                    ui,
                    "qmk_layer",
                    &group.candidates,
                    Some(&KeyAction::Qmk(staged_code)),
                    &style,
                    |candidate| {
                        if let KeyAction::Qmk(code) = &candidate.binding {
                            draft.mod_tap_layer = match draft.section {
                                Section::LayerTap => ((code - QK_LAYER_TAP.start) >> 8) as usize,
                                Section::LayerMod => ((code - QK_LAYER_MOD.start) >> 5) as usize,
                                _ => 0,
                            };
                            self.commit_qmk_draft(keyboard, target, draft);
                        }
                    },
                );
            });
        }

        if draft.section.has_mods() {
            let hand = if draft.right { Hand::Right } else { Hand::Left };
            let mod_style = self.paint_style(KEY_UNIT);
            titled_group(ui, "Modifiers", |ui| {
                ui.horizontal(|ui| {
                    modifier_toggle_row(ui, "qmk_mods", draft.mods, hand, &mod_style, |mask| {
                        draft.mods ^= mask;
                        self.commit_qmk_draft(keyboard, target, draft);
                    });
                    ui.weak("Hand");
                    if ui
                        .add(egui::Button::new("L").small().selected(!draft.right))
                        .clicked()
                    {
                        draft.right = false;
                        self.commit_qmk_draft(keyboard, target, draft);
                    }
                    if ui
                        .add(egui::Button::new("R").small().selected(draft.right))
                        .on_hover_text("Right-hand modifiers (RCTL, RSFT, RALT, RGUI)")
                        .clicked()
                    {
                        draft.right = true;
                        self.commit_qmk_draft(keyboard, target, draft);
                    }
                });
            });
        }

        if draft.section.has_tap_key() {
            titled_group(ui, "Tap/base key (8-bit basic only)", |ui| {
                self.draw_base_picker(ui, keyboard, target, draft);
            });
        }
    }

    /// Marks the draft touched and applies its staged keycode when it is a
    /// complete, changed binding. QMK writes are immediate, so a valid pick
    /// applies at once — there is no explicit Apply step.
    fn commit_qmk_draft(&mut self, keyboard: &Keyboard, target: EditTarget, draft: &mut QmkDraft) {
        draft.touched = true;
        if let Some(code) = draft.staged() {
            let action = KeyAction::Qmk(code);
            if keyboard
                .get_action(target.layer_index, target.row, target.col)
                .as_ref()
                != Some(&action)
            {
                self.apply_write(keyboard, target, action);
            }
        }
    }

    fn draw_base_picker(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut QmkDraft,
    ) {
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
                    self.commit_qmk_draft(keyboard, target, draft);
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap_editor::qmk_catalog::encode_layer;
    use qmk_via_api::keycodes::Keycode;

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

    #[test]
    fn layer_mod_round_trips() {
        let code = encode_layer_mod(3, MOD_LSFT | MOD_LCTL).unwrap(); // LM(3, LSFT|LCTL)
        let mut draft = QmkDraft::default();
        draft.section = Section::LayerMod;
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mod_tap_layer, 3);
        assert_eq!(decode(code).mods, MOD_LSFT | MOD_LCTL);
        assert!(encode_layer_mod(16, MOD_LSFT).is_none());
        assert!(encode_layer_mod(3, 0).is_none());
    }

    #[test]
    fn staged_requires_every_argument() {
        // A modifier encoding without modifiers or without a tap key is
        // mid-selection, not a bindable keycode.
        let mut draft = QmkDraft::default();
        draft.section = Section::ModTap;
        draft.base_code = 0x04;
        assert_eq!(draft.staged(), None);
        draft.mods = MOD_LSFT;
        assert_eq!(draft.staged(), encode_mod_tap(MOD_LSFT, 0x04));

        draft.section = Section::LayerTap;
        draft.mod_tap_layer = 2;
        draft.base_code = 0;
        assert_eq!(draft.staged(), None);
        draft.base_code = 0x1C;
        assert_eq!(draft.staged(), encode_layer_tap(2, 0x1C));

        draft.section = Section::LayerMod;
        draft.mod_tap_layer = 3;
        draft.mods = 0;
        assert_eq!(draft.staged(), None);
        draft.mods = MOD_LSFT;
        assert_eq!(draft.staged(), encode_layer_mod(3, MOD_LSFT));

        draft.section = Section::OneShot;
        draft.mods = 0;
        assert_eq!(draft.staged(), None);
        draft.mods = MOD_LCTL;
        assert_eq!(draft.staged(), encode_one_shot_mod(MOD_LCTL));
    }

    #[test]
    fn ghost_previews_the_closest_key_while_invalid() {
        // Untouched or valid drafts never ghost; an invalid touched draft
        // previews its picked argument.
        let mut draft = QmkDraft::default();
        draft.section = Section::ModTap;
        assert_eq!(draft.ghost_action(), None);
        draft.touched = true;
        assert_eq!(
            draft.ghost_action(),
            Some(KeyAction::Qmk(0)), // no tap key picked yet: empty key
        );
        draft.base_code = 0x04;
        assert_eq!(
            draft.ghost_action(),
            Some(KeyAction::Qmk(0x04)), // the picked tap key, mods still empty
        );
        draft.mods = MOD_LSFT;
        assert_eq!(draft.ghost_action(), None); // now valid: applies instead
    }

    #[test]
    fn direct_category_keys_decode_into_their_sections() {
        assert_eq!(
            decode(Keycode::QK_BOOTLOADER as u16).section,
            Section::Special
        );
        assert_eq!(
            decode(Keycode::QK_BACKLIGHT_TOGGLE as u16).section,
            Section::Backlight
        );
        assert_eq!(
            decode(Keycode::QK_UNDERGLOW_TOGGLE as u16).section,
            Section::Rgblight
        );
        assert_eq!(decode(Keycode::QK_AUDIO_ON as u16).section, Section::Audio);
        assert_eq!(decode(Keycode::QK_KB_0 as u16).section, Section::Custom);
    }
}
