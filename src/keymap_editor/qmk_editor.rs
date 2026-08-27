//! QMK/VIA editor content: sections for basic/media pickers, layer keys,
//! modifier combos, and a raw-hex "Any" fallback. Writes apply immediately
//! (VIA behavior — no save button).

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use crate::qmk_keycode_labels::constants::*;
use crate::qmk_keycode_labels::get_layout_key;

use super::picker::{picker_grid, qmk_candidate_text, Candidate};
use super::EditTarget;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Basic,
    Media,
    Layers,
    Mods,
    Any,
}

impl Section {
    const ALL: [Section; 5] = [
        Section::Basic,
        Section::Media,
        Section::Layers,
        Section::Mods,
        Section::Any,
    ];
    fn label(&self) -> &'static str {
        match self {
            Section::Basic => "Basic",
            Section::Media => "Media",
            Section::Layers => "Layers",
            Section::Mods => "Mods",
            Section::Any => "Any",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    Mo,
    Tg,
    To,
    Osl,
    Tt,
    Df,
}

impl LayerKind {
    const ALL: [LayerKind; 6] = [
        LayerKind::Mo,
        LayerKind::Tg,
        LayerKind::To,
        LayerKind::Osl,
        LayerKind::Tt,
        LayerKind::Df,
    ];
    fn label(&self) -> &'static str {
        match self {
            LayerKind::Mo => "MO",
            LayerKind::Tg => "TG",
            LayerKind::To => "TO",
            LayerKind::Osl => "OSL",
            LayerKind::Tt => "TT",
            LayerKind::Df => "DF",
        }
    }
    fn range(&self) -> std::ops::Range<u16> {
        match self {
            LayerKind::Mo => QK_MOMENTARY,
            LayerKind::Tg => QK_TOGGLE_LAYER,
            LayerKind::To => QK_TO,
            LayerKind::Osl => QK_ONE_SHOT_LAYER,
            LayerKind::Tt => QK_LAYER_TAP_TOGGLE,
            LayerKind::Df => QK_DEF_LAYER,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModMode {
    Combo,
    OneShot,
    ModTap,
    LayerTap,
}

impl ModMode {
    const ALL: [ModMode; 4] = [
        ModMode::Combo,
        ModMode::OneShot,
        ModMode::ModTap,
        ModMode::LayerTap,
    ];
    fn label(&self) -> &'static str {
        match self {
            ModMode::Combo => "LSFT(kc) combo",
            ModMode::OneShot => "One-shot mod",
            ModMode::ModTap => "MT(mod, kc)",
            ModMode::LayerTap => "LT(layer, kc)",
        }
    }
}

/// The editor's editable fields, rebuilt on each retarget.
#[derive(Clone)]
pub struct QmkDraft {
    pub section: Section,
    pub layer_kind: LayerKind,
    pub layer: usize,
    pub mod_mode: ModMode,
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
            layer_kind: LayerKind::Mo,
            layer: 0,
            mod_mode: ModMode::Combo,
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

/// `MO/TG/TO/OSL/TT/DF(layer)` → keycode.
fn encode_layer(kind: LayerKind, layer: usize) -> Option<u16> {
    let range = kind.range();
    let layer = layer as u16;
    range
        .contains(&(range.start + layer))
        .then_some(range.start + layer)
}

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
            draft.layer_kind = kind;
            draft.layer = (code - kind.range().start) as usize;
            return draft;
        }
    }

