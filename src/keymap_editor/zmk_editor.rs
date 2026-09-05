//! ZMK behavior editor and parameter encoder.

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use zmk_studio_api::{Behavior, HidUsage, HID_USAGE_KEYBOARD};

use super::picker::{
    framed_candidate_groups, modifier_toggle_grid, multi_candidate_groups, titled_candidate_group,
    Candidate, CandidateGroup, SelectedKey,
};
use super::zmk_catalog::{self, ZmkBehaviorKind};
use super::{EditTarget, EditorState};
use crate::ui_widgets::titled_group;

/// Editor sections and left sidebar items for ZMK behaviors.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Section {
    KeyPress,
    KeyToggle,
    StickyKey,
    Layers,
    ModTap,
    Bluetooth,
    OutputSelection,
    Backlight,
    Underglow,
    MouseKeyPress,
    MouseMove,
    MouseScroll,
    System,
    Special,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Self::KeyPress => "Key Press",
            Self::KeyToggle => "Key Toggle",
            Self::StickyKey => "Sticky Key",
            Self::Layers => "Layers",
            Self::ModTap => "Mod-Tap",
            Self::Bluetooth => "Bluetooth",
            Self::OutputSelection => "Output Selection",
            Self::Backlight => "Backlight",
            Self::Underglow => "Underglow",
            Self::MouseKeyPress => "Mouse Key",
            Self::MouseMove => "Mouse Move",
            Self::MouseScroll => "Mouse Scroll",
            Self::System => "System",
            Self::Special => "Special",
        }
    }

    fn from_kind(kind: ZmkBehaviorKind) -> Self {
        use ZmkBehaviorKind::*;
        match kind {
            KeyPress => Self::KeyPress,
            KeyToggle => Self::KeyToggle,
            StickyKey => Self::StickyKey,
            MomentaryLayer | ToggleLayer | ToLayer | StickyLayer | LayerTap => Self::Layers,
            ModTap => Self::ModTap,
            Bluetooth => Self::Bluetooth,
            OutputSelection => Self::OutputSelection,
            Backlight => Self::Backlight,
            Underglow => Self::Underglow,
            MouseKeyPress => Self::MouseKeyPress,
            MouseMove => Self::MouseMove,
            MouseScroll => Self::MouseScroll,
            Reset | Bootloader | SoftOff | StudioUnlock | ExternalPower => Self::System,
            Transparent | None | CapsWord | KeyRepeat | GraveEscape => Self::Special,
        }
    }

    fn default_kind(self) -> ZmkBehaviorKind {
        use ZmkBehaviorKind::*;
        match self {
            Self::KeyPress => KeyPress,
            Self::KeyToggle => KeyToggle,
            Self::StickyKey => StickyKey,
            Self::Layers => MomentaryLayer,
            Self::ModTap => ModTap,
            Self::Bluetooth => Bluetooth,
            Self::OutputSelection => OutputSelection,
            Self::Backlight => Backlight,
            Self::Underglow => Underglow,
            Self::MouseKeyPress => MouseKeyPress,
            Self::MouseMove => MouseMove,
            Self::MouseScroll => MouseScroll,
            Self::System => Reset,
            Self::Special => Transparent,
        }
    }

    fn kinds(self) -> &'static [ZmkBehaviorKind] {
        use ZmkBehaviorKind::*;
        match self {
            Self::KeyPress => &[KeyPress],
            Self::KeyToggle => &[KeyToggle],
            Self::StickyKey => &[StickyKey],
            Self::Layers => &[MomentaryLayer, ToggleLayer, ToLayer, StickyLayer, LayerTap],
            Self::ModTap => &[ModTap],
            Self::Bluetooth => &[Bluetooth],
            Self::OutputSelection => &[OutputSelection],
            Self::Backlight => &[Backlight],
            Self::Underglow => &[Underglow],
            Self::MouseKeyPress => &[MouseKeyPress],
            Self::MouseMove => &[MouseMove],
            Self::MouseScroll => &[MouseScroll],
            Self::System => &[Reset, Bootloader, SoftOff, StudioUnlock, ExternalPower],
            Self::Special => &[Transparent, None, CapsWord, KeyRepeat, GraveEscape],
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

    fn is_supported(self, keyboard: &Keyboard) -> bool {
        self.supported_kinds(keyboard).next().is_some()
    }
}

