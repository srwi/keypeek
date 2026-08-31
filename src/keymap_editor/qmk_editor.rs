//! QMK keycode editor and encoder.

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use crate::qmk_keycode_labels::constants::*;
use crate::qmk_keycode_labels::get_layout_key;

use super::picker::{
    framed_candidate_groups_rows, modifier_toggle_row, picker_grid_rows, Candidate, KEY_UNIT,
};
use super::qmk_catalog::{qmk_candidate, LayerKind};
use super::EditTarget;
use crate::ui_widgets::titled_group;

/// Editor categories for QMK keycodes.
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
    /// Keycode with modifier mask (e.g. `LSFT(kc)`).
    Combo,
    /// One-shot modifier.
    OneShot,
    /// Mod-tap keycode.
    ModTap,
    /// Layer-tap keycode.
    LayerTap,
    /// Layer-mod keycode.
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
            .is_none_or(|group| {
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

/// Editable parameter state for QMK keycodes.
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
    /// Creates draft state by decoding an existing keycode.
    pub fn from_keycode(code: u16) -> Self {
        decode(code)
    }

    fn mod_value(&self) -> u16 {
        (self.mods & 0x0F) | if self.right { MOD_RIGHT_FLAG } else { 0 }
    }

    /// Returns the encoded keycode if all required parameters are valid.
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
}

// ── Encoders ────────────────────────────────────────────────────────────────

/// Encodes a modified keycode: `(mods << 8) | keycode`.
pub(super) fn encode_combo(mods: u16, keycode: u16) -> Option<u16> {
    if mods & 0x0F == 0 || keycode > 0xFF {
        return None;
    }
    let code = (mods << 8) | keycode;
    QK_MODS.contains(&code).then_some(code)
}

pub(super) fn encode_one_shot_mod(mods: u16) -> Option<u16> {
    if mods & 0x0F == 0 {
        return None;
    }
    Some(QK_ONE_SHOT_MOD.start + (mods & 0x1F))
}

pub(super) fn encode_mod_tap(mods: u16, keycode: u16) -> Option<u16> {
    if mods & 0x0F == 0 || keycode > 0xFF {
        return None;
    }
    Some(QK_MOD_TAP.start + (mods << 8) + keycode)
}

/// Encodes a Layer-Tap keycode: `LT(layer, keycode)`.
pub(super) fn encode_layer_tap(layer: usize, keycode: u16) -> Option<u16> {
    if layer > 15 || keycode > 0xFF {
        return None;
    }
    Some(QK_LAYER_TAP.start + ((layer as u16) << 8) + keycode)
}

/// Encodes a Layer-Mod keycode: `LM(layer, mods)`.
pub(super) fn encode_layer_mod(layer: usize, mods: u16) -> Option<u16> {
    if layer > 15 || mods & 0x0F == 0 {
        return None;
    }
    Some(QK_LAYER_MOD.start + ((layer as u16) << 5) + (mods & 0x1F))
}

// ── Decoder ─────────────────────────────────────────────────────────────────

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
                if code <= 0xFF {
                    draft.base_code = code;
                }
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
    ) {
        super::editor_left_panel(ui, "qmk_sections", |ui| {
            for s in Section::ALL {
                if !s.is_supported(keyboard) {
                    continue;
                }
                ui.selectable_value(&mut self.editor.qmk_draft.section, s, s.label());
            }
        });

        let section = self.editor.qmk_draft.section;
        super::editor_central_panel(ui, |ui| match section {
            Section::Layers => {
                // One page of grouped layer keys, one per real layer; see
                // draw_qmk_layer_page.
                self.draw_qmk_layer_page(ui, keyboard, target);
            }
            Section::Combo
            | Section::OneShot
            | Section::ModTap
            | Section::LayerTap
            | Section::LayerMod => self.draw_qmk_mods_page(ui, keyboard, target),
            Section::Any => {
                titled_group(ui, "Keycode", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("0x");
                        let mut text = self.editor.qmk_draft.hex.clone();
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut text)
                                .desired_width(80.0)
                                .char_limit(4),
                        );
                        if response.changed() {
                            text.retain(|c| c.is_ascii_hexdigit());
                            self.editor.qmk_draft.hex = text;
                        }
                        if response.lost_focus()
                            || (response.changed() && self.editor.qmk_draft.hex.len() == 4)
                        {
                            self.commit_qmk_draft(keyboard, target);
                        }
                    });
                    if u16::from_str_radix(&self.editor.qmk_draft.hex, 16).is_err() {
                        ui.weak("Enter a 1–4 digit hex keycode");
                    }
                });
            }
            _ => {
                if let Some(group) = super::qmk_catalog::categories()
                    .iter()
                    .find(|g| g.name == section.label())
                {
                    let style = self.paint_style(KEY_UNIT);
                    titled_group(ui, group.name, |ui| {
                        super::picker::picker_grid_filtered(
                            ui,
                            group.name,
                            &group.candidates,
                            |c| keyboard.is_action_supported(&c.binding),
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
        });
    }

    /// Draws the layer key selection page.
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

    /// Draws the parameter controls for composite modifier and layer keycodes.
    fn draw_qmk_mods_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
    ) {
        let section = self.editor.qmk_draft.section;
        if section.has_layer() {
            let layer = self.editor.qmk_draft.mod_tap_layer.min(15);
            let (group, staged_code) = match section {
                Section::LayerTap => (
                    super::qmk_catalog::layer_tap_group(
                        keyboard.layer_infos().len(),
                        self.editor.qmk_draft.base_code,
                    ),
                    encode_layer_tap(layer, self.editor.qmk_draft.base_code),
                ),
                Section::LayerMod => (
                    super::qmk_catalog::layer_mod_group(
                        keyboard.layer_infos().len(),
                        self.editor.qmk_draft.mod_value(),
                    ),
                    encode_layer_mod(layer, self.editor.qmk_draft.mod_value()),
                ),
                _ => unreachable!(),
            };
            let style = self.paint_style(KEY_UNIT);
            titled_group(ui, "Layer", |ui| {
                picker_grid_rows(
                    ui,
                    "qmk_layer",
                    &group.candidates,
                    staged_code.map(KeyAction::Qmk).as_ref(),
                    &style,
                    |candidate| {
                        if let KeyAction::Qmk(code) = &candidate.binding {
                            self.editor.qmk_draft.mod_tap_layer = match section {
                                Section::LayerTap => ((code - QK_LAYER_TAP.start) >> 8) as usize,
                                Section::LayerMod => ((code - QK_LAYER_MOD.start) >> 5) as usize,
                                _ => 0,
                            };
                            self.commit_qmk_draft(keyboard, target);
                        }
                    },
                );
            });
        }

        if section.has_mods() {
            let mod_style = self.paint_style(KEY_UNIT);
            titled_group(ui, "Modifiers", |ui| {
                if modifier_toggle_row(
                    ui,
                    "qmk_mods",
                    &mut self.editor.qmk_draft.mods,
                    &mut self.editor.qmk_draft.right,
                    &mod_style,
                ) {
                    self.commit_qmk_draft(keyboard, target);
                }
            });
        }

        if section.has_tap_key() {
            titled_group(ui, "Tap/base key (8-bit basic only)", |ui| {
                self.draw_base_picker(ui, keyboard, target);
            });
        }
    }

    /// Applies the current draft keycode to the target key.
    fn commit_qmk_draft(&mut self, keyboard: &Keyboard, target: EditTarget) {
        let staged = self.editor.qmk_draft.staged().map(KeyAction::Qmk);
        self.commit_staged(keyboard, target, staged);
    }

    fn draw_base_picker(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
    ) {
        let candidates: Vec<Candidate> = (0x00u16..=0xFF)
            .filter(|&code| get_layout_key(code).is_some())
            .map(qmk_candidate)
            .collect();
        let selected = KeyAction::Qmk(self.editor.qmk_draft.base_code);
        let style = self.paint_style(KEY_UNIT);
        picker_grid_rows(
            ui,
            "qmk_base",
            &candidates,
            Some(&selected),
            &style,
            |candidate| {
                if let KeyAction::Qmk(code) = &candidate.binding {
                    self.editor.qmk_draft.base_code = *code;
                    self.commit_qmk_draft(keyboard, target);
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
            let draft = QmkDraft {
                section: Section::Layers,
                ..Default::default()
            };
            assert_round_trips(draft, code);
        }
        assert!(encode_layer(LayerKind::Mo, 32).is_none());
    }

    #[test]
    fn mod_combo_round_trips() {
        let code = encode_combo(MOD_LSFT, 0x2A).unwrap(); // LSFT(Backspace)
        assert_eq!(code, (MOD_LSFT << 8) | 0x2A);
        let draft = QmkDraft {
            section: Section::Combo,
            ..Default::default()
        };
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
        let draft = QmkDraft {
            section: Section::OneShot,
            ..Default::default()
        };
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mods, MOD_LCTL | MOD_LSFT);
        assert!(encode_one_shot_mod(0).is_none());
    }

    #[test]
    fn mod_tap_round_trips() {
        let code = encode_mod_tap(MOD_LSFT | MOD_LALT, 0x04).unwrap(); // MT(LSFT|LALT, A)
        let draft = QmkDraft {
            section: Section::ModTap,
            ..Default::default()
        };
        assert_round_trips(draft, code);
        assert_eq!(decode(code).base_code, 0x04);
        assert_eq!(decode(code).mods, MOD_LSFT | MOD_LALT);
        assert!(encode_mod_tap(0, 0x04).is_none());
        assert!(encode_mod_tap(MOD_LSFT, 0x100).is_none());
    }

    #[test]
    fn layer_tap_round_trips() {
        let code = encode_layer_tap(2, 0x1C).unwrap(); // LT(2, Enter)
        let draft = QmkDraft {
            section: Section::LayerTap,
            ..Default::default()
        };
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mod_tap_layer, 2);
        assert_eq!(decode(code).base_code, 0x1C);
        assert!(encode_layer_tap(16, 0x1C).is_none());
        assert!(encode_layer_tap(2, 0x100).is_none());
    }

    #[test]
    fn layer_mod_round_trips() {
        let code = encode_layer_mod(3, MOD_LSFT | MOD_LCTL).unwrap(); // LM(3, LSFT|LCTL)
        let draft = QmkDraft {
            section: Section::LayerMod,
            ..Default::default()
        };
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
        let mut draft = QmkDraft {
            section: Section::ModTap,
            base_code: 0x04,
            ..Default::default()
        };
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
        // Having only the right-hand flag without any modifier bit is still invalid.
        draft.right = true;
        assert_eq!(draft.staged(), None);
        draft.mods = MOD_LCTL;
        assert_eq!(draft.staged(), encode_one_shot_mod(MOD_LCTL | MOD_RIGHT_FLAG));
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

        // Basic keys retain their base_code for seamless transition into ModTap/LayerTap/Combo.
        let a_draft = decode(Keycode::KC_A as u16);
        assert_eq!(a_draft.section, Section::Basic);
        assert_eq!(a_draft.base_code, Keycode::KC_A as u16);
    }
}
