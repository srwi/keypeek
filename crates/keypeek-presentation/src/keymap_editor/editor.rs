//! Keymap editor body and category rendering.
//!
//! Renders key candidate grids, parameters, and sidebar categories based on [`KeySpec`].

use super::draft::{EditorSection, KeyDraft};
use super::picker::{
    candidate_groups, modifier_toggle_grid, multi_candidate_groups, section_headline,
    CandidateGroup, SelectedKey, GRID_SPACING, SECTION_SPACING,
};
use super::profile::SidebarSection;
use super::{EditTarget, EditorProfile, EditorState};
use crate::key_paint::KeyPaintStyle;
use keypeek_core::{BacklightAction, HidKey, KeySpec, LayerActivation, LightingAction};
use keypeek_protocol::Keyboard;

/// Width of the left category panel in pixels.
const SIDEBAR_WIDTH: f32 = 110.0;
/// Right margin to prevent scrollbar overlap with key grids.
const SCROLLBAR_GUTTER: f32 = 8.0;

struct PageContext<'a> {
    keyboard: &'a Keyboard,
    profile: &'a dyn EditorProfile,
    target: EditTarget,
    search_query: &'a str,
    style: &'a KeyPaintStyle,
}

/// Draws the left category panel with sectioned items and a search bar pinned to the bottom.
fn editor_left_panel(
    ui: &mut egui::Ui,
    left_id: &str,
    is_action_supported: &dyn Fn(&KeySpec) -> bool,
    profile: &dyn EditorProfile,
    current: EditorSection,
    sections: &[SidebarSection<EditorSection>],
    search_query: &mut String,
) -> Option<EditorSection> {
    let mut selected = None;
    let left_id = egui::Id::new(left_id);
    egui::Panel::left(left_id)
        .resizable(false)
        .exact_size(SIDEBAR_WIDTH)
        .show_separator_line(false)
        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
            left: 0,
            right: 8,
            top: 0,
            bottom: 0,
        }))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(2.0);
                super::picker::search_bar(ui, search_query);
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            for section in sections {
                                let supported: Vec<EditorSection> = section
                                    .items
                                    .iter()
                                    .copied()
                                    .filter(|item| {
                                        profile.is_section_supported(*item, is_action_supported)
                                    })
                                    .collect();
                                if supported.is_empty() {
                                    continue;
                                }
                                ui.weak(section.title);
                                for item in supported {
                                    if ui
                                        .selectable_label(
                                            current == item,
                                            profile.section_label(item),
                                        )
                                        .clicked()
                                    {
                                        selected = Some(item);
                                    }
                                }
                                ui.add_space(4.0);
                            }
                        });
                    });
                });
            });
        });
    selected
}

