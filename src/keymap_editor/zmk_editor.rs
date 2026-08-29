//! ZMK editor content: behavior-based editing with per-behavior parameters,
//! plus the session Save/Discard flow.

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use zmk_studio_api::{Behavior, HidUsage, HID_USAGE_KEYBOARD, MOD_LSFT};

use super::picker::{
    candidate_groups_rows, framed_candidate_groups_rows, modifier_toggle_grid, picker_grid_rows,
    Candidate, KEY_UNIT,
};
use super::zmk_catalog::{self, ZmkBehaviorKind};
use super::EditTarget;
use crate::ui_widgets::titled_group;

/// The editor's pages: how the left panel groups behavior kinds and which
/// pane the central panel shows. [`Page::kinds`] is the single source for the
/// groupings — the panel and the pane dispatch both derive from it.
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
                None,
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

    fn supported_kinds(self, keyboard: &Keyboard) -> Vec<ZmkBehaviorKind> {
        self.kinds()
            .iter()
            .copied()
            .filter(|k| {
                keyboard.is_action_supported(&KeyAction::Zmk(zmk_catalog::sample_behavior(*k)))
            })
            .collect()
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

/// The editor's editable fields for ZMK, rebuilt on each retarget.
#[derive(Clone)]
pub struct ZmkDraft {
    pub kind: ZmkBehaviorKind,
    /// Base key usage selected in a picker (key press/toggle/sticky, layer-tap
    /// and mod-tap tap side).
    pub usage: HidUsage,
    /// Keyboard modifiers OR'd onto `usage` for a key press.
    pub modifiers: u8,
    /// Held modifier mask for a Mod-Tap, in HID bit order (LCTL 0x01 …
    /// RGUI 0x80). Any combination of the eight is expressible: ZMK applies
    /// each bit as its own HID modifier, mixed hands included.
    pub hold_mods: u8,
    /// Layer a layer behavior or layer-tap targets (the stable layer id).
    pub layer_id: u32,
    /// Backlight `Set` level: a value that is part of the binding rather than
    /// a choice between keys, so it stays staged in the draft.
    pub bl_value: u32,
    /// Whether a Backlight `Set` binding is being staged, which reveals the
    /// level control. Every other backlight command is a plain key in the grid
    /// and applies directly.
    pub bl_set_staged: bool,
    /// Whether the user has interacted with the draft's parameter controls
    /// since it was built. Only a touched draft can be mid-selection (and so
    /// ghosted as invalid in the header); a fresh draft mirrors the current
    /// binding.
    pub touched: bool,
}

impl Default for ZmkDraft {
    fn default() -> Self {
        Self {
            kind: ZmkBehaviorKind::KeyPress,
            usage: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0),
            modifiers: 0,
            hold_mods: MOD_LSFT,
            layer_id: 0,
            bl_value: 0,
            bl_set_staged: false,
            touched: false,
        }
    }
}

impl ZmkDraft {
    /// Pre-fills the draft from an existing ZMK behavior.
    pub fn from_behavior(behavior: &Behavior) -> Self {
        let mut draft = ZmkDraft {
            kind: behavior.role().unwrap_or(ZmkBehaviorKind::KeyPress),
            ..Default::default()
        };
        match behavior {
            Behavior::KeyPress(usage) | Behavior::KeyToggle(usage) | Behavior::StickyKey(usage) => {
                draft.usage = usage.base();
                draft.modifiers = usage.modifiers();
            }
            Behavior::MomentaryLayer { layer_id }
            | Behavior::ToggleLayer { layer_id }
            | Behavior::ToLayer { layer_id }
            | Behavior::StickyLayer { layer_id } => {
                draft.layer_id = *layer_id;
            }
            Behavior::LayerTap { layer_id, tap } => {
                draft.layer_id = *layer_id;
                draft.usage = tap.base();
                draft.modifiers = tap.modifiers();
            }
            Behavior::ModTap { hold, tap } => {
                draft.hold_mods = hold_mod_mask(hold);
                draft.usage = tap.base();
                draft.modifiers = tap.modifiers();
            }
            Behavior::Backlight { command, value } => {
                draft.bl_value = *value;
                // A `Set` binding re-opens with its level control, so the
                // value can be tweaked and re-applied directly.
                draft.bl_set_staged = *command == zmk_catalog::BACKLIGHT_SET_COMMAND;
            }
            _ => {}
        }
        draft
    }

