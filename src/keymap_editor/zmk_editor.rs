//! ZMK behavior editor and parameter encoder.

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use zmk_studio_api::{Behavior, HidUsage, HID_USAGE_KEYBOARD};

use super::picker::{
    candidate_groups_rows, framed_candidate_groups_rows, modifier_toggle_grid, picker_grid_rows,
    Candidate, SelectedKey, KEY_UNIT,
};
use super::zmk_catalog::{self, ZmkBehaviorKind};
use super::EditTarget;
use crate::ui_widgets::titled_group;

/// Page categories for ZMK behaviors.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Page {
    Keys,
    Layers,
    Mods,
    Special,
    Commands,
}

impl Page {
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

    /// Returns behavior roles assigned to this page.
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

    fn supported_kinds<'a>(
        &self,
        keyboard: &'a Keyboard,
    ) -> impl Iterator<Item = ZmkBehaviorKind> + 'a {
        self.kinds().iter().copied().filter(|k| {
            keyboard.is_action_supported(&KeyAction::Zmk(zmk_catalog::sample_behavior(*k)))
        })
    }
}

/// Returns the page that contains the specified behavior role.
fn page_of(kind: ZmkBehaviorKind) -> Page {
    Page::ALL
        .into_iter()
        .find(|page| page.kinds().contains(&kind))
        .unwrap_or(Page::Keys)
}

/// Editable parameter state for ZMK behaviors.
#[derive(Clone)]
pub struct ZmkDraft {
    pub kind: ZmkBehaviorKind,
    /// Base HID usage, or None if unselected.
    pub usage: Option<HidUsage>,
    /// Modifier mask for key press behaviors.
    pub modifiers: u8,
    /// Held modifier mask for Mod-Tap behaviors.
    pub hold_mods: u8,
    /// Target layer ID for layer behaviors.
    pub layer_id: Option<u32>,
    /// Brightness level for Backlight Set commands.
    pub bl_value: u32,
    /// Indicates whether Backlight Set is staged.
    pub bl_set_staged: bool,
}

impl Default for ZmkDraft {
    fn default() -> Self {
        Self {
            kind: ZmkBehaviorKind::KeyPress,
            usage: None,
            modifiers: 0,
            hold_mods: 0,
            layer_id: None,
            bl_value: 0,
            bl_set_staged: false,
        }
    }
}

impl ZmkDraft {
    /// Initializes draft for a specific kind, preserving the active behavior if it matches.
    fn for_kind(kind: ZmkBehaviorKind, current_action: Option<&KeyAction>) -> Self {
        match current_action {
            Some(KeyAction::Zmk(b)) if b.role() == Some(kind) => Self::from_behavior(b),
            _ => Self {
                kind,
                ..Default::default()
            },
        }
    }

    /// Initializes draft for a page, preserving the active behavior if it belongs to this page.
    fn for_page(page: Page, current_action: Option<&KeyAction>, fallback: ZmkBehaviorKind) -> Self {
        match current_action {
            Some(KeyAction::Zmk(b)) if b.role().map(page_of) == Some(page) => {
                Self::from_behavior(b)
            }
            _ => Self {
                kind: fallback,
                ..Default::default()
            },
        }
    }

    /// Returns the combined HID usage with draft modifiers applied, if selected.
    pub fn tap_usage(&self) -> Option<HidUsage> {
        self.usage
            .map(|u| HidUsage::from_parts(u.page(), u.id(), self.modifiers))
    }

    /// Creates draft state from an existing ZMK behavior.
    pub fn from_behavior(behavior: &Behavior) -> Self {
        let kind = behavior
            .role()
            .filter(|&r| Page::ALL.iter().any(|p| p.kinds().contains(&r)))
            .unwrap_or(ZmkBehaviorKind::KeyPress);
        let mut draft = ZmkDraft {
            kind,
            layer_id: behavior.layer_id(),
            ..Default::default()
        };
        match behavior {
            Behavior::KeyPress(usage) | Behavior::KeyToggle(usage) | Behavior::StickyKey(usage) => {
                draft.usage = Some(usage.base());
                draft.modifiers = usage.modifiers();
            }
            Behavior::LayerTap { tap, .. } => {
                draft.usage = Some(tap.base());
                draft.modifiers = tap.modifiers();
            }
            Behavior::ModTap { hold, tap } => {
                draft.hold_mods = hold.modifier_mask();
                draft.usage = Some(tap.base());
                draft.modifiers = tap.modifiers();
            }
            Behavior::Backlight(zmk_studio_api::BacklightCommand::Set(value)) => {
                draft.bl_value = *value as u32;
                draft.bl_set_staged = true;
            }
            _ => {}
        }
        draft
    }

