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
use crate::application::Keyboard;
use crate::key_paint::KeyPaintStyle;
use crate::key_spec::{HidKey, KeySpec, LayerActivation};
use crate::ui_widgets::titled_group;

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
        let search_query = self.search_query.clone();

        super::editor_central_panel(ui, (target.layer_index, current_section), |ui| {
            match current_section {
                EditorSection::Keyboard => {
                    self.draw_keyboard_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::KeyToggle => {
                    self.draw_key_toggle_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::Combo => {
                    self.draw_combo_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::ModTap => {
                    self.draw_mod_tap_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::Layers => {
                    self.draw_layers_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::LayerMod => {
                    self.draw_layer_mod_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::OneShot => {
                    self.draw_one_shot_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::Backlight => {
                    self.draw_backlight_page(ui, keyboard, profile, target, &search_query, style);
                }
                EditorSection::RawHex => {
                    self.draw_raw_hex_page(ui, keyboard, profile, target);
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

    fn commit_draft(
        &mut self,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
    ) {
        let staged = self.draft.staged_for(profile);
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
        titled_group(ui, profile.section_label(EditorSection::Keyboard), |ui| {
            modifier_toggle_grid(ui, "kb_mods", self.draft.modifiers, true, style, |mask| {
                self.draft.modifiers ^= mask;
                self.commit_draft(keyboard, profile, target);
            });

            let tap_spec = self.draft.tap_key_spec();
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
                        self.commit_draft(keyboard, profile, target);
                    }
                },
            );
        });
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
        search_query: &str,
        style: &KeyPaintStyle,
    ) {
        let is_valid = self.draft.is_valid();
        titled_group(ui, "Tap key", |ui| {
            if profile.supports_tap_modifiers() {
                modifier_toggle_grid(
                    ui,
                    "tap_mods",
                    self.draft.tap_modifiers,
                    is_valid,
                    style,
                    |mask| {
                        self.draft.tap_modifiers ^= mask;
                        self.commit_draft(keyboard, profile, target);
                    },
                );
            }

            let tap_spec = self.draft.tap_key_spec();
            let selected = tap_spec.as_ref().map(|s| SelectedKey::new(s, is_valid));

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
                        self.commit_draft(keyboard, profile, target);
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
        style: &KeyPaintStyle,
    ) {
        let is_valid = self.draft.is_valid();
        titled_group(ui, profile.section_label(EditorSection::Combo), |ui| {
            modifier_toggle_grid(
                ui,
                "combo_mods",
                self.draft.modifiers,
                is_valid,
                style,
                |mask| {
                    self.draft.modifiers ^= mask;
                    self.commit_draft(keyboard, profile, target);
                },
            );

            let tap_spec = self.draft.tap_key_spec();
            let selected = tap_spec.as_ref().map(|s| SelectedKey::new(s, is_valid));

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
                        self.commit_draft(keyboard, profile, target);
                    }
                },
            );

            if self.draft.modifiers == 0 {
                ui.weak("Select at least one modifier.");
            }
        });
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
                    self.commit_draft(keyboard, profile, target);
                },
            );
        });

        self.draw_tap_key_picker(
            ui,
            keyboard,
            profile,
            target,
            search_query,
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
        let layer_names = keyboard.layer_names();
        let groups = profile.layer_groups(
            layer_names.len(),
            &layer_names,
            self.draft.layer_tap_target(),
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
                    self.commit_draft(keyboard, profile, target);
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
            self.draw_tap_key_picker(
                ui,
                keyboard,
                profile,
                target,
                search_query,
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
        style: &KeyPaintStyle,
    ) {
        let is_valid = self.draft.is_valid();
        titled_group(ui, profile.section_label(EditorSection::OneShot), |ui| {
            modifier_toggle_grid(
                ui,
                "oneshot_mods",
                self.draft.modifiers,
                is_valid,
                style,
                |mask| {
                    self.draft.modifiers ^= mask;
                    self.commit_draft(keyboard, profile, target);
                },
            );

            if profile.supports_oneshot_keys() {
                let tap_spec = self.draft.tap_key_spec();
                let selected = tap_spec.as_ref().map(|s| SelectedKey::new(s, is_valid));

                multi_candidate_groups(
                    ui,
                    profile.tap_categories(),
                    search_query,
                    |c| keyboard.is_action_supported(&c.binding),
                    selected,
                    style,
                    |_, candidate| {
                        if let KeySpec::KeyPress { key, .. } = &candidate.binding {
                            if self.draft.tap_key == Some(*key) {
                                self.draft.tap_key = None;
                            } else {
                                self.draft.tap_key = Some(*key);
                            }
                            self.commit_draft(keyboard, profile, target);
                        }
                    },
                );
            } else if self.draft.modifiers == 0 {
                ui.weak("Select at least one modifier.");
            }
        });
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
        style: &KeyPaintStyle,
    ) {
        let is_valid = self.draft.is_valid();
        titled_group(ui, profile.section_label(EditorSection::KeyToggle), |ui| {
            modifier_toggle_grid(
                ui,
                "toggle_mods",
                self.draft.modifiers,
                is_valid,
                style,
                |mask| {
                    self.draft.modifiers ^= mask;
                    self.commit_draft(keyboard, profile, target);
                },
            );

            let tap_spec = self.draft.tap_key_spec();
            let action = target.action(keyboard);
            let selected = tap_spec
                .as_ref()
                .or(action
                    .as_ref()
                    .filter(|a| matches!(a, KeySpec::KeyToggle { .. })))
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
                        self.commit_draft(keyboard, profile, target);
                    }
                },
            );
        });
    }

    fn draw_layer_mod_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
        search_query: &str,
        style: &KeyPaintStyle,
    ) {
        let is_valid = self.draft.is_valid();
        let layer_names = keyboard.layer_names();
        let layer_count = layer_names.len().min(16);

        let candidates: Vec<super::picker::Candidate> = (0..layer_count)
            .map(|layer| {
                let mut cand = super::picker::Candidate::from_action(
                    KeySpec::Layer {
                        layer: layer as u8,
                        activation: LayerActivation::Momentary,
                    },
                    profile.presenter(),
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
                    self.commit_draft(keyboard, profile, target);
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
                    self.commit_draft(keyboard, profile, target);
                },
            );
        });

        if self.draft.modifiers == 0 {
            ui.weak("Select at least one modifier.");
        }
    }

    fn draw_raw_hex_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
        target: EditTarget,
    ) {
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
                    self.commit_draft(keyboard, profile, target);
                }
            });
            if u16::from_str_radix(&self.draft.hex, 16).is_err() {
                ui.weak("Enter a 1–4 digit hex keycode");
            }
        });
    }
}