    if QK_MODS.contains(&code) {
        draft.section = Section::Mods;
        draft.mod_mode = ModMode::Combo;
        draft.mods = (code >> 8) & 0x0F;
        draft.right = (code >> 8) & MOD_RIGHT_FLAG != 0;
        draft.base_code = code & 0xFF;
        return draft;
    }
    if QK_MOD_TAP.contains(&code) {
        let remainder = code - QK_MOD_TAP.start;
        let mod_value = (remainder >> 8) & 0x1F;
        draft.section = Section::Mods;
        draft.mod_mode = ModMode::ModTap;
        draft.mods = mod_value & 0x0F;
        draft.right = mod_value & MOD_RIGHT_FLAG != 0;
        draft.base_code = remainder & 0xFF;
        return draft;
    }
    if QK_ONE_SHOT_MOD.contains(&code) {
        let mod_value = (code - QK_ONE_SHOT_MOD.start) & 0x1F;
        draft.section = Section::Mods;
        draft.mod_mode = ModMode::OneShot;
        draft.mods = mod_value & 0x0F;
        draft.right = mod_value & MOD_RIGHT_FLAG != 0;
        return draft;
    }
    if QK_LAYER_TAP.contains(&code) {
        let remainder = code - QK_LAYER_TAP.start;
        draft.section = Section::Mods;
        draft.mod_mode = ModMode::LayerTap;
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
        egui::ComboBox::from_id_salt("qmk_section_combo")
            .selected_text(draft.section.label())
            .show_ui(ui, |ui| {
                for section in Section::ALL {
                    ui.selectable_value(&mut draft.section, section, section.label());
                }
            });
        ui.separator();

        match draft.section {
            Section::Basic | Section::Media => {
                let codes = super::qmk_catalog::categories()
                    .into_iter()
                    .find(|c| {
                        (draft.section == Section::Basic && c.name == "Basic")
                            || (draft.section == Section::Media && c.name == "Media")
                    })
                    .map(|c| c.codes)
                    .unwrap_or_default();
                let candidates: Vec<Candidate> = codes
                    .iter()
                    .map(|&code| Candidate {
                        code: code as u32,
                        text: qmk_candidate_text(code),
                    })
                    .collect();
                let selected = self
                    .editor
                    .target
                    .and_then(|t| keyboard.get_action(t.layer_index, t.row, t.col))
                    .and_then(|a| match a {
                        KeyAction::Qmk(code) if !has_params(code) => Some(code as u32),
                        _ => None,
                    });
                let salt = if draft.section == Section::Basic {
                    "qmk_basic"
                } else {
                    "qmk_media"
                };
                picker_grid(ui, salt, &candidates, selected, |code| {
                    self.apply_qmk_write(keyboard, target, code as u16);
                });
            }
            Section::Layers => {
                ui.horizontal(|ui| {
                    ui.label("Kind");
                    egui::ComboBox::from_id_salt("layer_kind_combo")
                        .selected_text(draft.layer_kind.label())
                        .show_ui(ui, |ui| {
                            for kind in LayerKind::ALL {
                                ui.selectable_value(&mut draft.layer_kind, kind, kind.label());
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Layer");
                    ui.add(egui::DragValue::new(&mut draft.layer).range(0..=31));
                });
                if ui.button("Apply").clicked() {
                    if let Some(code) = encode_layer(draft.layer_kind, draft.layer) {
                        self.apply_qmk_write(keyboard, target, code);
                    }
                }
            }
            Section::Mods => self.draw_mods_section(ui, keyboard, target, draft),
            Section::Any => {
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
                let code = u16::from_str_radix(&draft.hex, 16);
                match code {
                    Ok(code) => {
                        let label = get_layout_key(code)
                            .map(|k| k.tap.full.clone())
                            .unwrap_or_else(|| format!("0x{code:04X}"));
                        ui.weak(format!("Preview: {label}"));
                        if ui.button("Apply").clicked() {
                            self.apply_qmk_write(keyboard, target, code);
                        }
                    }
                    Err(_) => {
                        ui.weak("Enter a 1–4 digit hex keycode");
                    }
                }
            }
        }
    }

    fn draw_mods_section(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut QmkDraft,
    ) {
        egui::ComboBox::from_id_salt("mod_mode_combo")
            .selected_text(draft.mod_mode.label())
            .show_ui(ui, |ui| {
                for mode in ModMode::ALL {
                    ui.selectable_value(&mut draft.mod_mode, mode, mode.label());
                }
            });

        ui.horizontal(|ui| {
            let mut ctrl = draft.mods & MOD_LCTL != 0;
            let mut shift = draft.mods & MOD_LSFT != 0;
            let mut alt = draft.mods & MOD_LALT != 0;
            let mut gui = draft.mods & MOD_LGUI != 0;
            if ui.checkbox(&mut ctrl, "Ctrl").changed()
                || ui.checkbox(&mut shift, "Shift").changed()
                || ui.checkbox(&mut alt, "Alt").changed()
                || ui.checkbox(&mut gui, "Gui").changed()
            {
                draft.mods = 0;
                draft.mods |= if ctrl { MOD_LCTL } else { 0 };
                draft.mods |= if shift { MOD_LSFT } else { 0 };
                draft.mods |= if alt { MOD_LALT } else { 0 };
                draft.mods |= if gui { MOD_LGUI } else { 0 };
            }
            if draft.mod_mode == ModMode::Combo || draft.mod_mode == ModMode::ModTap {
                ui.checkbox(&mut draft.right, "Right");
            }
        });

        match draft.mod_mode {
            ModMode::Combo | ModMode::ModTap => {
                ui.label("Tap/base key (8-bit basic only):");
                self.draw_base_picker(ui, draft);
            }
            ModMode::LayerTap => {
                ui.horizontal(|ui| {
                    ui.label("Layer (0–15)");
                    ui.add(egui::DragValue::new(&mut draft.mod_tap_layer).range(0..=15));
                });
                ui.label("Tap key (8-bit basic only):");
                self.draw_base_picker(ui, draft);
            }
            ModMode::OneShot => {}
        }

        let code = match draft.mod_mode {
            ModMode::Combo => encode_combo(draft.mod_value(), draft.base_code),
            ModMode::OneShot => encode_one_shot_mod(draft.mod_value()),
            ModMode::ModTap => encode_mod_tap(draft.mod_value(), draft.base_code),
            ModMode::LayerTap => encode_layer_tap(draft.mod_tap_layer, draft.base_code),
        };

        if ui.button("Apply").clicked() {
            if let Some(code) = code {
                self.apply_qmk_write(keyboard, target, code);
            }
        }
    }

    fn draw_base_picker(&mut self, ui: &mut egui::Ui, draft: &mut QmkDraft) {
        let candidates: Vec<Candidate> = (0x00u16..=0xFF)
            .filter(|&code| get_layout_key(code).is_some())
            .map(|code| Candidate {
                code: code as u32,
                text: qmk_candidate_text(code),
            })
            .collect();
        picker_grid(
            ui,
            "qmk_base",
            &candidates,
            Some(draft.base_code as u32),
            |code| {
                draft.base_code = code as u16;
            },
        );
    }

    fn apply_qmk_write(&mut self, keyboard: &Keyboard, target: EditTarget, code: u16) {
        if self.editor.pending.is_some() {
            return;
        }
        let receiver = keyboard.set_key(
            target.layer_index,
            target.row,
            target.col,
            KeyAction::Qmk(code),
        );
        self.editor.pending = Some(receiver);
        self.editor.error = None;
    }
}

/// Whether a keycode carries parameters (layer/mod logic) and so is better
/// edited in its dedicated section than highlighted in the plain picker.
fn has_params(code: u16) -> bool {
    QK_MODS.contains(&code)
        || QK_MOD_TAP.contains(&code)
        || QK_LAYER_TAP.contains(&code)
        || QK_ONE_SHOT_MOD.contains(&code)
        || LayerKind::ALL.iter().any(|k| k.range().contains(&code))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            draft.layer_kind = kind;
            assert_round_trips(draft, code);
        }
        assert!(encode_layer(LayerKind::Mo, 32).is_none());
    }

    #[test]
    fn mod_combo_round_trips() {
        let code = encode_combo(MOD_LSFT, 0x2A).unwrap(); // LSFT(Backspace)
        assert_eq!(code, (MOD_LSFT << 8) | 0x2A);
        let mut draft = QmkDraft::default();
        draft.section = Section::Mods;
        draft.mod_mode = ModMode::Combo;
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
        draft.section = Section::Mods;
        draft.mod_mode = ModMode::OneShot;
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mods, MOD_LCTL | MOD_LSFT);
        assert!(encode_one_shot_mod(0).is_none());
    }

    #[test]
    fn mod_tap_round_trips() {
        let code = encode_mod_tap(MOD_LSFT | MOD_LALT, 0x04).unwrap(); // MT(LSFT|LALT, A)
        let mut draft = QmkDraft::default();
        draft.section = Section::Mods;
        draft.mod_mode = ModMode::ModTap;
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
        draft.section = Section::Mods;
        draft.mod_mode = ModMode::LayerTap;
        assert_round_trips(draft, code);
        assert_eq!(decode(code).mod_tap_layer, 2);
        assert_eq!(decode(code).base_code, 0x1C);
        assert!(encode_layer_tap(16, 0x1C).is_none());
        assert!(encode_layer_tap(2, 0x100).is_none());
    }
}
