//! ZMK editor content: behavior-based editing with per-behavior parameters,
//! plus the session Save/Discard flow.

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use zmk_studio_api::{Behavior, HidUsage, HID_USAGE_KEYBOARD};

use super::picker::{
    candidate_groups_rows, modifier_select_grid, modifier_toggle_row, picker_grid_rows, Candidate,
    KEY_UNIT, MOD_KEY_UNIT,
};
use super::zmk_catalog::{self, ZmkBehaviorKind};
use super::{EditTarget, PendingKind};

/// The editor's pages: how the left panel groups behavior kinds and which
/// pane the central panel shows. [`Page::kinds`] is the single source for the
/// groupings — the panel, the pane dispatch, and the Apply rule all derive
/// from it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Keys,
    Layers,
    Mods,
    Special,
    Commands,
}

impl Page {
    /// Panel order: one selectable entry per kind except the grouped pages.
    const ALL: [Page; 5] = [
        Page::Keys,
        Page::Layers,
        Page::Mods,
        Page::Commands,
        Page::Special,
    ];

    fn label(self) -> &'static str {
        match self {
            Page::Keys => "Keys",
            Page::Layers => "Layers",
            Page::Mods => "Mods",
            Page::Special => "Special",
            Page::Commands => "Commands",
        }
    }

    /// The kinds sharing this page. For [`Page::Layers`] this is also the
    /// grid-group order the layer page's `on_select` indexes into.
    fn kinds(self) -> &'static [ZmkBehaviorKind] {
        use ZmkBehaviorKind::*;
        match self {
            Page::Keys => &[KeyPress, KeyToggle, StickyKey],
            Page::Layers => &[MomentaryLayer, ToggleLayer, ToLayer, StickyLayer, LayerTap],
            Page::Mods => &[ModTap],
            Page::Special => &[
                Transparent,
                NoneBehavior,
                CapsWord,
                KeyRepeat,
                GraveEscape,
                StudioUnlock,
                Reset,
                Bootloader,
                SoftOff,
            ],
            Page::Commands => &[
                Bluetooth,
                OutputSelection,
                Backlight,
                Underglow,
                MouseKeyPress,
                MouseMove,
                MouseScroll,
            ],
        }
    }
}

/// The page a behavior kind is edited on; drives the left panel's selection
/// highlight and the central pane dispatch.
fn page_of(kind: ZmkBehaviorKind) -> Page {
    Page::ALL
        .into_iter()
        .find(|page| page.kinds().contains(&kind))
        .expect("every behavior kind belongs to a page")
}

/// Kinds staged in the draft get an Apply button; every other kind applies
/// directly on click from its page's key grids.
fn needs_params(kind: ZmkBehaviorKind) -> bool {
    matches!(page_of(kind), Page::Keys | Page::Mods)
}

/// The editor's editable fields for ZMK, rebuilt on each retarget.
#[derive(Clone)]
pub struct ZmkDraft {
    pub kind: ZmkBehaviorKind,
    /// Base key usage selected in a picker (key press/toggle/sticky, layer-tap
    /// and mod-tap tap side).
    pub usage: HidUsage,
    /// Keyboard modifiers OR'd onto `usage` for a key press.
    pub modifiers: u8,
    /// Hold modifier usage for a Mod-Tap.
    pub hold_mod: HidUsage,
    /// Layer a layer behavior or layer-tap targets (the stable layer id).
    pub layer_id: u32,
    /// Backlight `Set` level: a value that is part of the binding rather than
    /// a choice between keys, so it stays staged in the draft.
    pub bl_value: u32,
}

impl Default for ZmkDraft {
    fn default() -> Self {
        Self {
            kind: ZmkBehaviorKind::KeyPress,
            usage: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0),
            modifiers: 0,
            hold_mod: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0xE1, 0),
            layer_id: 0,
            bl_value: 0,
        }
    }
}