impl super::SidebarItem for Section {
    fn label(self) -> &'static str {
        Section::label(self)
    }

    fn is_supported(self, keyboard: &Keyboard) -> bool {
        Section::is_supported(self, keyboard)
    }
}

const ZMK_SECTIONS: [super::SidebarSection<Section>; 6] = [
    super::SidebarSection {
        title: "Keys",
        items: &[Section::KeyPress, Section::KeyToggle, Section::StickyKey],
    },
    super::SidebarSection {
        title: "Layers & Mods",
        items: &[Section::Layers, Section::ModTap],
    },
    super::SidebarSection {
        title: "Wireless",
        items: &[Section::Bluetooth, Section::OutputSelection],
    },
    super::SidebarSection {
        title: "Lighting",
        items: &[Section::Backlight, Section::Underglow],
    },
    super::SidebarSection {
        title: "Mouse",
        items: &[
            Section::MouseKeyPress,
            Section::MouseMove,
            Section::MouseScroll,
        ],
    },
    super::SidebarSection {
        title: "Other",
        items: &[Section::System, Section::Special],
    },
];

/// Backlight command parameters for staged backlight adjustment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BacklightDraft {
    pub value: u8,
    pub staged: bool,
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
    /// Parameter state for backlight adjustment commands.
    pub backlight: BacklightDraft,
}

impl Default for ZmkDraft {
    fn default() -> Self {
        Self {
            kind: ZmkBehaviorKind::KeyPress,
            usage: None,
            modifiers: 0,
            hold_mods: 0,
            layer_id: None,
            backlight: BacklightDraft::default(),
        }
    }
}

impl ZmkDraft {
    /// Initializes draft for a specific kind, preserving the active behavior if it matches.
    pub(super) fn for_kind(kind: ZmkBehaviorKind, current_action: Option<&KeyAction>) -> Self {
        match current_action {
            Some(KeyAction::Zmk(b)) if b.role() == Some(kind) => Self::from_behavior(b),
            _ => Self {
                kind,
                ..Default::default()
            },
        }
    }