    /// Returns the encoded Behavior if all required parameters are valid.
    pub fn staged(&self) -> Option<Behavior> {
        let tap_usage = self.tap_usage();
        match self.kind {
            ZmkBehaviorKind::KeyPress => tap_usage.map(Behavior::KeyPress),
            ZmkBehaviorKind::KeyToggle => tap_usage.map(Behavior::KeyToggle),
            ZmkBehaviorKind::StickyKey => tap_usage.map(Behavior::StickyKey),
            ZmkBehaviorKind::LayerTap => match (self.layer_id, tap_usage) {
                (Some(layer_id), Some(tap)) => Some(Behavior::LayerTap { layer_id, tap }),
                _ => None,
            },
            ZmkBehaviorKind::ModTap if self.hold_mods != 0 => {
                tap_usage.map(|tap| Behavior::ModTap {
                    hold: HidUsage::from_modifier_mask(self.hold_mods),
                    tap,
                })
            }
            _ => None,
        }
    }

    /// Returns whether the draft contains a valid, complete behavior configuration.
    pub fn is_valid(&self) -> bool {
        match self.kind {
            ZmkBehaviorKind::KeyPress
            | ZmkBehaviorKind::KeyToggle
            | ZmkBehaviorKind::StickyKey
            | ZmkBehaviorKind::LayerTap
            | ZmkBehaviorKind::ModTap => self.staged().is_some(),
            _ => true,
        }
    }
}

impl crate::overlay_window::OverlayApp {
    /// Applies the current draft behavior to the target key.
    fn commit_zmk_draft(&mut self, keyboard: &Keyboard, target: EditTarget) {
        let staged = self.editor.zmk_draft.staged().map(KeyAction::Zmk);
        self.commit_staged(keyboard, target, staged);
    }

    pub(super) fn draw_zmk_editor_body(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
    ) {
        let layer_infos = keyboard.layer_infos();
        let style = self.paint_style(KEY_UNIT);

        // If the current draft kind is not supported on this keyboard, switch to the first supported one.
        if !keyboard.is_action_supported(&KeyAction::Zmk(zmk_catalog::sample_behavior(
            self.editor.zmk_draft.kind,
        ))) {
            if let Some(first) = Page::ALL
                .iter()
                .flat_map(|p| p.supported_kinds(keyboard))
                .next()
            {
                self.editor.zmk_draft.kind = first;
            }
            self.editor.zmk_draft.bl_set_staged = false;
        }

        // Left pane: grouped behavior-kind selector. Direct-apply and
        // parameterless behaviors share one "Special" entry whose pane is a
        // key grid.
        super::editor_left_panel(ui, "zmk_kinds", |ui| {
            for page in Page::ALL {
                let mut kinds = page.supported_kinds(keyboard).peekable();
                if kinds.peek().is_none() {
                    continue;
                }
                if matches!(page, Page::Keys | Page::Mods | Page::Commands) {
                    ui.weak(page.label());
                    for kind in kinds {
                        let response = ui.selectable_value(
                            &mut self.editor.zmk_draft.kind,
                            kind,
                            kind.label(),
                        );
                        if response.changed() {
                            self.reset_zmk_draft_for_kind(keyboard, target, kind);
                        }
                    }
                } else {
                    let first_kind = *kinds.peek().unwrap();
                    let response = ui.selectable_label(
                        page_of(self.editor.zmk_draft.kind) == page,
                        page.label(),
                    );
                    if response.clicked() {
                        let current_action =
                            keyboard.get_action(target.layer_index, target.row, target.col);
                        self.editor.zmk_draft =
                            ZmkDraft::for_page(page, current_action.as_ref(), first_kind);
                    }
                }
                ui.add_space(4.0);
            }
        });

        let current_page = page_of(self.editor.zmk_draft.kind);
        let is_valid = self.editor.zmk_draft.is_valid();
        super::editor_central_panel(ui, (target.layer_index, current_page), |ui| {
            match current_page {
                Page::Special | Page::Commands => {
                    // Every parameterless behavior and command option is a
                    // key here; clicking applies it directly.
                    self.draw_direct_grid(ui, keyboard, target, &style);
                }
                Page::Keys => {
                    // One argument, one group: the usage's modifier toggles
                    // and key grid are tightly coupled, so they share the
                    // boundary.
                    titled_group(ui, "Key", |ui| {
                        self.draw_usage_picker(ui, keyboard, target, is_valid, &style);
                    });
                }
                Page::Layers => {
                    // One page of grouped layer keys; see draw_zmk_layer_page.
                    self.draw_zmk_layer_page(ui, keyboard, target, &layer_infos, is_valid, &style);
                }
                Page::Mods => {
                    // Two distinct arguments, two groups: the hold-side
                    // modifier (single choice for standard ZMK &mt), and
                    // the tap-side usage (whose own modifier toggles stay
                    // inside the tap group).
                    titled_group(ui, "Hold modifier", |ui| {
                        modifier_toggle_grid(
                            ui,
                            "zmk_hold_mod",
                            self.editor.zmk_draft.hold_mods,
                            is_valid,
                            &style,
                            |mask| {
                                self.editor.zmk_draft.hold_mods =
                                    if self.editor.zmk_draft.hold_mods == mask {
                                        0
                                    } else {
                                        mask
                                    };
                                self.commit_zmk_draft(keyboard, target);
                            },
                        );
                    });
                    titled_group(ui, "Tap key", |ui| {
                        self.draw_usage_picker(ui, keyboard, target, is_valid, &style);
                    });
                    // A Mod-Tap without a hold modifier has nothing to do
                    // on hold, so the header ghosts it as invalid.
                    if self.editor.zmk_draft.kind == ZmkBehaviorKind::ModTap
                        && self.editor.zmk_draft.hold_mods == 0
                    {
                        ui.weak("Select a hold modifier.");
                    }
                }
            }
        });
    }