    /// Builds the `Behavior` the draft currently describes. Staged kinds
    /// encode from the draft; every other kind applies directly on click from
    /// its page's key grids.
    pub fn to_behavior(&self) -> Behavior {
        use ZmkBehaviorKind as K;
        let usage = HidUsage::from_parts(self.usage.page(), self.usage.id(), self.modifiers);
        match self.kind {
            K::KeyPress => Behavior::KeyPress(usage),
            K::KeyToggle => Behavior::KeyToggle(usage),
            K::StickyKey => Behavior::StickyKey(usage),
            K::ModTap => Behavior::ModTap {
                hold: hold_usage(self.hold_mods),
                tap: usage,
            },
            K::LayerTap => Behavior::LayerTap {
                layer_id: self.layer_id,
                tap: usage,
            },
            _ => unreachable!("only staged kinds encode from the draft"),
        }
    }

    /// The behavior the draft currently describes, if complete. A Mod-Tap
    /// without a hold modifier has nothing to do on hold and cannot be
    /// applied; every other staged kind always encodes. A valid draft applies
    /// instantly, so it never lingers staged.
    pub(super) fn staged(&self) -> Option<Behavior> {
        match self.kind {
            ZmkBehaviorKind::KeyPress
            | ZmkBehaviorKind::KeyToggle
            | ZmkBehaviorKind::StickyKey
            | ZmkBehaviorKind::LayerTap => Some(self.to_behavior()),
            ZmkBehaviorKind::ModTap if self.hold_mods != 0 => Some(self.to_behavior()),
            _ => None,
        }
    }

    /// The ghost binding shown in the header while the user is mid-selection
    /// and the draft is not yet a valid binding. The only staged kind that can
    /// be invalid is a Mod-Tap without hold modifiers; it previews its tap side.
    pub(super) fn ghost_action(&self) -> Option<KeyAction> {
        if !self.touched || self.staged().is_some() {
            return None;
        }
        Some(KeyAction::Zmk(Behavior::KeyPress(HidUsage::from_parts(
            self.usage.page(),
            self.usage.id(),
            self.modifiers,
        ))))
    }
}

/// The held-modifier mask of a Mod-Tap hold side. Holds written as a modifier
/// *usage* (`&mt LSHIFT A` encodes LEFT_SHIFT, usage 0xE0–0xE7) map to their
/// HID bit; holds written as a masked usage (`&mt LC(RS(X)) X` puts the whole
/// mask in the usage's modifier byte) carry that mask directly. Anything else
/// (a non-modifier hold, which the UI cannot represent) shows no selection.
fn hold_mod_mask(hold: &HidUsage) -> u8 {
    if hold.modifiers() != 0 {
        return hold.modifiers();
    }
    // HID keyboard modifier usages are the contiguous block 0xE0–0xE7, in the
    // same order as the mask bits.
    if (0xE0..=0xE7).contains(&hold.id()) {
        1 << (hold.id() - 0xE0)
    } else {
        0
    }
}

/// Builds the Mod-Tap hold usage from the selected mask. A single modifier
/// keeps the bare modifier-usage form (`&mt LSHIFT A`), the canonical shape
/// firmware writes for plain mod-taps; any other selection encodes the full
/// HID mask in the usage's modifier byte (`&mt LC(RS(X)) X` style).
fn hold_usage(hold_mods: u8) -> HidUsage {
    if hold_mods.count_ones() == 1 {
        HidUsage::from_parts(
            HID_USAGE_KEYBOARD,
            0xE0 + hold_mods.trailing_zeros() as u16,
            0,
        )
    } else {
        HidUsage::from_parts(HID_USAGE_KEYBOARD, 0, hold_mods)
    }
}

