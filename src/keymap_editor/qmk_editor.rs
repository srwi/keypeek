//! QMK keycode editor and encoder.

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use crate::qmk_keycode_labels::get_layout_key;

use super::picker::{
    framed_candidate_groups_rows, modifier_toggle_row, picker_grid_rows, Candidate, KEY_UNIT,
};
use super::qmk_catalog::qmk_candidate;
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

use qmk_via_api::keycodes::{Keycode, KeycodeCategory};
use qmk_via_api::{QmkKeycode, QmkModMask};

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

    pub const fn category(&self) -> Option<KeycodeCategory> {
        match self {
            Section::Basic => Some(KeycodeCategory::Basic),
            Section::Media => Some(KeycodeCategory::Media),
            Section::Special => Some(KeycodeCategory::Special),
            Section::Backlight => Some(KeycodeCategory::Backlight),
            Section::Rgblight => Some(KeycodeCategory::Rgblight),
            Section::RgbMatrix => Some(KeycodeCategory::RgbMatrix),
            Section::Audio => Some(KeycodeCategory::Audio),
            Section::Custom => Some(KeycodeCategory::Custom),
            _ => None,
        }
    }

    pub const fn from_category(cat: KeycodeCategory) -> Option<Section> {
        match cat {
            KeycodeCategory::Basic => Some(Section::Basic),
            KeycodeCategory::Media => Some(Section::Media),
            KeycodeCategory::Special => Some(Section::Special),
            KeycodeCategory::Backlight => Some(Section::Backlight),
            KeycodeCategory::Rgblight => Some(Section::Rgblight),
            KeycodeCategory::RgbMatrix => Some(Section::RgbMatrix),
            KeycodeCategory::Audio => Some(Section::Audio),
            KeycodeCategory::Custom => Some(Section::Custom),
        }
    }

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

    fn is_supported(&self, keyboard: &Keyboard) -> bool {
        match self.category() {
            Some(cat) => super::qmk_catalog::category(cat)
                .candidates
                .iter()
                .any(|c| keyboard.is_action_supported(&c.binding)),
            None => true,
        }
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

    fn mod_mask(&self) -> QmkModMask {
        QmkModMask::from_bits((self.mods & 0x0F) as u8).with_right(self.right)
    }

    fn mod_value(&self) -> u16 {
        self.mod_mask().bits() as u16
    }

    /// Returns the encoded keycode if all required parameters are valid.
    pub(super) fn staged(&self) -> Option<u16> {
        match self.section {
            Section::Combo if self.base_code != 0 => {
                QmkKeycode::encode_mod_combo(self.mod_mask(), self.base_code as u8)
            }
            Section::ModTap if self.base_code != 0 => {
                QmkKeycode::encode_mod_tap(self.mod_mask(), self.base_code as u8)
            }
            Section::LayerTap if self.base_code != 0 => {
                QmkKeycode::encode_layer_tap(self.mod_tap_layer.min(15) as u8, self.base_code as u8)
            }
            Section::LayerMod => {
                QmkKeycode::encode_layer_mod(self.mod_tap_layer.min(15) as u8, self.mod_mask())
            }
            Section::OneShot => QmkKeycode::encode_one_shot_mod(self.mod_mask()),
            Section::Any => u16::from_str_radix(&self.hex, 16).ok(),
            _ => None,
        }
    }
}

// ── Decoder ─────────────────────────────────────────────────────────────────