/// The editor's scrolling central panel.
fn editor_central_panel(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    content: impl FnOnce(&mut egui::Ui),
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
            left: 4,
            right: 0,
            top: 0,
            bottom: 0,
        }))
        .show(ui, |ui| {
            ui.push_id(&id_salt, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(&id_salt)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let content_width = (ui.available_width() - SCROLLBAR_GUTTER).max(100.0);
                        ui.set_max_width(content_width);
                        content(ui);
                    });
            });
        });
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
        let is_action_supported = |spec: &KeySpec| keyboard.is_action_supported(spec);

        // If current section is unsupported on this keyboard, switch to first supported
        if !profile.is_section_supported(self.draft.section, &is_action_supported) {
            if let Some(first) = sections
                .iter()
                .flat_map(|s| s.items.iter())
                .copied()
                .find(|s| profile.is_section_supported(*s, &is_action_supported))
            {
                self.draft.section = first;
            }
        }

        let current_section = self.draft.section;
        let mut search_query = std::mem::take(&mut self.search_query);

        if let Some(section) = editor_left_panel(
            ui,
            "editor_sections",
            &is_action_supported,
            profile,
            current_section,
            sections,
            &mut search_query,
        ) {
            search_query.clear();
            let current_action = target.action(keyboard);
            self.draft = KeyDraft::for_section(section, current_action.as_ref());
        }

        let current_section = self.draft.section;
        let ctx = PageContext {
            keyboard,
            profile,
            target,
            search_query: &search_query,
            style,
        };

        editor_central_panel(
            ui,
            (target.layer_index, current_section),
            |ui| match current_section {
                EditorSection::Keyboard => {
                    self.draw_modified_key_page(ui, &ctx, EditorSection::Keyboard, "kb_mods")
                }
                EditorSection::KeyToggle => {
                    self.draw_modified_key_page(ui, &ctx, EditorSection::KeyToggle, "toggle_mods")
                }
                EditorSection::Combo => {
                    self.draw_modified_key_page(ui, &ctx, EditorSection::Combo, "combo_mods")
                }
                EditorSection::ModTap => self.draw_mod_tap_page(ui, &ctx),
                EditorSection::Layers => self.draw_layers_page(ui, &ctx),
                EditorSection::LayerMod => self.draw_layer_mod_page(ui, &ctx),
                EditorSection::OneShot => self.draw_one_shot_page(ui, &ctx),
                EditorSection::Backlight => self.draw_backlight_page(ui, &ctx),
                EditorSection::RawHex => self.draw_raw_hex_page(ui, &ctx),
                _ => {
                    let groups = profile.section_groups(current_section);
                    if !groups.is_empty() {
                        self.draw_candidate_groups_page(ui, &ctx, groups);
                    }
                }
            },
        );

        self.search_query = search_query;
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

    fn draw_mod_grid(
        &mut self,
        ui: &mut egui::Ui,
        id_salt: &str,
        mods: u8,
        valid: bool,
        ctx: &PageContext<'_>,
        mut apply: impl FnMut(&mut KeyDraft, u8),
    ) {
        modifier_toggle_grid(ui, id_salt, mods, valid, ctx.style, |mask| {
            apply(&mut self.draft, mask);
            self.commit_draft(ctx.keyboard, ctx.profile, ctx.target);
        });
    }

    fn draw_tap_candidates(&mut self, ui: &mut egui::Ui, ctx: &PageContext<'_>, toggle: bool) {
        let is_valid = self.draft.is_valid();
        let tap_spec = self.draft.tap_key_spec();
        let selected = tap_spec.as_ref().map(|s| SelectedKey::new(s, is_valid));

        multi_candidate_groups(
            ui,
            ctx.profile.tap_categories(),
            ctx.search_query,
            |c| ctx.keyboard.is_action_supported(&c.binding),
            selected,
            ctx.style,
            |_, candidate| {
                if let KeySpec::KeyPress { key, .. } = &candidate.binding {
                    if toggle && self.draft.tap_key == Some(*key) {
                        self.draft.tap_key = None;
                    } else {
                        self.draft.tap_key = Some(*key);
                    }
                    self.commit_draft(ctx.keyboard, ctx.profile, ctx.target);
                }
            },
        );
    }

    fn draw_modified_key_page(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &PageContext<'_>,
        section: EditorSection,
        id_salt: &str,
    ) {
        let is_valid = self.draft.is_valid();
        section_headline(ui, ctx.profile.section_label(section));
        self.draw_mod_grid(ui, id_salt, self.draft.modifiers, is_valid, ctx, |d, m| {
            d.modifiers ^= m;
        });
        ui.add_space(GRID_SPACING);
        self.draw_tap_candidates(ui, ctx, false);
    }

    fn draw_candidate_groups_page(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &PageContext<'_>,
        groups: &[CandidateGroup],
    ) {
        let current_action = ctx.target.action(ctx.keyboard);
        let selected = current_action.as_ref().map(SelectedKey::valid);
        candidate_groups(
            ui,
            groups,
            ctx.search_query,
            |c| ctx.keyboard.is_action_supported(&c.binding),
            selected,
            ctx.style,
            |_, candidate| {
                self.commit_action(ctx.keyboard, ctx.target, candidate.binding.clone());
            },
        );
    }

    fn draw_tap_key_picker(&mut self, ui: &mut egui::Ui, ctx: &PageContext<'_>) {
        let is_valid = self.draft.is_valid();
        section_headline(ui, "Tap key");
        if ctx.profile.supports_tap_modifiers() {
            self.draw_mod_grid(
                ui,
                "tap_mods",
                self.draft.tap_modifiers,
                is_valid,
                ctx,
                |d, m| d.tap_modifiers ^= m,
            );
            ui.add_space(GRID_SPACING);
        }
        self.draw_tap_candidates(ui, ctx, false);
    }

    fn draw_mod_tap_page(&mut self, ui: &mut egui::Ui, ctx: &PageContext<'_>) {
        let is_valid = self.draft.is_valid();
        section_headline(ui, "Hold modifier");
        self.draw_mod_grid(
            ui,
            "hold_mods",
            self.draft.hold_mods,
            is_valid,
            ctx,
            |d, m| d.hold_mods ^= m,
        );
        ui.add_space(SECTION_SPACING);

        self.draw_tap_key_picker(ui, ctx);
    }

    fn draw_layers_page(&mut self, ui: &mut egui::Ui, ctx: &PageContext<'_>) {
        let layer_names = ctx.keyboard.layer_names();
        let groups = ctx.profile.layer_groups(
            layer_names.len(),
            &layer_names,
            self.draft.layer_tap_target(),
        );

        let current_action = ctx.target.action(ctx.keyboard);
        let selected_action = self.draft.staged().or(current_action);
        let selected = selected_action.as_ref().map(SelectedKey::valid);

        candidate_groups(
            ui,
            &groups,
            ctx.search_query,
            |c| ctx.keyboard.is_action_supported(&c.binding),
            selected,
            ctx.style,
            |_, candidate| match &candidate.binding {
                KeySpec::LayerTap { layer, .. } => {
                    self.draft.is_layer_tap = true;
                    self.draft.target_layer = Some(*layer as usize);
                    self.draft.layer_activation = None;
                    if self.draft.tap_key.is_none() {
                        self.draft.tap_key = Some(HidKey::keyboard(0x2C));
                    }
                    self.commit_draft(ctx.keyboard, ctx.profile, ctx.target);
                }
                KeySpec::Layer { layer, activation } => {
                    self.draft.is_layer_tap = false;
                    self.draft.target_layer = Some(*layer as usize);
                    self.draft.layer_activation = Some(*activation);
                    self.commit_action(ctx.keyboard, ctx.target, candidate.binding.clone());
                }
                _ => {}
            },
        );

        if self.draft.is_layer_tap {
            self.draw_tap_key_picker(ui, ctx);
        }
    }

    fn draw_one_shot_page(&mut self, ui: &mut egui::Ui, ctx: &PageContext<'_>) {
        let is_valid = self.draft.is_valid();
        section_headline(ui, ctx.profile.section_label(EditorSection::OneShot));
        self.draw_mod_grid(
            ui,
            "oneshot_mods",
            self.draft.modifiers,
            is_valid,
            ctx,
            |d, m| d.modifiers ^= m,
        );

        if ctx.profile.supports_oneshot_keys() {
            ui.add_space(GRID_SPACING);
            self.draw_tap_candidates(ui, ctx, true);
        }
    }

    fn draw_backlight_page(&mut self, ui: &mut egui::Ui, ctx: &PageContext<'_>) {
        let groups = ctx.profile.section_groups(EditorSection::Backlight);
        if !groups.is_empty() {
            self.draw_candidate_groups_page(ui, ctx, groups);
            ui.add_space(SECTION_SPACING);
        }

        section_headline(ui, "Brightness Level");
        ui.horizontal(|ui| {
            ui.label("Level:");
            let drag = ui.add(
                egui::DragValue::new(&mut self.draft.backlight_level)
                    .range(0..=255)
                    .speed(1),
            );
            if drag.changed() || ui.button("Set").clicked() {
                let spec = KeySpec::Lighting(LightingAction::Backlight(BacklightAction::Set(
                    self.draft.backlight_level,
                )));
                self.commit_action(ctx.keyboard, ctx.target, spec);
            }
        });
    }

    fn draw_layer_mod_page(&mut self, ui: &mut egui::Ui, ctx: &PageContext<'_>) {
        let is_valid = self.draft.is_valid();
        let layer_names = ctx.keyboard.layer_names();
        let layer_count = layer_names.len().min(16);

        let candidates: Vec<super::picker::Candidate> = (0..layer_count)
            .map(|layer| {
                let mut cand = super::picker::Candidate::from_action(
                    KeySpec::Layer {
                        layer: layer as u8,
                        activation: LayerActivation::Momentary,
                    },
                    ctx.profile.presenter(),
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
        candidate_groups(
            ui,
            std::slice::from_ref(&group),
            ctx.search_query,
            |_| true,
            selected_layer
                .as_ref()
                .map(|s| SelectedKey::new(s, is_valid)),
            ctx.style,
            |_, candidate| {
                if let KeySpec::Layer { layer, .. } = &candidate.binding {
                    self.draft.target_layer = Some(*layer as usize);
                    self.commit_draft(ctx.keyboard, ctx.profile, ctx.target);
                }
            },
        );

        ui.add_space(SECTION_SPACING);
        section_headline(ui, "Modifiers");
        self.draw_mod_grid(
            ui,
            "layermod_mods",
            self.draft.modifiers,
            is_valid,
            ctx,
            |d, m| d.modifiers ^= m,
        );
    }

    fn draw_raw_hex_page(&mut self, ui: &mut egui::Ui, ctx: &PageContext<'_>) {
        section_headline(ui, "Keycode");
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
                self.commit_draft(ctx.keyboard, ctx.profile, ctx.target);
            }
        });
        if u16::from_str_radix(&self.draft.hex, 16).is_err() {
            ui.add_space(4.0);
            ui.weak("Enter a 1–4 digit hex keycode");
        }
    }
}