impl crate::overlay_window::OverlayApp {
    /// Marks the draft touched and applies its staged behavior when it is a
    /// complete, changed binding. Valid picks apply at once; ZMK writes are
    /// session changes tracked by the save bar.
    fn commit_zmk_draft(&mut self, keyboard: &Keyboard, target: EditTarget, draft: &mut ZmkDraft) {
        draft.touched = true;
        if let Some(behavior) = draft.staged() {
            let action = KeyAction::Zmk(behavior);
            if keyboard
                .get_action(target.layer_index, target.row, target.col)
                .as_ref()
                != Some(&action)
            {
                self.apply_write(keyboard, target, action);
            }
        }
    }

    pub(super) fn draw_zmk_editor_body(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut ZmkDraft,
    ) {
        let layer_infos = keyboard.layer_infos();

        // If the current draft kind is not supported on this keyboard, switch to the first supported one.
        if !keyboard.is_action_supported(&KeyAction::Zmk(zmk_catalog::sample_behavior(draft.kind))) {
            draft.kind = Page::ALL
                .iter()
                .find_map(|page| page.supported_kinds(keyboard).into_iter().next())
                .unwrap_or(ZmkBehaviorKind::KeyPress);
            draft.bl_set_staged = false;
            draft.touched = false;
        }

        // Two-pane layout: every behavior kind lives in the left panel under
        // its group header; the right pane holds the selected page. The
        // parameterless behaviors share one "Special" entry whose pane is a
        // key grid.
        super::editor_panes(
            ui,
            "zmk_kinds",
            110.0,
            draft,
            |ui, draft| {
                for page in Page::ALL {
                    let kinds = page.supported_kinds(keyboard);
                    if kinds.is_empty() {
                        continue;
                    }
                    if matches!(page, Page::Keys | Page::Mods | Page::Commands) {
                        ui.weak(page.label());
                        for kind in kinds {
                            let response =
                                ui.selectable_value(&mut draft.kind, kind, kind.label());
                            // Switching behavior kinds leaves any staged
                            // Backlight Set behind and starts a fresh
                            // selection.
                            if response.changed() {
                                draft.bl_set_staged = false;
                                draft.touched = false;
                            }
                        }
                    } else {
                        let response =
                            ui.selectable_label(page_of(draft.kind) == page, page.label());
                        if response.clicked() {
                            draft.kind = kinds[0];
                            draft.bl_set_staged = false;
                            draft.touched = false;
                        }
                    }
                    ui.add_space(4.0);
                }
            },
            |ui, draft| {
                match page_of(draft.kind) {
                    Page::Special | Page::Commands => {
                        // Every parameterless behavior and command option is a
                        // key here; clicking applies it directly.
                        self.draw_direct_grid(ui, keyboard, target, draft);
                    }
                    Page::Keys => {
                        // One argument, one group: the usage's modifier toggles
                        // and key grid are tightly coupled, so they share the
                        // boundary.
                        titled_group(ui, "Key", |ui| {
                            self.draw_usage_picker(ui, keyboard, target, draft, true);
                        });
                    }
                    Page::Layers => {
                        // One page of grouped layer keys; see draw_zmk_layer_page.
                        self.draw_zmk_layer_page(ui, keyboard, target, draft, &layer_infos);
                    }
                    Page::Mods => {
                        // Two distinct arguments, two groups: the hold-side
                        // modifier mask, and the tap-side usage (whose own
                        // modifier toggles stay inside the tap group).
                        let mod_style = self.paint_style(KEY_UNIT);
                        titled_group(ui, "Hold modifiers", |ui| {
                            modifier_toggle_grid(
                                ui,
                                "zmk_hold_mod",
                                draft.hold_mods,
                                &mod_style,
                                |mask| {
                                    draft.hold_mods ^= mask;
                                    self.commit_zmk_draft(keyboard, target, draft);
                                },
                            );
                        });
                        titled_group(ui, "Tap key", |ui| {
                            self.draw_usage_picker(ui, keyboard, target, draft, true);
                        });
                        // A Mod-Tap without a hold modifier has nothing to do
                        // on hold, so the header ghosts it as invalid.
                        if draft.kind == ZmkBehaviorKind::ModTap && draft.hold_mods == 0 {
                            ui.weak("Select at least one hold modifier.");
                        }
                    }
                }
            },
        );
    }