impl ZmkDraft {
    /// Pre-fills the draft from an existing ZMK behavior.
    pub fn from_behavior(behavior: &Behavior) -> Self {
        let mut draft = ZmkDraft::default();
        use ZmkBehaviorKind as K;
        draft.kind = match behavior {
            Behavior::KeyPress(usage) => {
                draft.usage = usage.base();
                draft.modifiers = usage.modifiers();
                K::KeyPress
            }
            Behavior::KeyToggle(usage) => {
                draft.usage = usage.base();
                draft.modifiers = usage.modifiers();
                K::KeyToggle
            }
            Behavior::StickyKey(usage) => {
                draft.usage = usage.base();
                draft.modifiers = usage.modifiers();
                K::StickyKey
            }
            Behavior::MomentaryLayer { layer_id } => {
                draft.layer_id = *layer_id;
                K::MomentaryLayer
            }
            Behavior::ToggleLayer { layer_id } => {
                draft.layer_id = *layer_id;
                K::ToggleLayer
            }
            Behavior::ToLayer { layer_id } => {
                draft.layer_id = *layer_id;
                K::ToLayer
            }
            Behavior::StickyLayer { layer_id } => {
                draft.layer_id = *layer_id;
                K::StickyLayer
            }
            Behavior::LayerTap { layer_id, tap } => {
                draft.layer_id = *layer_id;
                draft.usage = tap.base();
                draft.modifiers = tap.modifiers();
                K::LayerTap
            }
            Behavior::ModTap { hold, tap } => {
                draft.hold_mod = hold.base();
                draft.usage = tap.base();
                draft.modifiers = tap.modifiers();
                K::ModTap
            }
            Behavior::Transparent => K::Transparent,
            Behavior::None => K::NoneBehavior,
            Behavior::CapsWord => K::CapsWord,
            Behavior::KeyRepeat => K::KeyRepeat,
            Behavior::GraveEscape => K::GraveEscape,
            Behavior::StudioUnlock => K::StudioUnlock,
            Behavior::Reset => K::Reset,
            Behavior::Bootloader => K::Bootloader,
            Behavior::SoftOff => K::SoftOff,
            Behavior::Bluetooth { .. } => K::Bluetooth,
            Behavior::OutputSelection { .. } => K::OutputSelection,
            Behavior::Backlight { value, .. } => {
                draft.bl_value = *value;
                K::Backlight
            }
            Behavior::Underglow { .. } => K::Underglow,
            Behavior::MouseKeyPress { .. } => K::MouseKeyPress,
            Behavior::MouseMove { .. } => K::MouseMove,
            Behavior::MouseScroll { .. } => K::MouseScroll,
            Behavior::Custom { .. } | Behavior::Unknown { .. } | Behavior::ExternalPower { .. } => {
                K::KeyPress
            }
        };
        draft
    }

    /// Builds the `Behavior` the draft currently describes. Only the staged
    /// kinds (`needs_params`) encode from the draft; every other kind applies
    /// directly on click from its page's key grids.
    pub fn to_behavior(&self) -> Behavior {
        use ZmkBehaviorKind as K;
        let usage = HidUsage::from_parts(self.usage.page(), self.usage.id(), self.modifiers);
        match self.kind {
            K::KeyPress => Behavior::KeyPress(usage),
            K::KeyToggle => Behavior::KeyToggle(usage),
            K::StickyKey => Behavior::StickyKey(usage),
            K::ModTap => Behavior::ModTap {
                hold: self.hold_mod,
                tap: usage,
            },
            _ => unreachable!("only staged kinds encode from the draft"),
        }
    }
}