    fn reset_zmk_draft_for_kind(
        &mut self,
        keyboard: &Keyboard,
        target: EditTarget,
        kind: ZmkBehaviorKind,
    ) {
        self.editor.zmk_draft.bl_set_staged = false;
        let current_action = keyboard.get_action(target.layer_index, target.row, target.col);
        self.editor.zmk_draft = ZmkDraft::for_kind(kind, current_action.as_ref());
    }

    /// Draws the HID usage picker grid.
    fn draw_usage_picker(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        valid: bool,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let action = self
            .editor
            .zmk_draft
            .usage
            .map(|u| KeyAction::Zmk(Behavior::KeyPress(u.base())));
        let selected = action.as_ref().map(|a| SelectedKey::new(a, valid));

        modifier_toggle_grid(
            ui,
            "zmk_mods",
            self.editor.zmk_draft.modifiers,
            valid,
            style,
            |mask| {
                self.editor.zmk_draft.modifiers ^= mask;
                self.commit_zmk_draft(keyboard, target);
            },
        );

        candidate_groups_rows(
            ui,
            zmk_catalog::categories(),
            |c| keyboard.is_action_supported(&c.binding),
            selected,
            style,
            |_, candidate| {
                if let KeyAction::Zmk(Behavior::KeyPress(usage)) = &candidate.binding {
                    self.editor.zmk_draft.usage = Some(usage.base());
                    self.commit_zmk_draft(keyboard, target);
                }
            },
        );
    }

    /// Draws the ZMK layer behavior selection page.
    fn draw_zmk_layer_page(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        layer_infos: &[crate::key_action::LayerInfo],
        valid: bool,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let layer_names: Vec<String> = layer_infos
            .iter()
            .map(|info| info.name.clone().unwrap_or_default())
            .collect();
        let tap = self
            .editor
            .zmk_draft
            .tap_usage()
            .unwrap_or_else(|| HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0));
        let kinds: Vec<_> = Page::Layers.supported_kinds(keyboard).collect();
        let groups = zmk_catalog::layer_groups(&kinds, layer_infos, &layer_names, tap);