    /// The usage-page key grid (with optional modifier toggles). Stages the
    /// picked usage into the draft and commits it: the staged behavior follows
    /// the draft's kind (key press, key toggle, sticky key, layer-tap tap side,
    /// mod-tap tap side), so a complete pick applies at once.
    fn draw_usage_picker(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut ZmkDraft,
        with_mods: bool,
    ) {
        // The usage-page categories stay as headers inside one shared scroll
        // region; the key-shaped cells come from the shared picker grid.
        let selected = KeyAction::Zmk(Behavior::KeyPress(draft.usage.base()));
        let style = self.paint_style(KEY_UNIT);

        if with_mods {
            let mod_style = self.paint_style(KEY_UNIT);
            modifier_toggle_grid(ui, "zmk_mods", draft.modifiers, &mod_style, |mask| {
                draft.modifiers ^= mask;
                self.commit_zmk_draft(keyboard, target, draft);
            });
        }

        // No inner scroll area: the surrounding editor pane already scrolls,
        // so categories lay out flat inside it.
        let categories = zmk_catalog::categories();
        candidate_groups_rows(
            ui,
            categories,
            |_| Some(selected.clone()),
            &style,
            |_, candidate| {
                // The base usage is staged; the draft's modifiers ride along.
                if let KeyAction::Zmk(Behavior::KeyPress(usage)) = &candidate.binding {
                    draft.usage = HidUsage::from_parts(usage.page(), usage.id(), draft.modifiers);
                    self.commit_zmk_draft(keyboard, target, draft);
                }
            },
        );
    }

    /// The ZMK layer page: every layer behavior as one framed group of one key
    /// per layer, one outline per behavior kind. Momentary/Toggle/To/Sticky
    /// apply on click. Picking a Layer-Tap key only stages the layer and
    /// reveals the tap-key group below; picking a tap key then applies the
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
        let kinds = Page::Layers.supported_kinds(keyboard);
        let groups = zmk_catalog::layer_groups(&kinds, layer_infos, &layer_names, tap);

        // Per-group highlight: the staged Layer-Tap key, or the key matching
        // the current binding.
        let staged_tap = if draft.kind == ZmkBehaviorKind::LayerTap {
            Some(KeyAction::Zmk(Behavior::LayerTap {
                layer_id: draft.layer_id,
                tap,
            }))
        } else {
            None
        };
        let selected = |gi: usize| {
            if kinds[gi] == ZmkBehaviorKind::LayerTap {
                staged_tap.clone()
            } else {
                keyboard.get_action(target.layer_index, target.row, target.col)
            }
        };

        framed_candidate_groups_rows(ui, &groups, selected, &style, |gi, candidate| {
            let kind = kinds[gi];
            if let KeyAction::Zmk(behavior) = &candidate.binding {
                if kind == ZmkBehaviorKind::LayerTap {
                    // Selecting a Layer-Tap key applies it with the draft's
                    // current tap key; the tap-key group below retunes the
                    // tap side.
                    if let Some(layer_id) = behavior_layer_id(behavior) {
                        draft.kind = ZmkBehaviorKind::LayerTap;
                        draft.layer_id = layer_id;
                        self.commit_zmk_draft(keyboard, target, draft);
                    }
                } else {
                    draft.kind = kind;
                    if let Some(layer_id) = behavior_layer_id(behavior) {
                        draft.layer_id = layer_id;
                    }
                    self.apply_write(keyboard, target, candidate.binding.clone());
                }
            }
        });