impl crate::overlay_window::OverlayApp {
    pub(super) fn draw_zmk_editor_body(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut ZmkDraft,
    ) {
        let layer_infos = keyboard.layer_infos();

        // Two-pane layout: every behavior kind lives in the left panel under
        // its group header, full-width selectable; the right pane holds the
        // selected kind's parameter form. Each pane scrolls independently and
        // fills the window instead of growing it. The parameterless behaviors
        // share one "Special" entry whose pane is a key grid.
        egui::Panel::left("zmk_kinds")
            .resizable(false)
            .exact_size(110.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(2.0);
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        for page in Page::ALL {
                            // Staged kinds get one entry each; the grouped
                            // pages share a single entry and apply from their
                            // page's key grids.
                            if matches!(page, Page::Keys | Page::Mods | Page::Commands) {
                                ui.weak(page.label());
                                for kind in page.kinds() {
                                    ui.selectable_value(&mut draft.kind, *kind, kind.label());
                                }
                            } else {
                                let response =
                                    ui.selectable_label(page_of(draft.kind) == page, page.label());
                                if response.clicked() {
                                    draft.kind = page.kinds()[0];
                                }
                            }
                            ui.add_space(4.0);
                        }
                    });
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match page_of(draft.kind) {
                    Page::Special => {
                        // Every parameterless behavior is a key here; clicking
                        // applies it directly.
                        self.draw_special_grid(ui, keyboard, target);
                    }
                    Page::Keys => {
                        self.draw_usage_picker(ui, draft, true);
                    }
                    Page::Layers => {
                        // One page of grouped layer keys; see draw_zmk_layer_page.
                        self.draw_zmk_layer_page(ui, keyboard, target, draft, &layer_infos);
                    }
                    Page::Mods => {
                        ui.label("Hold modifier:");
                        let mod_style = self.paint_style(MOD_KEY_UNIT);
                        modifier_select_grid(
                            ui,
                            "zmk_hold_mod",
                            Some(draft.hold_mod.id()),
                            &mod_style,
                            |id| {
                                draft.hold_mod = HidUsage::from_parts(HID_USAGE_KEYBOARD, id, 0);
                            },
                        );
                        ui.label("Tap key:");
                        self.draw_usage_picker(ui, draft, true);
                    }
                    Page::Commands => {
                        // Commands render as their own keys too; clicking
                        // applies directly.
                        self.draw_command_grid(ui, keyboard, target, draft);
                    }
                }

