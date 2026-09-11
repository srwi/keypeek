//! Unified keymap editor central panel and category rendering.
//!
//! Draws key candidate grids, parameter controls, and sidebar categories
//! driven directly by domain [`KeySpec`] capabilities.

use super::draft::{EditorSection, KeyDraft};
use super::picker::{
    framed_candidate_groups, modifier_toggle_grid, multi_candidate_groups, titled_candidate_group,
    CandidateGroup, SelectedKey,
};
use super::{EditTarget, EditorProfile, EditorState};
use crate::hid_labels::Modifiers;
use crate::key_paint::KeyPaintStyle;
use crate::key_spec::{HidKey, KeySpec, LayerActivation};
use crate::keyboard::Keyboard;
use crate::ui_widgets::titled_group;

struct TapPickerOpts<'a> {
    id_salt: &'static str,
    supports_tap_mods: bool,
    search_query: &'a str,
}

impl EditorState {
    /// Draws the unified keymap editor body (sidebar + central panel).
    pub(super) fn draw_editor_body(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        style: &KeyPaintStyle,
    ) {
        let sections = profile.sidebar_sections();

        // If current section is unsupported on this keyboard, switch to first supported
        if !profile.is_section_supported(self.draft.section, keyboard)
            || !sections
                .iter()
                .any(|s| s.items.contains(&self.draft.section))
        {
            if let Some(first) = sections
                .iter()
                .flat_map(|s| s.items.iter())
                .copied()
                .find(|s| profile.is_section_supported(*s, keyboard))
            {
                self.draft.section = first;
            }
        }

        let current_section = self.draft.section;
        if let Some(section) = super::editor_left_panel(
            ui,
            "editor_sections",
            keyboard,
            profile,
            current_section,
            sections,
            &mut self.search_query,
        ) {
            self.search_query.clear();
            let current_action = target.action(keyboard);
            self.draft = KeyDraft::for_section(section, current_action.as_ref());
        }

        let current_section = self.draft.section;
        let is_valid = self.draft.is_valid();
        let search_query = self.search_query.clone();

        super::editor_central_panel(ui, (target.layer_index, current_section), |ui| {
            match current_section {
                EditorSection::Keyboard => {
                    self.draw_keyboard_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::KeyToggle => {
                    self.draw_key_toggle_page(ui, keyboard, profile, target, &search_query, is_valid, style);
                }
                EditorSection::Combo => {
                    self.draw_combo_page(ui, keyboard, profile, target, &search_query, is_valid, style);
                }
                EditorSection::ModTap => {
                    self.draw_mod_tap_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::Layers => {
                    self.draw_layers_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::LayerMod => {
                    self.draw_layer_mod_page(ui, keyboard, target, &search_query, is_valid, style);
                }
                EditorSection::OneShot => {
                    self.draw_one_shot_page(ui, keyboard, profile, target, &search_query, is_valid, style);
                }
                EditorSection::Backlight => {
                    self.draw_backlight_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::RawHex => {
                    self.draw_raw_hex_page(ui, keyboard, target);
                }
                _ => {
                    let groups = profile.section_groups(current_section);
                    if groups.len() == 1 {
                        self.draw_single_group_page(
                            ui,
                            keyboard,
                            target,
                            &groups[0],
                            &search_query,
                            style,
                        );
                    } else if !groups.is_empty() {
                        self.draw_framed_groups_page(
                            ui,
                            keyboard,
                            target,
                            groups,
                            &search_query,
                            style,
                        );
                    }
                }
            }
        });
    }

    fn commit_draft(&mut self, keyboard: &Keyboard, target: EditTarget) {
        let staged = self.draft.staged_for(keyboard);
        self.commit_staged(keyboard, target, staged);
    }

    fn draw_keyboard_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        search_query: &str,
        style: &KeyPaintStyle,
    ) {
        titled_group(ui, "Modifiers", |ui| {
            modifier_toggle_grid(ui, "kb_mods", self.draft.modifiers, true, style, |mask| {
                self.draft.modifiers ^= mask;
                self.commit_draft(keyboard, target);
            });
        });

        let tap_spec = self.draft.tap_key.map(|key| KeySpec::KeyPress {
            key,
            modifiers: Modifiers::default(),
        });
        let action = target.action(keyboard);
        let selected = tap_spec
            .as_ref()
            .or(action
                .as_ref()
                .filter(|a| matches!(a, KeySpec::KeyPress { .. })))
            .map(SelectedKey::valid);

        multi_candidate_groups(
            ui,
            profile.tap_categories(),
            search_query,
            |c| keyboard.is_action_supported(&c.binding),
            selected,
            style,
            |_, candidate| {
                if let KeySpec::KeyPress { key, .. } = &candidate.binding {
                    self.draft.tap_key = Some(*key);
                    self.commit_draft(keyboard, target);
                }
            },
        );
    }

    fn draw_single_group_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        group: &CandidateGroup,
        search_query: &str,
        style: &KeyPaintStyle,
    ) {
        let action = target.action(keyboard);
        titled_candidate_group(
            ui,
            group,
            search_query,
            |c| keyboard.is_action_supported(&c.binding),
            action.as_ref().map(SelectedKey::valid),
            style,
            |candidate| {
                self.apply_write(keyboard, target, candidate.binding.clone());
            },
        );
    }

    fn draw_framed_groups_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        groups: &[CandidateGroup],
        search_query: &str,
        style: &KeyPaintStyle,
    ) {
        let action = target.action(keyboard);
        let selected = action.as_ref().map(SelectedKey::valid);
        framed_candidate_groups(
            ui,
            groups,
            search_query,
            |c| keyboard.is_action_supported(&c.binding),
            selected,
            style,
            |_, candidate| {
                self.apply_write(keyboard, target, candidate.binding.clone());
            },
        );
    }

    fn draw_tap_key_picker(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        opts: TapPickerOpts<'_>,
        style: &KeyPaintStyle,
    ) {
        let is_valid = self.draft.is_valid();
        titled_group(ui, "Tap key", |ui| {
            if opts.supports_tap_mods {
                modifier_toggle_grid(
                    ui,
                    opts.id_salt,
                    self.draft.tap_modifiers,
                    is_valid,
                    style,
                    |mask| {
                        self.draft.tap_modifiers ^= mask;
                        self.commit_draft(keyboard, target);
                    },
                );
            }

            let tap_spec = self.draft.tap_key.map(|key| KeySpec::KeyPress {
                key,
                modifiers: Modifiers::default(),
            });
            let selected = tap_spec.as_ref().map(|s| SelectedKey::new(s, is_valid));

            multi_candidate_groups(
                ui,
                profile.tap_categories(),
                opts.search_query,
                |c| keyboard.is_action_supported(&c.binding),
                selected,
                style,
                |_, candidate| {
                    if let KeySpec::KeyPress { key, .. } = &candidate.binding {
                        self.draft.tap_key = Some(*key);
                        self.commit_draft(keyboard, target);
                    }
                },
            );
        });
    }

    fn draw_combo_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        search_query: &str,
        is_valid: bool,
        style: &KeyPaintStyle,
    ) {
        titled_group(ui, "Modifiers", |ui| {
            modifier_toggle_grid(
                ui,
                "combo_mods",
                self.draft.modifiers,
                is_valid,
                style,
                |mask| {
                    self.draft.modifiers ^= mask;
                    self.commit_draft(keyboard, target);
                },
            );
        });

        let tap_spec = self.draft.tap_key.map(|key| KeySpec::KeyPress {
            key,
            modifiers: crate::hid_labels::Modifiers::default(),
        });
        let selected = tap_spec.as_ref().map(|s| SelectedKey::new(s, is_valid));

        titled_candidate_group(
            ui,
            profile.keyboard_group(),
            search_query,
            |c| keyboard.is_action_supported(&c.binding),
            selected,
            style,
            |candidate| {
                if let KeySpec::KeyPress { key, .. } = &candidate.binding {
                    self.draft.tap_key = Some(*key);
                    self.commit_draft(keyboard, target);
                }
            },
        );

        if self.draft.modifiers == 0 {
            ui.weak("Select at least one modifier.");
        }
    }