        if draft.kind == ZmkBehaviorKind::LayerTap {
            // The staged tap side is one distinct argument: usage + its
            // modifier toggles share the group.
            titled_group(ui, "Tap key", |ui| {
                self.draw_usage_picker(ui, keyboard, target, draft, true);
            });
        }
    }

    /// The Special and Commands pages as key grids; clicking applies directly.
    /// The whole grid sits in one outline — one for the shared Special entry,
    /// one per selected command kind. The exception is Backlight `Set`, whose
    /// level is part of the binding: clicking it stages the binding and
    /// reveals a Level group below the grid; the tuned level commits when the
    /// drag or text entry finishes.
    fn draw_direct_grid(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        draft: &mut ZmkDraft,
    ) {
        let staging_set = draft.kind == ZmkBehaviorKind::Backlight && draft.bl_set_staged;
        let is_special = page_of(draft.kind) == Page::Special;
        let candidates: Vec<Candidate> = if is_special {
            Page::Special
                .supported_kinds(keyboard)
                .into_iter()
                .map(|kind| zmk_catalog::behavior_candidate(&zmk_catalog::sample_behavior(kind), &[]))
                .collect()
        } else {
            zmk_catalog::command_candidates(draft.kind, draft.bl_value)
        };
        let grid_title = if is_special {
            "Special"
        } else {
            draft.kind.label()
        };
        let style = self.paint_style(KEY_UNIT);
        titled_group(ui, grid_title, |ui| {
            picker_grid_rows(
                ui,
                draft.kind.label(),
                &candidates,
                keyboard
                    .get_action(target.layer_index, target.row, target.col)
                    .as_ref(),
                &style,
                |candidate| {
                    if is_backlight_set(&candidate.binding) {
                        draft.bl_set_staged = true;
                        return;
                    }
                    draft.bl_set_staged = false;
                    self.apply_write(keyboard, target, candidate.binding.clone());
                },
            );
        });

        if staging_set {
            // The level is the one command argument that is part of the
            // binding, so it gets its own group below the key grid. A level is
            // tuned rather than picked, so the binding commits when the drag
            // or the text entry finishes instead of on every frame it changes.
            titled_group(ui, "Level", |ui| {
                let level = ui.add(egui::DragValue::new(&mut draft.bl_value).range(0..=255));
                if level.drag_stopped() || level.lost_focus() {
                    let behavior = Behavior::Backlight {
                        command: zmk_catalog::BACKLIGHT_SET_COMMAND,
                        value: draft.bl_value,
                    };
                    self.apply_write(keyboard, target, KeyAction::Zmk(behavior));
                }
            });
        }
    }
}

/// Whether a binding is the Backlight `Set` command, the one command whose
/// value is part of the binding.
fn is_backlight_set(binding: &KeyAction) -> bool {
    matches!(
        binding,
        KeyAction::Zmk(Behavior::Backlight {
            command: zmk_catalog::BACKLIGHT_SET_COMMAND,
            ..
        })
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use zmk_studio_api::{MOD_LCTL, MOD_RALT, MOD_RSFT};

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
    fn mod_tap_masked_hold_round_trips() {
        // `&mt LC(RS(A)) A`-style holds: the whole mask rides in the usage's
        // modifier byte, mixed hands included.
        let behavior = Behavior::ModTap {
            hold: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0, MOD_LCTL | MOD_RSFT),
            tap: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0),
        };
        let draft = ZmkDraft::from_behavior(&behavior);
        assert_eq!(draft.hold_mods, MOD_LCTL | MOD_RSFT);
        assert_eq!(draft.to_behavior(), behavior);
    }

    #[test]
    fn single_mod_hold_stays_a_modifier_usage() {
        // One selected bit encodes back as the bare modifier usage the
        // firmware writes for plain mod-taps, not as a masked usage.
        let draft = ZmkDraft {
            kind: ZmkBehaviorKind::ModTap,
            hold_mods: MOD_RALT,
            ..Default::default()
        };
        match draft.to_behavior() {
            Behavior::ModTap { hold, .. } => {
                assert_eq!(hold.id(), 0xE6);
                assert_eq!(hold.modifiers(), 0);
            }
            _ => unreachable!("ModTap draft must encode a ModTap"),
        }
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

    #[test]
    fn mod_tap_without_hold_modifier_is_invalid_and_ghosts() {
        let mut draft = ZmkDraft::default();
        draft.kind = ZmkBehaviorKind::ModTap;
        draft.hold_mods = 0;
        draft.touched = true;
        assert_eq!(draft.staged(), None);
        // The ghost previews the tap side while the hold side is empty.
        assert_eq!(
            draft.ghost_action(),
            Some(KeyAction::Zmk(Behavior::KeyPress(draft.usage.base())))
        );
        draft.hold_mods = MOD_LSFT;
        assert_eq!(
            draft.staged(),
            Some(Behavior::ModTap {
                hold: hold_usage(MOD_LSFT),
                tap: draft.usage.base(),
            })
        );
        assert_eq!(draft.ghost_action(), None);
    }
}
