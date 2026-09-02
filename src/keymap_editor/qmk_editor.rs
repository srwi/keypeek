//! QMK keycode editor and encoder.

use super::picker::{
    framed_candidate_groups, modifier_toggle_row, multi_candidate_groups, titled_candidate_group,
    SelectedKey,
};
use super::{EditTarget, EditorState};
use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use crate::ui_widgets::titled_group;

/// Editor categories for QMK keycodes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
use qmk_via_api::{QmkKeycode, QmkLayerOp, QmkModMask};

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
        if *self == Section::Custom {
            return super::qmk_catalog::custom_groups()
                .iter()
                .flat_map(|g| &g.candidates)
                .any(|c| keyboard.is_action_supported(&c.binding));
        }
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
    pub mod_tap_layer: Option<usize>,
    pub hex: String,
}

impl Default for QmkDraft {
    fn default() -> Self {
        Self {
            section: Section::Basic,
            mods: 0,
            right: false,
            base_code: 0,
            mod_tap_layer: None,
            hex: String::new(),
        }
    }
}

impl QmkDraft {
    /// Initializes draft for a section, preserving the active keycode if it matches the section.
    pub(super) fn for_section(section: Section, current_action: Option<&KeyAction>) -> Self {
        match current_action {
            Some(KeyAction::Qmk(code)) => {
                let d = Self::from_keycode(*code);
                if d.section == section {
                    d
                } else {
                    Self {
                        section,
                        ..Default::default()
                    }
                }
            }
            _ => Self {
                section,
                ..Default::default()
            },
        }
    }

    /// Creates draft state by decoding an existing keycode.
    pub fn from_keycode(code: u16) -> Self {
        decode(code)
    }

    fn mod_mask(&self) -> QmkModMask {
        QmkModMask::from_bits((self.mods & 0x0F) as u8).with_right(self.right)
    }

    /// Returns the encoded keycode if all required parameters are valid.
    pub(super) fn staged(&self) -> Option<u16> {
        match self.section {
            Section::Combo if self.base_code != 0 && self.mods != 0 => {
                QmkKeycode::encode_mod_combo(self.mod_mask(), self.base_code as u8)
            }
            Section::ModTap if self.base_code != 0 && self.mods != 0 => {
                QmkKeycode::encode_mod_tap(self.mod_mask(), self.base_code as u8)
            }
            Section::LayerTap if self.base_code != 0 => {
                let layer = self.mod_tap_layer?.min(15) as u8;
                QmkKeycode::encode_layer_tap(layer, self.base_code as u8)
            }
            Section::LayerMod if self.mods != 0 => {
                let layer = self.mod_tap_layer?.min(15) as u8;
                QmkKeycode::encode_layer_mod(layer, self.mod_mask())
            }
            Section::OneShot if self.mods != 0 => QmkKeycode::encode_one_shot_mod(self.mod_mask()),
            Section::Any => u16::from_str_radix(&self.hex, 16).ok(),
            _ => None,
        }
    }