                if needs_params(draft.kind) {
                    ui.add_space(8.0);
                    if ui.button("Apply").clicked() {
                        self.apply_zmk_write(keyboard, target, draft.to_behavior());
                    }
                }
            });
        });
    }

    /// The usage-page key grid (with optional modifier toggles). Stages the
    /// picked usage into the draft; returns true when a usage was clicked so
    /// callers can apply the finished binding immediately.
    fn draw_usage_picker(
        &mut self,
        ui: &mut egui::Ui,
        draft: &mut ZmkDraft,
        with_mods: bool,
    ) -> bool {
        // The usage-page categories stay as headers inside one shared scroll
        // region; the key-shaped cells come from the shared picker grid.
        let selected =
            zmk_studio_api::HidUsage::from_parts(draft.usage.page(), draft.usage.id(), 0)
                .to_hid_usage();
        let style = self.paint_style(KEY_UNIT);
        let mut picked = false;

        if with_mods {
            let mod_style = self.paint_style(MOD_KEY_UNIT);
            modifier_toggle_row(
                ui,
                "zmk_mods",
                u16::from(draft.modifiers),
                &mod_style,
                |mask| {
                    draft.modifiers ^= mask as u8;
                },
            );
        }

        // No inner scroll area: the surrounding editor pane already scrolls,
        // so categories lay out flat inside it.
        let categories = zmk_catalog::categories();
        candidate_groups_rows(
            ui,
            categories,
            |_| Some(selected),
            &style,
            |_, candidate| {
                let usage = HidUsage::from_encoded(candidate.code);
                draft.usage = HidUsage::from_parts(usage.page(), usage.id(), draft.modifiers);
                picked = true;
            },
        );
        picked
    }

    /// The ZMK layer page: every layer behavior as one key per layer, grouped
    /// by behavior like the usage picker's categories. Momentary/Toggle/To/
    /// Sticky apply on click. Picking a Layer-Tap key only stages the layer and
    /// reveals the tap-key picker below; picking a tap key then applies the
    /// finished binding.
    fn draw_zmk_layer_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut ZmkDraft,
        layer_infos: &[crate::key_action::LayerInfo],
    ) {
        let layer_names: Vec<String> = layer_infos
            .iter()
            .map(|info| info.name.clone().unwrap_or_default())
            .collect();
        let tap = HidUsage::from_parts(draft.usage.page(), draft.usage.id(), draft.modifiers);
        let style = self.paint_style(KEY_UNIT);
        let kinds = Page::Layers.kinds();
        let groups = zmk_catalog::layer_groups(kinds, layer_infos, &layer_names, tap);

        // Per-group highlight: the staged Layer-Tap key, or the key matching
        // the current binding.
        let staged_layer = if draft.kind == ZmkBehaviorKind::LayerTap {
            layer_infos
                .iter()
                .position(|info| info.id == draft.layer_id)
                .map(|i| i as u32)
        } else {
            None
        };
        let selected = |gi: usize| {
            if kinds[gi] == ZmkBehaviorKind::LayerTap {
                staged_layer
            } else {
                selected_behavior_code(keyboard, target, &groups[gi].candidates)
            }
        };

        candidate_groups_rows(ui, &groups, selected, &style, |gi, candidate| {
            let kind = kinds[gi];
            if let Some(behavior) = &candidate.behavior {
                if kind == ZmkBehaviorKind::LayerTap {
                    // Selecting a Layer-Tap key only stages the layer; the
                    // tap-key picker appears below and its click applies.
                    if let Some(layer_id) = behavior_layer_id(behavior) {
                        draft.kind = ZmkBehaviorKind::LayerTap;
                        draft.layer_id = layer_id;
                    }
                } else {
                    draft.kind = kind;
                    if let Some(layer_id) = behavior_layer_id(behavior) {
                        draft.layer_id = layer_id;
                    }
                    self.apply_zmk_write(keyboard, target, behavior.clone());
                }
            }
        });

        if draft.kind == ZmkBehaviorKind::LayerTap {
            ui.label("Tap key:");
            if self.draw_usage_picker(ui, draft, true) {
                let tap =
                    HidUsage::from_parts(draft.usage.page(), draft.usage.id(), draft.modifiers);
                self.apply_zmk_write(
                    keyboard,
                    target,
                    Behavior::LayerTap {
                        layer_id: draft.layer_id,
                        tap,
                    },
                );
            }
        }
    }

    /// The parameterless behaviors as one key grid; clicking applies directly.
    fn draw_special_grid(&mut self, ui: &mut egui::Ui, keyboard: &Keyboard, target: EditTarget) {
        let candidates = zmk_catalog::special_candidates();
        let selected = selected_behavior_code(keyboard, target, candidates);
        let style = self.paint_style(KEY_UNIT);
        picker_grid_rows(
            ui,
            "zmk_special",
            candidates,
            selected,
            &style,
            |candidate| {
                if let Some(behavior) = &candidate.behavior {
                    self.apply_zmk_write(keyboard, target, behavior.clone());
                }
            },
        );
    }

    /// One command kind's options as a key grid; clicking applies directly.
    /// Backlight's `Set` keeps a level DragValue, since its value is part of
    /// the binding rather than a choice between keys.
    fn draw_command_grid(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut ZmkDraft,
    ) {
        if draft.kind == ZmkBehaviorKind::Backlight {
            ui.horizontal(|ui| {
                ui.weak("Set level");
                ui.add(egui::DragValue::new(&mut draft.bl_value).range(0..=255));
            });
            ui.add_space(4.0);
        }
        let candidates = zmk_catalog::command_candidates(draft.kind, draft.bl_value);
        let selected = selected_behavior_code(keyboard, target, &candidates);
        let style = self.paint_style(KEY_UNIT);
        picker_grid_rows(
            ui,
            draft.kind.label(),
            &candidates,
            selected,
            &style,
            |candidate| {
                if let Some(behavior) = &candidate.behavior {
                    self.apply_zmk_write(keyboard, target, behavior.clone());
                }
            },
        );
    }

    fn apply_zmk_write(&mut self, keyboard: &Keyboard, target: EditTarget, behavior: Behavior) {
        if self.editor.pending.is_some() {
            return;
        }
        let receiver = keyboard.set_key(
            target.layer_index,
            target.row,
            target.col,
            KeyAction::Zmk(behavior),
        );
        self.editor.pending = Some(receiver);
        self.editor.pending_kind = Some(PendingKind::Set);
        self.editor.error = None;
    }
}