fn decode(code: u16) -> QmkDraft {
    let mut draft = QmkDraft::default();

    match QmkKeycode::from_u16(code) {
        QmkKeycode::LayerOp { .. } => {
            draft.section = Section::Layers;
            return draft;
        }
        QmkKeycode::ModCombo { mods, keycode } => {
            draft.section = Section::Combo;
            draft.mods = (mods.bits() & 0x0F) as u16;
            draft.right = mods.is_right();
            draft.base_code = keycode as u16;
            return draft;
        }
        QmkKeycode::ModTap { mods, keycode } => {
            draft.section = Section::ModTap;
            draft.mods = (mods.bits() & 0x0F) as u16;
            draft.right = mods.is_right();
            draft.base_code = keycode as u16;
            return draft;
        }
        QmkKeycode::OneShotMod(mods) => {
            draft.section = Section::OneShot;
            draft.mods = (mods.bits() & 0x0F) as u16;
            draft.right = mods.is_right();
            return draft;
        }
        QmkKeycode::LayerTap { layer, keycode } => {
            draft.section = Section::LayerTap;
            draft.mod_tap_layer = layer as usize;
            draft.base_code = keycode as u16;
            return draft;
        }
        QmkKeycode::LayerMod { layer, mods } => {
            draft.section = Section::LayerMod;
            draft.mod_tap_layer = layer as usize;
            draft.mods = (mods.bits() & 0x0F) as u16;
            draft.right = mods.is_right();
            return draft;
        }
        _ => {}
    }

    for &cat in &KeycodeCategory::ALL {
        if Keycode::all_in_category(cat)
            .iter()
            .any(|&k| k as u16 == code)
        {
            if let Some(section) = Section::from_category(cat) {
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
        let style = self.paint_style(KEY_UNIT);
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
                self.draw_qmk_layer_page(ui, keyboard, target, &style);
            }
            Section::Combo
            | Section::OneShot
            | Section::ModTap
            | Section::LayerTap
            | Section::LayerMod => self.draw_qmk_mods_page(ui, keyboard, target, &style),
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
                if let Some(cat) = section.category() {
                    let group = super::qmk_catalog::category(cat);
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
    fn draw_qmk_layer_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let layer_count = keyboard.layer_infos().len();
        let groups = super::qmk_catalog::layer_groups(layer_count);
        let selected = keyboard.get_action(target.layer_index, target.row, target.col);
        framed_candidate_groups_rows(
            ui,
            &groups,
            |_| selected.clone(),
            style,
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
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let section = self.editor.qmk_draft.section;
        if section.has_layer() {
            let layer = self.editor.qmk_draft.mod_tap_layer.min(15) as u8;
            let (group, staged_code) = match section {
                Section::LayerTap => (
                    super::qmk_catalog::layer_tap_group(
                        keyboard.layer_infos().len(),
                        self.editor.qmk_draft.base_code,
                    ),
                    QmkKeycode::encode_layer_tap(layer, self.editor.qmk_draft.base_code as u8),
                ),
                Section::LayerMod => (
                    super::qmk_catalog::layer_mod_group(
                        keyboard.layer_infos().len(),
                        self.editor.qmk_draft.mod_value(),
                    ),
                    QmkKeycode::encode_layer_mod(layer, self.editor.qmk_draft.mod_mask()),
                ),
                _ => unreachable!(),
            };
            titled_group(ui, "Layer", |ui| {
                picker_grid_rows(
                    ui,
                    "qmk_layer",
                    &group.candidates,
                    staged_code.map(KeyAction::Qmk).as_ref(),
                    style,
                    |candidate| {
                        if let KeyAction::Qmk(code) = &candidate.binding {
                            match QmkKeycode::from_u16(*code) {
                                QmkKeycode::LayerTap { layer, .. } => {
                                    self.editor.qmk_draft.mod_tap_layer = layer as usize;
                                }
                                QmkKeycode::LayerMod { layer, .. } => {
                                    self.editor.qmk_draft.mod_tap_layer = layer as usize;
                                }
                                _ => {}
                            }
                            self.commit_qmk_draft(keyboard, target);
                        }
                    },
                );
            });
        }

        if section.has_mods() {
            titled_group(ui, "Modifiers", |ui| {
                if modifier_toggle_row(
                    ui,
                    "qmk_mods",
                    &mut self.editor.qmk_draft.mods,
                    &mut self.editor.qmk_draft.right,
                    style,
                ) {
                    self.commit_qmk_draft(keyboard, target);
                }
            });
        }

        if section.has_tap_key() {
            titled_group(ui, "Tap/base key (8-bit basic only)", |ui| {
                self.draw_base_picker(ui, keyboard, target, style);
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
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let candidates: Vec<Candidate> = (0x00u16..=0xFF)
            .filter(|&code| get_layout_key(code).is_some())
            .map(qmk_candidate)
            .collect();
        let selected = KeyAction::Qmk(self.editor.qmk_draft.base_code);
        picker_grid_rows(
            ui,
            "qmk_base",
            &candidates,
            Some(&selected),
            style,
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
    use crate::qmk_keycode_labels::constants::*;
    use qmk_via_api::keycodes::Keycode;
    use qmk_via_api::QmkLayerOp;

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
        for (op, layer, expected_start) in [
            (QmkLayerOp::Momentary, 3, QK_MOMENTARY.start),
            (QmkLayerOp::Toggle, 1, QK_TOGGLE_LAYER.start),
            (QmkLayerOp::To, 0, QK_TO.start),
            (QmkLayerOp::OneShot, 2, QK_ONE_SHOT_LAYER.start),
            (QmkLayerOp::TapToggle, 5, QK_LAYER_TAP_TOGGLE.start),
            (QmkLayerOp::Default, 4, QK_DEF_LAYER.start),
        ] {
            let code = op.encode(layer).unwrap();
            assert_eq!(code, expected_start + layer as u16);
            let draft = QmkDraft {
                section: Section::Layers,
                ..Default::default()
            };
            assert_round_trips(draft, code);
        }
        assert!(QmkLayerOp::Momentary.encode(32).is_none());
    }

    #[test]
    fn mod_combo_round_trips() {
        let code =
            QmkKeycode::encode_mod_combo(QmkModMask::from_bits(QmkModMask::LSFT), 0x2A).unwrap(); // LSFT(Backspace)
        assert_eq!(code, (MOD_LSFT << 8) | 0x2A);
        let draft = QmkDraft {
            section: Section::Combo,
            ..Default::default()
        };
        assert_round_trips(draft, code);
        assert_eq!(decode(code).base_code, 0x2A);
        assert!(decode(code).mods & MOD_LSFT != 0);

        // No modifier -> not a valid QK_MODS code.
        assert!(QmkKeycode::encode_mod_combo(QmkModMask::empty(), 0x2A).is_none());
    }

    #[test]
    fn one_shot_mod_round_trips() {
        let mods = QmkModMask::from_bits(QmkModMask::LCTL | QmkModMask::LSFT);
        let code = QmkKeycode::encode_one_shot_mod(mods).unwrap();
        assert_eq!(code, QK_ONE_SHOT_MOD.start + (MOD_LCTL | MOD_LSFT));
        let draft = QmkDraft {
            section: Section::OneShot,
            ..Default::default()
        };
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mods, MOD_LCTL | MOD_LSFT);
        assert!(QmkKeycode::encode_one_shot_mod(QmkModMask::empty()).is_none());
    }

    #[test]
    fn mod_tap_round_trips() {
        let mods = QmkModMask::from_bits(QmkModMask::LSFT | QmkModMask::LALT);
        let code = QmkKeycode::encode_mod_tap(mods, 0x04).unwrap(); // MT(LSFT|LALT, A)
        let draft = QmkDraft {
            section: Section::ModTap,
            ..Default::default()
        };
        assert_round_trips(draft, code);
        assert_eq!(decode(code).base_code, 0x04);
        assert_eq!(decode(code).mods, MOD_LSFT | MOD_LALT);
        assert!(QmkKeycode::encode_mod_tap(QmkModMask::empty(), 0x04).is_none());
    }

    #[test]
    fn layer_tap_round_trips() {
        let code = QmkKeycode::encode_layer_tap(2, 0x1C).unwrap(); // LT(2, Enter)
        let draft = QmkDraft {
            section: Section::LayerTap,
            ..Default::default()
        };
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mod_tap_layer, 2);
        assert_eq!(decode(code).base_code, 0x1C);
        assert!(QmkKeycode::encode_layer_tap(16, 0x1C).is_none());
    }

    #[test]
    fn layer_mod_round_trips() {
        let mods = QmkModMask::from_bits(QmkModMask::LSFT | QmkModMask::LCTL);
        let code = QmkKeycode::encode_layer_mod(3, mods).unwrap(); // LM(3, LSFT|LCTL)
        let draft = QmkDraft {
            section: Section::LayerMod,
            ..Default::default()
        };
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mod_tap_layer, 3);
        assert_eq!(decode(code).mods, MOD_LSFT | MOD_LCTL);
        assert!(QmkKeycode::encode_layer_mod(16, mods).is_none());
        assert!(QmkKeycode::encode_layer_mod(3, QmkModMask::empty()).is_none());
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
        assert_eq!(
            draft.staged(),
            QmkKeycode::encode_mod_tap(QmkModMask::from_bits(QmkModMask::LSFT), 0x04)
        );

        draft.section = Section::LayerTap;
        draft.mod_tap_layer = 2;
        draft.base_code = 0;
        assert_eq!(draft.staged(), None);
        draft.base_code = 0x1C;
        assert_eq!(draft.staged(), QmkKeycode::encode_layer_tap(2, 0x1C));

        draft.section = Section::LayerMod;
        draft.mod_tap_layer = 3;
        draft.mods = 0;
        assert_eq!(draft.staged(), None);
        draft.mods = MOD_LSFT;
        assert_eq!(
            draft.staged(),
            QmkKeycode::encode_layer_mod(3, QmkModMask::from_bits(QmkModMask::LSFT))
        );

        draft.section = Section::OneShot;
        draft.mods = 0;
        assert_eq!(draft.staged(), None);
        // Having only the right-hand flag without any modifier bit is still invalid.
        draft.right = true;
        assert_eq!(draft.staged(), None);
        draft.mods = MOD_LCTL;
        assert_eq!(
            draft.staged(),
            QmkKeycode::encode_one_shot_mod(QmkModMask::from_bits(
                QmkModMask::LCTL | QmkModMask::RIGHT_HAND
            ))
        );
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