    /// Initializes draft for a section, preserving the active behavior if it belongs to this section.
    fn for_section(section: Section, current_action: Option<&KeyAction>) -> Self {
        match current_action {
            Some(KeyAction::Zmk(b)) if b.role().map(Section::from_kind) == Some(section) => {
                Self::from_behavior(b)
            }
            _ => Self {
                kind: section.default_kind(),
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
        let kind = behavior.role().unwrap_or(ZmkBehaviorKind::KeyPress);
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
                draft.backlight.value = *value;
                draft.backlight.staged = true;
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

impl EditorState {
    /// Applies the current draft behavior to the target key.
    fn commit_zmk_draft(&mut self, keyboard: &Keyboard, target: EditTarget) {
        let staged = self.zmk_draft.staged().map(KeyAction::Zmk);
        self.commit_staged(keyboard, target, staged);
    }

    pub(super) fn draw_zmk_editor_body(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        // If the current draft kind is not supported on this keyboard, switch to the first supported one.
        if !keyboard.is_action_supported(&KeyAction::Zmk(zmk_catalog::sample_behavior(
            self.zmk_draft.kind,
        ))) {
            if let Some(first) = ZMK_SECTIONS
                .iter()
                .flat_map(|s| s.items.iter())
                .filter(|s| s.is_supported(keyboard))
                .map(|s| s.default_kind())
                .next()
            {
                self.zmk_draft.kind = first;
            }
            self.zmk_draft.backlight.staged = false;
        }

        let current_section = Section::from_kind(self.zmk_draft.kind);
        if let Some(section) = super::editor_left_panel(
            ui,
            "zmk_kinds",
            keyboard,
            current_section,
            &ZMK_SECTIONS,
            &mut self.search_query,
        ) {
            self.search_query.clear();
            let current_action = target.action(keyboard);
            match section {
                Section::Layers | Section::System | Section::Special => {
                    self.zmk_draft = ZmkDraft::for_section(section, current_action.as_ref());
                }
                _ => {
                    self.reset_zmk_draft_for_kind(keyboard, target, section.default_kind());
                }
            }
        }

        let current_section = Section::from_kind(self.zmk_draft.kind);
        let is_valid = self.zmk_draft.is_valid();
        let search_query = self.search_query.clone();
        super::editor_central_panel(ui, (target.layer_index, current_section), |ui| {
            match current_section {
                Section::KeyPress | Section::KeyToggle | Section::StickyKey => {
                    titled_group(ui, self.zmk_draft.kind.label(), |ui| {
                        self.draw_usage_picker(
                            ui,
                            keyboard,
                            target,
                            &search_query,
                            is_valid,
                            style,
                        );
                    });
                }
                Section::Layers => {
                    self.draw_zmk_layer_page(ui, keyboard, target, &search_query, is_valid, style);
                }
                Section::ModTap => {
                    titled_group(ui, "Hold modifier", |ui| {
                        modifier_toggle_grid(
                            ui,
                            "zmk_hold_mod",
                            self.zmk_draft.hold_mods,
                            is_valid,
                            style,
                            |mask| {
                                self.zmk_draft.hold_mods = if self.zmk_draft.hold_mods == mask {
                                    0
                                } else {
                                    mask
                                };
                                self.commit_zmk_draft(keyboard, target);
                            },
                        );
                    });
                    titled_group(ui, "Tap key", |ui| {
                        self.draw_usage_picker(
                            ui,
                            keyboard,
                            target,
                            &search_query,
                            is_valid,
                            style,
                        );
                    });
                    if self.zmk_draft.kind == ZmkBehaviorKind::ModTap
                        && self.zmk_draft.hold_mods == 0
                    {
                        ui.weak("Select a hold modifier.");
                    }
                }
                _ => {
                    self.draw_direct_grid(ui, keyboard, target, &search_query, style);
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
        let current_action = target.action(keyboard);
        self.reset_zmk_kind(kind, current_action.as_ref());
    }

    /// Draws the HID usage picker grid.
    fn draw_usage_picker(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        search_query: &str,
        valid: bool,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let action = self
            .zmk_draft
            .usage
            .map(|u| KeyAction::Zmk(Behavior::KeyPress(u.base())));
        let selected = action.as_ref().map(|a| SelectedKey::new(a, valid));

        modifier_toggle_grid(
            ui,
            "zmk_mods",
            self.zmk_draft.modifiers,
            valid,
            style,
            |mask| {
                self.zmk_draft.modifiers ^= mask;
                self.commit_zmk_draft(keyboard, target);
            },
        );

        multi_candidate_groups(
            ui,
            zmk_catalog::categories(),
            search_query,
            |c| keyboard.is_action_supported(&c.binding),
            selected,
            style,
            |_, candidate| {
                if let KeyAction::Zmk(Behavior::KeyPress(usage)) = &candidate.binding {
                    self.zmk_draft.usage = Some(usage.base());
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
        search_query: &str,
        valid: bool,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let layer_infos = keyboard.layer_infos();
        let layer_names: Vec<String> = layer_infos
            .iter()
            .map(|info| info.name.clone().unwrap_or_default())
            .collect();
        let tap = self
            .zmk_draft
            .tap_usage()
            .unwrap_or_else(|| HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0));
        let kinds: Vec<_> = Section::Layers.supported_kinds(keyboard).collect();
        let groups = zmk_catalog::layer_groups(&kinds, &layer_infos, &layer_names, tap);

        let is_lt = self.zmk_draft.kind == ZmkBehaviorKind::LayerTap;
        let lt_action = is_lt
            .then(|| {
                self.zmk_draft
                    .layer_id
                    .map(|id| KeyAction::Zmk(Behavior::LayerTap { layer_id: id, tap }))
            })
            .flatten();
        let current_action = target.action(keyboard);
        let selected = if is_lt {
            lt_action.as_ref().map(|a| SelectedKey::new(a, valid))
        } else {
            current_action.as_ref().map(SelectedKey::valid)
        };

        framed_candidate_groups(
            ui,
            &groups,
            search_query,
            |_| true,
            selected,
            style,
            |gi, candidate| {
                let kind = kinds[gi];
                if let KeyAction::Zmk(behavior) = &candidate.binding {
                    if kind == ZmkBehaviorKind::LayerTap {
                        if let Some(layer_id) = behavior.layer_id() {
                            self.zmk_draft.kind = ZmkBehaviorKind::LayerTap;
                            self.zmk_draft.layer_id = Some(layer_id);
                            self.commit_zmk_draft(keyboard, target);
                        }
                    } else {
                        self.zmk_draft.kind = kind;
                        self.zmk_draft.layer_id = behavior.layer_id();
                        self.apply_write(keyboard, target, candidate.binding.clone());
                    }
                }
            },
        );

        if self.zmk_draft.kind == ZmkBehaviorKind::LayerTap {
            titled_group(ui, "Tap key", |ui| {
                self.draw_usage_picker(ui, keyboard, target, search_query, valid, style);
            });
        }
    }

    /// Draws direct-apply behavior grids for command and special keys.
    fn draw_direct_grid(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        target: EditTarget,
        search_query: &str,
        style: &crate::key_paint::KeyPaintStyle,
    ) {
        let kind = self.zmk_draft.kind;
        let staging_set = kind == ZmkBehaviorKind::Backlight && self.zmk_draft.backlight.staged;
        let current_section = Section::from_kind(kind);
        let candidates: Vec<Candidate> = match current_section {
            Section::System => Section::System
                .supported_kinds(keyboard)
                .map(|k| zmk_catalog::behavior_candidate(&zmk_catalog::sample_behavior(k), &[]))
                .collect(),
            Section::Special => Section::Special
                .supported_kinds(keyboard)
                .map(|k| zmk_catalog::behavior_candidate(&zmk_catalog::sample_behavior(k), &[]))
                .collect(),
            _ => zmk_catalog::command_candidates(kind, self.zmk_draft.backlight.value)
                .into_iter()
                .filter(|c| keyboard.is_action_supported(&c.binding))
                .collect(),
        };
        let grid_title = match current_section {
            Section::System => "System",
            Section::Special => "Special",
            _ => kind.label(),
        };
        let action = target.action(keyboard);
        let selected = action.as_ref().map(SelectedKey::valid);
        let group = CandidateGroup {
            name: grid_title,
            candidates,
        };
        titled_candidate_group(
            ui,
            &group,
            search_query,
            |_| true,
            selected,
            style,
            |candidate| {
                if is_backlight_set(&candidate.binding) {
                    self.zmk_draft.backlight.staged = true;
                    return;
                }
                self.zmk_draft.backlight.staged = false;
                self.apply_write(keyboard, target, candidate.binding.clone());
            },
        );

        if staging_set {
            titled_group(ui, "Level", |ui| {
                let level = ui
                    .add(egui::DragValue::new(&mut self.zmk_draft.backlight.value).range(0..=255));
                if level.drag_stopped() || level.lost_focus() {
                    let behavior = Behavior::Backlight(zmk_studio_api::BacklightCommand::Set(
                        self.zmk_draft.backlight.value,
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
            K::ExternalPower
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
