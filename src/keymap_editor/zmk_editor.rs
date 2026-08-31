//! ZMK behavior editor and parameter encoder.

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

/// Page categories for ZMK behaviors.
#[derive(Clone, Copy, PartialEq, Eq)]
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
    /// Base HID usage.
    pub usage: HidUsage,
    /// Modifier mask for key press behaviors.
    pub modifiers: u8,
    /// Held modifier mask for Mod-Tap behaviors.
    pub hold_mods: u8,
    /// Target layer ID for layer behaviors.
    pub layer_id: u32,
    /// Brightness level for Backlight Set commands.
    pub bl_value: u32,
    /// Indicates whether Backlight Set is staged.
    pub bl_set_staged: bool,
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
        }
    }
}

impl ZmkDraft {
    /// Creates draft state from an existing ZMK behavior.
    pub fn from_behavior(behavior: &Behavior) -> Self {
        let kind = behavior
            .role()
            .filter(|&r| Page::ALL.iter().any(|p| p.kinds().contains(&r)))
            .unwrap_or(ZmkBehaviorKind::KeyPress);
        let mut draft = ZmkDraft {
            kind,
            ..Default::default()
        };
        if let Some(layer_id) = behavior.layer_id() {
            draft.layer_id = layer_id;
        }
        match behavior {
            Behavior::KeyPress(usage) | Behavior::KeyToggle(usage) | Behavior::StickyKey(usage) => {
                draft.usage = usage.base();
                draft.modifiers = usage.modifiers();
            }
            Behavior::LayerTap { tap, .. } => {
                draft.usage = tap.base();
                draft.modifiers = tap.modifiers();
            }
            Behavior::ModTap { hold, tap } => {
                draft.hold_mods = hold.modifier_mask();
                draft.usage = tap.base();
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

    /// Encodes the current draft parameters into a Behavior.
    pub fn to_behavior(&self) -> Behavior {
        use ZmkBehaviorKind as K;
        let usage = HidUsage::from_parts(self.usage.page(), self.usage.id(), self.modifiers);
        match self.kind {
            K::KeyPress => Behavior::KeyPress(usage),
            K::KeyToggle => Behavior::KeyToggle(usage),
            K::StickyKey => Behavior::StickyKey(usage),
            K::ModTap => Behavior::ModTap {
                hold: HidUsage::from_modifier_mask(self.hold_mods),
                tap: usage,
            },
            K::LayerTap => Behavior::LayerTap {
                layer_id: self.layer_id,
                tap: usage,
            },
            _ => unreachable!("only staged kinds encode from the draft"),
        }
    }

    /// Returns the encoded Behavior if all required parameters are valid.
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

        // If the current draft kind is not supported on this keyboard, switch to the first supported one.
        if !keyboard.is_action_supported(&KeyAction::Zmk(zmk_catalog::sample_behavior(
            self.editor.zmk_draft.kind,
        ))) {
            self.editor.zmk_draft.kind = Page::ALL
                .iter()
                .find_map(|page| page.supported_kinds(keyboard).next())
                .unwrap_or(ZmkBehaviorKind::KeyPress);
            self.editor.zmk_draft.bl_set_staged = false;
        }

        // Two-pane layout: every behavior kind lives in the left panel under
        // its group header; the right pane holds the selected page. The
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
                        // Switching behavior kinds leaves any staged
                        // Backlight Set behind.
                        if response.changed() {
                            self.editor.zmk_draft.bl_set_staged = false;
                        }
                    }
                } else {
                    let first_kind = *kinds.peek().unwrap();
                    let response = ui.selectable_label(
                        page_of(self.editor.zmk_draft.kind) == page,
                        page.label(),
                    );
                    if response.clicked() {
                        self.editor.zmk_draft.kind = first_kind;
                        self.editor.zmk_draft.bl_set_staged = false;
                    }
                }
                ui.add_space(4.0);
            }
        });

        let current_page = page_of(self.editor.zmk_draft.kind);
        super::editor_central_panel(ui, |ui| {
            match current_page {
                Page::Special | Page::Commands => {
                    // Every parameterless behavior and command option is a
                    // key here; clicking applies it directly.
                    self.draw_direct_grid(ui, keyboard, target);
                }
                Page::Keys => {
                    // One argument, one group: the usage's modifier toggles
                    // and key grid are tightly coupled, so they share the
                    // boundary.
                    titled_group(ui, "Key", |ui| {
                        self.draw_usage_picker(ui, keyboard, target, true);
                    });
                }
                Page::Layers => {
                    // One page of grouped layer keys; see draw_zmk_layer_page.
                    self.draw_zmk_layer_page(ui, keyboard, target, &layer_infos);
                }
                Page::Mods => {
                    // Two distinct arguments, two groups: the hold-side
                    // modifier (single choice for standard ZMK &mt), and
                    // the tap-side usage (whose own modifier toggles stay
                    // inside the tap group).
                    let mod_style = self.paint_style(KEY_UNIT);
                    titled_group(ui, "Hold modifier", |ui| {
                        modifier_toggle_grid(
                            ui,
                            "zmk_hold_mod",
                            self.editor.zmk_draft.hold_mods,
                            &mod_style,
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
                        self.draw_usage_picker(ui, keyboard, target, true);
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

    /// Draws the HID usage picker grid.
    fn draw_usage_picker(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        with_mods: bool,
    ) {
        let selected = KeyAction::Zmk(Behavior::KeyPress(self.editor.zmk_draft.usage.base()));
        let style = self.paint_style(KEY_UNIT);

        if with_mods {
            let mod_style = self.paint_style(KEY_UNIT);
            modifier_toggle_grid(
                ui,
                "zmk_mods",
                self.editor.zmk_draft.modifiers,
                &mod_style,
                |mask| {
                    self.editor.zmk_draft.modifiers ^= mask;
                    self.commit_zmk_draft(keyboard, target);
                },
            );
        }

        candidate_groups_rows(
            ui,
            zmk_catalog::categories(),
            |c| keyboard.is_action_supported(&c.binding),
            |_| Some(selected.clone()),
            &style,
            |_, candidate| {
                if let KeyAction::Zmk(Behavior::KeyPress(usage)) = &candidate.binding {
                    self.editor.zmk_draft.usage = HidUsage::from_parts(
                        usage.page(),
                        usage.id(),
                        self.editor.zmk_draft.modifiers,
                    );
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
    ) {
        let layer_names: Vec<String> = layer_infos
            .iter()
            .map(|info| info.name.clone().unwrap_or_default())
            .collect();
        let tap = HidUsage::from_parts(
            self.editor.zmk_draft.usage.page(),
            self.editor.zmk_draft.usage.id(),
            self.editor.zmk_draft.modifiers,
        );
        let style = self.paint_style(KEY_UNIT);
        let kinds: Vec<_> = Page::Layers.supported_kinds(keyboard).collect();
        let groups = zmk_catalog::layer_groups(&kinds, layer_infos, &layer_names, tap);

        let staged_tap = if self.editor.zmk_draft.kind == ZmkBehaviorKind::LayerTap {
            Some(KeyAction::Zmk(Behavior::LayerTap {
                layer_id: self.editor.zmk_draft.layer_id,
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
                    if let Some(layer_id) = behavior.layer_id() {
                        self.editor.zmk_draft.kind = ZmkBehaviorKind::LayerTap;
                        self.editor.zmk_draft.layer_id = layer_id;
                        self.commit_zmk_draft(keyboard, target);
                    }
                } else {
                    self.editor.zmk_draft.kind = kind;
                    if let Some(layer_id) = behavior.layer_id() {
                        self.editor.zmk_draft.layer_id = layer_id;
                    }
                    self.apply_write(keyboard, target, candidate.binding.clone());
                }
            }
        });

        if self.editor.zmk_draft.kind == ZmkBehaviorKind::LayerTap {
            titled_group(ui, "Tap key", |ui| {
                self.draw_usage_picker(ui, keyboard, target, true);
            });
        }
    }

    /// Draws direct-apply behavior grids for command and special keys.
    fn draw_direct_grid(&mut self, ui: &mut egui::Ui, keyboard: &Keyboard, target: EditTarget) {
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
        let style = self.paint_style(KEY_UNIT);
        titled_group(ui, grid_title, |ui| {
            picker_grid_rows(
                ui,
                kind.label(),
                &candidates,
                keyboard
                    .get_action(target.layer_index, target.row, target.col)
                    .as_ref(),
                &style,
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
            kind_of(Behavior::Bluetooth(
                zmk_studio_api::BluetoothCommand::Select(2)
            )),
            K::Bluetooth
        );
        assert_eq!(
            kind_of(Behavior::MouseScroll { x: 0, y: 1 }),
            K::MouseScroll
        );
        // Behaviors not assignable through ZMK Studio (e.g. ExternalPower) fall back to KeyPress.
        assert_eq!(
            kind_of(Behavior::ExternalPower(
                zmk_studio_api::ExternalPowerCommand::Off
            )),
            K::KeyPress
        );
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
    fn mod_tap_without_hold_modifier_is_invalid() {
        let mut draft = ZmkDraft {
            kind: ZmkBehaviorKind::ModTap,
            hold_mods: 0,
            ..Default::default()
        };
        assert_eq!(draft.staged(), None);
        draft.hold_mods = MOD_LSFT;
        assert_eq!(
            draft.staged(),
            Some(Behavior::ModTap {
                hold: HidUsage::from_modifier_mask(MOD_LSFT),
                tap: draft.usage.base(),
            })
        );
    }
}