    fn draw_mod_tap_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        search_query: &str,
        style: &KeyPaintStyle,
    ) {
        let is_valid = self.draft.is_valid();
        titled_group(ui, "Hold modifier", |ui| {
            modifier_toggle_grid(
                ui,
                "hold_mods",
                self.draft.hold_mods,
                is_valid,
                style,
                |mask| {
                    self.draft.hold_mods ^= mask;
                    self.commit_draft(keyboard, target);
                },
            );
        });

        let supports_tap_mods = keyboard.is_action_supported(&KeySpec::ModTap {
            hold: Modifiers {
                shift: true,
                ..Default::default()
            },
            tap: HidKey::keyboard(0x04),
            tap_modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
        });
        self.draw_tap_key_picker(
            ui,
            keyboard,
            profile,
            target,
            TapPickerOpts {
                id_salt: "mt_tap_mods",
                supports_tap_mods,
                search_query,
            },
            style,
        );

        if self.draft.hold_mods == 0 {
            ui.weak("Select a hold modifier.");
        }
    }

    fn draw_layers_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        search_query: &str,
        style: &KeyPaintStyle,
    ) {
        let layer_infos = keyboard.layer_infos();
        let layer_names: Vec<String> = layer_infos
            .iter()
            .map(|l| l.name.clone().unwrap_or_default())
            .collect();
        let groups = profile.layer_groups(
            layer_infos.len(),
            &layer_infos,
            &layer_names,
            self.draft.tap_key,
        );

        let current_action = target.action(keyboard);
        let selected_action = self.draft.staged().or(current_action);
        let selected = selected_action.as_ref().map(SelectedKey::valid);

        framed_candidate_groups(
            ui,
            &groups,
            search_query,
            |c| keyboard.is_action_supported(&c.binding),
            selected,
            style,
            |_, candidate| match &candidate.binding {
                KeySpec::LayerTap { layer, .. } => {
                    self.draft.is_layer_tap = true;
                    self.draft.target_layer = Some(*layer as usize);
                    self.draft.layer_activation = None;
                    if self.draft.tap_key.is_none() {
                        self.draft.tap_key = Some(HidKey::keyboard(0x2C));
                    }
                    self.commit_draft(keyboard, target);
                }
                KeySpec::Layer { layer, activation } => {
                    self.draft.is_layer_tap = false;
                    self.draft.target_layer = Some(*layer as usize);
                    self.draft.layer_activation = Some(*activation);
                    self.apply_write(keyboard, target, candidate.binding.clone());
                }
                _ => {}
            },
        );

        if self.draft.is_layer_tap {
            let supports_tap_mods = keyboard.is_action_supported(&KeySpec::LayerTap {
                layer: 0,
                tap: HidKey::keyboard(0x04),
                tap_modifiers: Modifiers {
                    shift: true,
                    ..Default::default()
                },
            });
            self.draw_tap_key_picker(
                ui,
                keyboard,
                profile,
                target,
                TapPickerOpts {
                    id_salt: "lt_tap_mods",
                    supports_tap_mods,
                    search_query,
                },
                style,
            );
        }
    }

    fn draw_one_shot_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        search_query: &str,
        is_valid: bool,
        style: &KeyPaintStyle,
    ) {
        titled_group(ui, "Sticky Modifier", |ui| {
            modifier_toggle_grid(
                ui,
                "oneshot_mods",
                self.draft.modifiers,
                is_valid,
                style,
                |mask| {
                    self.draft.modifiers ^= mask;
                    self.commit_draft(keyboard, target);
                },
            );
        });

        let tap_spec = self.draft.tap_key.map(|key| KeySpec::KeyPress {
            key,
            modifiers: crate::hid_labels::Modifiers::default(),
        });
        let selected = tap_spec.as_ref().map(|s| SelectedKey::new(s, is_valid));

        let candidate_filter = |c: &super::picker::Candidate| {
            if let KeySpec::KeyPress { key, .. } = &c.binding {
                let sample = KeySpec::StickyKey {
                    key: Some(*key),
                    modifiers: Modifiers::default(),
                };
                keyboard.is_action_supported(&sample)
            } else {
                false
            }
        };

        multi_candidate_groups(
            ui,
            profile.tap_categories(),
            search_query,
            candidate_filter,
            selected,
            style,
            |_, candidate| {
                if let KeySpec::KeyPress { key, .. } = &candidate.binding {
                    if self.draft.tap_key == Some(*key) {
                        self.draft.tap_key = None;
                    } else {
                        self.draft.tap_key = Some(*key);
                    }
                    self.commit_draft(keyboard, target);
                }
            },
        );
    }

    fn draw_backlight_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        search_query: &str,
        style: &KeyPaintStyle,
    ) {
        let groups = profile.section_groups(EditorSection::Backlight);
        if let Some(bl_group) = groups.first() {
            self.draw_single_group_page(ui, keyboard, target, bl_group, search_query, style);
        }

        titled_group(ui, "Brightness Level", |ui| {
            ui.horizontal(|ui| {
                ui.label("Level:");
                let drag = ui.add(
                    egui::DragValue::new(&mut self.draft.backlight.value)
                        .range(0..=255)
                        .speed(1),
                );
                if drag.changed() || ui.button("Set").clicked() {
                    let spec = KeySpec::Lighting(crate::key_spec::LightingAction::Backlight(
                        crate::key_spec::BacklightAction::Set(self.draft.backlight.value),
                    ));
                    self.apply_write(keyboard, target, spec);
                }
            });
        });
    }

    fn draw_key_toggle_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        search_query: &str,
        is_valid: bool,
        style: &KeyPaintStyle,
    ) {
        titled_group(ui, "Modifiers", |ui| {
            modifier_toggle_grid(
                ui,
                "toggle_mods",
                self.draft.modifiers,
                is_valid,
                style,
                |mask| {
                    self.draft.modifiers ^= mask;
                    self.commit_draft(keyboard, target);
                },
            );
        });

        let tap_spec = self.draft.tap_key.map(|key| KeySpec::KeyPress {
            key,
            modifiers: Modifiers::default(),
        });
        let action = target.action(keyboard);
        let selected = tap_spec
            .as_ref()
            .or(action
                .as_ref()
                .filter(|a| matches!(a, KeySpec::KeyToggle { .. })))
            .map(SelectedKey::valid);

        let candidate_filter = |c: &super::picker::Candidate| match &c.binding {
            KeySpec::KeyPress { key, modifiers } => {
                keyboard.is_action_supported(&KeySpec::KeyToggle {
                    key: *key,
                    modifiers: *modifiers,
                })
            }
            _ => false,
        };

        multi_candidate_groups(
            ui,
            profile.tap_categories(),
            search_query,
            candidate_filter,
            selected,
            style,
            |_, candidate| {
                if let KeySpec::KeyPress { key, .. } = &candidate.binding {
                    self.draft.tap_key = Some(*key);
                    self.commit_draft(keyboard, target);
                }
            },
        );
    }

    fn draw_layer_mod_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        search_query: &str,
        is_valid: bool,
        style: &KeyPaintStyle,
    ) {
        let layer_infos = keyboard.layer_infos();
        let layer_count = layer_infos.len().min(16);
        let layer_names: Vec<String> = layer_infos
            .iter()
            .map(|l| l.name.clone().unwrap_or_default())
            .collect();

        let candidates: Vec<super::picker::Candidate> = (0..layer_count)
            .map(|layer| {
                let mut cand = super::picker::Candidate::from_action(
                    KeySpec::Layer {
                        layer: layer as u8,
                        activation: LayerActivation::Momentary,
                    },
                    &layer_names,
                );
                cand = cand.with_search_token(format!("l{layer}"));
                cand
            })
            .collect();
        let group = super::picker::CandidateGroup {
            name: "Target Layer",
            candidates,
        };
        let selected_layer = self.draft.target_layer.map(|layer| KeySpec::Layer {
            layer: layer as u8,
            activation: LayerActivation::Momentary,
        });
        titled_candidate_group(
            ui,
            &group,
            search_query,
            |_| true,
            selected_layer.as_ref().map(SelectedKey::valid),
            style,
            |candidate| {
                if let KeySpec::Layer { layer, .. } = &candidate.binding {
                    self.draft.target_layer = Some(*layer as usize);
                    self.commit_draft(keyboard, target);
                }
            },
        );

        titled_group(ui, "Modifiers", |ui| {
            modifier_toggle_grid(
                ui,
                "layermod_mods",
                self.draft.modifiers,
                is_valid,
                style,
                |mask| {
                    self.draft.modifiers ^= mask;
                    self.commit_draft(keyboard, target);
                },
            );
        });

        if self.draft.modifiers == 0 {
            ui.weak("Select at least one modifier.");
        }
    }

    fn draw_raw_hex_page(&mut self, ui: &mut egui::Ui, keyboard: &Keyboard, target: EditTarget) {
        titled_group(ui, "Keycode", |ui| {
            ui.horizontal(|ui| {
                ui.label("0x");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.draft.hex)
                        .desired_width(80.0)
                        .char_limit(4),
                );
                if response.changed() {
                    self.draft.hex.retain(|c| c.is_ascii_hexdigit());
                }
                if response.lost_focus() || (response.changed() && self.draft.hex.len() == 4) {
                    self.commit_draft(keyboard, target);
                }
            });
            if u16::from_str_radix(&self.draft.hex, 16).is_err() {
                ui.weak("Enter a 1–4 digit hex keycode");
            }
        });
    }
}