        let is_lt = self.editor.zmk_draft.kind == ZmkBehaviorKind::LayerTap;
        let lt_action = is_lt
            .then(|| {
                self.editor
                    .zmk_draft
                    .layer_id
                    .map(|id| KeyAction::Zmk(Behavior::LayerTap { layer_id: id, tap }))
            })
            .flatten();
        let current_action = keyboard.get_action(target.layer_index, target.row, target.col);
        let selected = if is_lt {
            lt_action.as_ref().map(|a| SelectedKey::new(a, valid))
        } else {
            current_action.as_ref().map(SelectedKey::valid)
        };

        framed_candidate_groups_rows(ui, &groups, selected, style, |gi, candidate| {
            let kind = kinds[gi];
            if let KeyAction::Zmk(behavior) = &candidate.binding {
                if kind == ZmkBehaviorKind::LayerTap {
                    if let Some(layer_id) = behavior.layer_id() {
                        self.editor.zmk_draft.kind = ZmkBehaviorKind::LayerTap;
                        self.editor.zmk_draft.layer_id = Some(layer_id);
                        self.commit_zmk_draft(keyboard, target);
                    }
                } else {
                    self.editor.zmk_draft.kind = kind;
                    self.editor.zmk_draft.layer_id = behavior.layer_id();
                    self.apply_write(keyboard, target, candidate.binding.clone());
                }
            }
        });

        if self.editor.zmk_draft.kind == ZmkBehaviorKind::LayerTap {
            titled_group(ui, "Tap key", |ui| {
                self.draw_usage_picker(ui, keyboard, target, valid, style);
            });
        }
    }

    /// Draws direct-apply behavior grids for command and special keys.
    fn draw_direct_grid(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let kind = self.editor.zmk_draft.kind;
        let staging_set = kind == ZmkBehaviorKind::Backlight && self.editor.zmk_draft.bl_set_staged;
        let is_special = page_of(kind) == Page::Special;
        let candidates: Vec<Candidate> = if is_special {
            Page::Special
                .supported_kinds(keyboard)
                .map(|k| zmk_catalog::behavior_candidate(&zmk_catalog::sample_behavior(k), &[]))
                .collect()
        } else {
            zmk_catalog::command_candidates(kind, self.editor.zmk_draft.bl_value)
                .into_iter()
                .filter(|c| keyboard.is_action_supported(&c.binding))
                .collect()
        };
        let grid_title = if is_special { "Special" } else { kind.label() };
        let action = keyboard.get_action(target.layer_index, target.row, target.col);
        let selected = action.as_ref().map(SelectedKey::valid);
        titled_group(ui, grid_title, |ui| {
            picker_grid_rows(
                ui,
                kind.label(),
                &candidates,
                selected,
                style,
                |candidate| {
                    if is_backlight_set(&candidate.binding) {
                        self.editor.zmk_draft.bl_set_staged = true;
                        return;
                    }
                    self.editor.zmk_draft.bl_set_staged = false;
                    self.apply_write(keyboard, target, candidate.binding.clone());
                },
            );
        });

        if staging_set {
            titled_group(ui, "Level", |ui| {
                let level = ui
                    .add(egui::DragValue::new(&mut self.editor.zmk_draft.bl_value).range(0..=255));
                if level.drag_stopped() || level.lost_focus() {
                    let behavior = Behavior::Backlight(zmk_studio_api::BacklightCommand::Set(
                        self.editor.zmk_draft.bl_value as u8,
                    ));
                    self.apply_write(keyboard, target, KeyAction::Zmk(behavior));
                }
            });
        }
    }
}