    /// Returns whether the draft contains a valid, complete key configuration.
    pub fn is_valid(&self) -> bool {
        match self.section {
            Section::Combo
            | Section::OneShot
            | Section::ModTap
            | Section::LayerTap
            | Section::LayerMod
            | Section::Any => self.staged().is_some(),
            _ => true,
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
            draft.mod_tap_layer = Some(layer as usize);
            draft.base_code = keycode as u16;
            return draft;
        }
        QmkKeycode::LayerMod { layer, mods } => {
            draft.section = Section::LayerMod;
            draft.mod_tap_layer = Some(layer as usize);
            draft.mods = (mods.bits() & 0x0F) as u16;
            draft.right = mods.is_right();
            return draft;
        }
        QmkKeycode::TapDance(_)
        | QmkKeycode::Macro(_)
        | QmkKeycode::CustomKb(_)
        | QmkKeycode::CustomUser(_) => {
            draft.section = Section::Custom;
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

impl EditorState {
    pub(super) fn draw_qmk_editor_body(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let current_section = self.qmk_draft.section;
        let mut selected_section = None;
        super::editor_left_panel(ui, "qmk_sections", &mut self.search_query, |ui| {
            for s in Section::ALL {
                if !s.is_supported(keyboard) {
                    continue;
                }
                if ui
                    .selectable_label(current_section == s, s.label())
                    .clicked()
                {
                    selected_section = Some(s);
                }
            }
        });

        if let Some(s) = selected_section {
            self.search_query.clear();
            self.reset_qmk_draft_for_section(keyboard, target, s);
        }

        let section = self.qmk_draft.section;
        let search_query = self.search_query.clone();
        super::editor_central_panel(ui, (target.layer_index, section), |ui| match section {
            Section::Layers => {
                // One page of grouped layer keys, one per real layer; see
                // draw_qmk_layer_page.
                self.draw_qmk_layer_page(ui, keyboard, target, &search_query, style);
            }
            Section::Custom => {
                let groups = super::qmk_catalog::custom_groups();
                let action = keyboard.get_action(target.layer_index, target.row, target.col);
                let selected = action.as_ref().map(SelectedKey::valid);
                titled_group(ui, "Custom", |ui| {
                    multi_candidate_groups(
                        ui,
                        groups,
                        &search_query,
                        |c| keyboard.is_action_supported(&c.binding),
                        selected,
                        style,
                        |_, candidate| {
                            self.apply_write(keyboard, target, candidate.binding.clone());
                        },
                    );
                });
            }
            Section::Combo
            | Section::OneShot
            | Section::ModTap
            | Section::LayerTap
            | Section::LayerMod => {
                self.draw_qmk_mods_page(ui, keyboard, target, &search_query, style)
            }
            Section::Any => {
                titled_group(ui, "Keycode", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("0x");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.qmk_draft.hex)
                                .desired_width(80.0)
                                .char_limit(4),
                        );
                        if response.changed() {
                            self.qmk_draft.hex.retain(|c| c.is_ascii_hexdigit());
                        }
                        if response.lost_focus()
                            || (response.changed() && self.qmk_draft.hex.len() == 4)
                        {
                            self.commit_qmk_draft(keyboard, target);
                        }
                    });
                    if u16::from_str_radix(&self.qmk_draft.hex, 16).is_err() {
                        ui.weak("Enter a 1–4 digit hex keycode");
                    }
                });
            }
            _ => {
                if let Some(cat) = section.category() {
                    let group = super::qmk_catalog::category(cat);
                    let action = keyboard.get_action(target.layer_index, target.row, target.col);
                    titled_candidate_group(
                        ui,
                        group,
                        &search_query,
                        |c| keyboard.is_action_supported(&c.binding),
                        action.as_ref().map(SelectedKey::valid),
                        style,
                        |candidate| {
                            self.apply_write(keyboard, target, candidate.binding.clone());
                        },
                    );
                }
            }
        });
    }

    fn reset_qmk_draft_for_section(
        &mut self,
        keyboard: &Keyboard,
        target: EditTarget,
        section: Section,
    ) {
        let current_action = keyboard.get_action(target.layer_index, target.row, target.col);
        self.reset_qmk_section(section, current_action.as_ref());
    }

    /// Draws the layer key selection page.
    fn draw_qmk_layer_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        search_query: &str,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let layer_count = keyboard.layer_infos().len();
        let groups = super::qmk_catalog::layer_groups(layer_count);
        let action = keyboard.get_action(target.layer_index, target.row, target.col);
        let selected = action.as_ref().map(SelectedKey::valid);
        framed_candidate_groups(
            ui,
            &groups,
            search_query,
            |_| true,
            selected,
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
        search_query: &str,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let is_valid = self.qmk_draft.is_valid();
        let section = self.qmk_draft.section;

        if section.has_layer() {
            let action = self
                .qmk_draft
                .mod_tap_layer
                .and_then(|l| QmkLayerOp::Momentary.encode(l.min(15) as u8))
                .map(KeyAction::Qmk);
            let group = super::qmk_catalog::layer_picker_group(keyboard.layer_infos().len());
            titled_candidate_group(
                ui,
                &group,
                search_query,
                |c| keyboard.is_action_supported(&c.binding),
                action.as_ref().map(|a| SelectedKey::new(a, is_valid)),
                style,
                |candidate| {
                    if let KeyAction::Qmk(code) = &candidate.binding {
                        if let QmkKeycode::LayerOp { layer, .. } = QmkKeycode::from_u16(*code) {
                            self.qmk_draft.mod_tap_layer = Some(layer as usize);
                            self.commit_qmk_draft(keyboard, target);
                        }
                    }
                },
            );
        }

        if section.has_mods() {
            titled_group(ui, "Modifiers", |ui| {
                if modifier_toggle_row(
                    ui,
                    "qmk_mods",
                    &mut self.qmk_draft.mods,
                    &mut self.qmk_draft.right,
                    is_valid,
                    style,
                ) {
                    self.commit_qmk_draft(keyboard, target);
                }
            });
        }

        if section.has_tap_key() {
            let action = (self.qmk_draft.base_code != 0)
                .then_some(self.qmk_draft.base_code)
                .map(KeyAction::Qmk);
            let group = super::qmk_catalog::category(KeycodeCategory::Basic);
            titled_candidate_group(
                ui,
                group,
                search_query,
                |c| keyboard.is_action_supported(&c.binding),
                action.as_ref().map(|a| SelectedKey::new(a, is_valid)),
                style,
                |candidate| {
                    if let KeyAction::Qmk(code) = &candidate.binding {
                        self.qmk_draft.base_code = *code;
                        self.commit_qmk_draft(keyboard, target);
                    }
                },
            );
        }
    }

    /// Applies the current draft keycode to the target key.
    fn commit_qmk_draft(&mut self, keyboard: &Keyboard, target: EditTarget) {
        let staged = self.qmk_draft.staged().map(KeyAction::Qmk);
        self.commit_staged(keyboard, target, staged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_key::LayoutKey;
    use crate::qmk_keycode_labels::constants::*;
    use crate::qmk_keycode_labels::{resolve_qmk_key, KeyResolution};
    use qmk_via_api::QmkLayerOp;

    fn get_layout_key(bytes: u16) -> Option<LayoutKey> {
        match resolve_qmk_key(bytes) {
            KeyResolution::Key(key) => Some(*key),
            _ => None,
        }
    }

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
        assert_eq!(decode(code).mod_tap_layer, Some(2));
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
        assert_eq!(decode(code).mod_tap_layer, Some(3));
        assert_eq!(decode(code).mods, MOD_LSFT | MOD_LCTL);
        assert!(QmkKeycode::encode_layer_mod(16, mods).is_none());
        assert!(QmkKeycode::encode_layer_mod(3, QmkModMask::empty()).is_none());
    }

    #[test]
    fn tap_dance_round_trips() {
        for td in 0..32 {
            let code = QK_TAP_DANCE.start + td;
            let draft = QmkDraft {
                section: Section::Custom,
                ..Default::default()
            };
            assert_round_trips(draft, code);
        }
    }
}