/// The layer a layer-ish behavior targets, if any.
fn behavior_layer_id(behavior: &Behavior) -> Option<u32> {
    match behavior {
        Behavior::MomentaryLayer { layer_id }
        | Behavior::ToggleLayer { layer_id }
        | Behavior::ToLayer { layer_id }
        | Behavior::StickyLayer { layer_id }
        | Behavior::LayerTap { layer_id, .. } => Some(*layer_id),
        _ => None,
    }
}

/// The candidate matching the target's current binding, for the pressed
/// highlight in behavior grids.
fn selected_behavior_code(
    keyboard: &Keyboard,
    target: EditTarget,
    candidates: &[Candidate],
) -> Option<u32> {
    match keyboard.get_action(target.layer_index, target.row, target.col) {
        Some(KeyAction::Zmk(behavior)) => candidates
            .iter()
            .find(|candidate| candidate.behavior.as_ref() == Some(&behavior))
            .map(|candidate| candidate.code),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zmk_studio_api::MOD_LSFT;

    fn round_trip(behavior: Behavior) {
        let draft = ZmkDraft::from_behavior(&behavior);
        let rebuilt = draft.to_behavior();
        assert_eq!(
            rebuilt, behavior,
            "behavior should survive a draft round trip"
        );
    }

    #[test]
    fn key_press_with_modifiers_round_trips() {
        round_trip(Behavior::KeyPress(HidUsage::from_parts(
            HID_USAGE_KEYBOARD,
            0x04,
            MOD_LSFT,
        )));
    }

    #[test]
    fn mod_tap_round_trips() {
        round_trip(Behavior::ModTap {
            hold: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0xE1, 0),
            tap: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0),
        });
    }

    #[test]
    fn retarget_opens_the_matching_page() {
        use ZmkBehaviorKind as K;
        let kind_of = |behavior: Behavior| ZmkDraft::from_behavior(&behavior).kind;
        // Layer and command kinds apply from their page's grids, so the draft
        // only records which page to open on retarget.
        assert_eq!(
            kind_of(Behavior::MomentaryLayer { layer_id: 2 }),
            K::MomentaryLayer
        );
        assert_eq!(kind_of(Behavior::ToLayer { layer_id: 3 }), K::ToLayer);
        assert_eq!(kind_of(Behavior::Transparent), K::Transparent);
        assert_eq!(kind_of(Behavior::CapsWord), K::CapsWord);
        assert_eq!(
            kind_of(Behavior::Bluetooth {
                command: 3,
                value: 2
            }),
            K::Bluetooth
        );
        assert_eq!(kind_of(Behavior::MouseScroll { value: 1 }), K::MouseScroll);
    }

    #[test]
    fn layer_tap_stages_layer_and_tap_side() {
        // The layer page stages a Layer-Tap's layer; the tap picker below
        // applies the finished binding from the draft.
        let draft = ZmkDraft::from_behavior(&Behavior::LayerTap {
            layer_id: 3,
            tap: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x1C, MOD_LSFT),
        });
        assert_eq!(draft.kind, ZmkBehaviorKind::LayerTap);
        assert_eq!(draft.layer_id, 3);
        assert_eq!(draft.usage.id(), 0x1C);
        assert_eq!(draft.modifiers, MOD_LSFT);
    }
}