/// Returns true if the action is a Backlight Set command.
fn is_backlight_set(binding: &KeyAction) -> bool {
    matches!(
        binding,
        KeyAction::Zmk(Behavior::Backlight(zmk_studio_api::BacklightCommand::Set(
            _
        )))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zmk_studio_api::{MOD_LCTL, MOD_LSFT, MOD_RALT, MOD_RSFT};

    fn round_trip(behavior: Behavior) {
        let draft = ZmkDraft::from_behavior(&behavior);
        let rebuilt = draft.staged().expect("valid behavior draft must stage");
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
        assert_eq!(draft.staged(), Some(behavior));
    }
    #[test]
    fn single_mod_hold_stays_a_modifier_usage() {
        let draft = ZmkDraft {
            kind: ZmkBehaviorKind::ModTap,
            usage: Some(HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0)),
            hold_mods: MOD_RALT,
            ..Default::default()
        };
        match draft.staged().unwrap() {
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
        assert_eq!(
            kind_of(Behavior::MomentaryLayer { layer_id: 2 }),
            K::MomentaryLayer
        );
        assert_eq!(kind_of(Behavior::ToLayer { layer_id: 3 }), K::ToLayer);
        assert_eq!(kind_of(Behavior::Transparent), K::Transparent);
        assert_eq!(kind_of(Behavior::CapsWord), K::CapsWord);
        assert_eq!(
            kind_of(Behavior::Bluetooth(
                zmk_studio_api::BluetoothCommand::Select(2)
            )),
            K::Bluetooth
        );
        assert_eq!(
            kind_of(Behavior::MouseScroll { x: 0, y: 1 }),
            K::MouseScroll
        );
        assert_eq!(
            kind_of(Behavior::ExternalPower(
                zmk_studio_api::ExternalPowerCommand::Off
            )),
            K::KeyPress
        );
    }

    #[test]
    fn layer_tap_stages_layer_and_tap_side() {
        let draft = ZmkDraft::from_behavior(&Behavior::LayerTap {
            layer_id: 3,
            tap: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x1C, MOD_LSFT),
        });
        assert_eq!(draft.kind, ZmkBehaviorKind::LayerTap);
        assert_eq!(draft.layer_id, Some(3));
        assert_eq!(draft.usage.unwrap().id(), 0x1C);
        assert_eq!(draft.modifiers, MOD_LSFT);
    }

    #[test]
    fn mod_tap_requires_both_hold_and_tap() {
        let mut draft = ZmkDraft {
            kind: ZmkBehaviorKind::ModTap,
            usage: None,
            hold_mods: 0,
            ..Default::default()
        };
        assert_eq!(draft.staged(), None);
        assert!(!draft.is_valid());
        draft.hold_mods = MOD_LSFT;
        assert_eq!(draft.staged(), None); // Still missing tap usage
        assert!(!draft.is_valid());
        draft.usage = Some(HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0));
        assert_eq!(
            draft.staged(),
            Some(Behavior::ModTap {
                hold: HidUsage::from_modifier_mask(MOD_LSFT),
                tap: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0),
            })
        );
        assert!(draft.is_valid());
    }

    #[test]
    fn layer_tap_requires_both_layer_and_tap() {
        let mut draft = ZmkDraft {
            kind: ZmkBehaviorKind::LayerTap,
            usage: None,
            layer_id: None,
            ..Default::default()
        };
        assert_eq!(draft.staged(), None);
        assert!(!draft.is_valid());
        draft.layer_id = Some(2);
        assert_eq!(draft.staged(), None); // Still missing tap usage
        assert!(!draft.is_valid());
        draft.usage = Some(HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0));
        assert_eq!(
            draft.staged(),
            Some(Behavior::LayerTap {
                layer_id: 2,
                tap: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0),
            })
        );
        assert!(draft.is_valid());
    }

    #[test]
    fn key_press_requires_usage() {
        let mut draft = ZmkDraft {
            kind: ZmkBehaviorKind::KeyPress,
            usage: None,
            ..Default::default()
        };
        assert_eq!(draft.staged(), None);
        assert!(!draft.is_valid());
        draft.usage = Some(HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0));
        assert!(draft.staged().is_some());
        assert!(draft.is_valid());

        let direct_draft = ZmkDraft {
            kind: ZmkBehaviorKind::MomentaryLayer,
            ..Default::default()
        };
        assert!(direct_draft.is_valid());
    }

    #[test]
    fn default_draft_starts_empty() {
        let draft = ZmkDraft {
            kind: ZmkBehaviorKind::ModTap,
            ..Default::default()
        };
        assert_eq!(draft.usage, None);
        assert_eq!(draft.hold_mods, 0);
        assert_eq!(draft.modifiers, 0);
        assert!(!draft.is_valid());

        let kp_draft = ZmkDraft {
            kind: ZmkBehaviorKind::KeyPress,
            ..Default::default()
        };
        assert_eq!(kp_draft.usage, None);
        assert_eq!(kp_draft.modifiers, 0);
        assert!(!kp_draft.is_valid());
    }
}
