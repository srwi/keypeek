//! ZMK editor content: behavior-based editing with per-behavior parameters,
//! plus the session Save/Discard flow.

use crate::key_action::KeyAction;
use crate::keyboard::Keyboard;
use zmk_studio_api::{
    Behavior, HidUsage, HID_USAGE_KEYBOARD, MOD_LALT, MOD_LCTL, MOD_LGUI, MOD_LSFT,
};

use super::picker::picker_grid_rows;
use super::zmk_catalog;
use super::{EditTarget, PendingKind};

/// The eight dedicated modifier keycodes offered as a Mod-Tap hold side.
const MODIFIER_KEYCODES: [(u16, &str); 8] = [
    (0xE0, "LCTL"),
    (0xE1, "LSFT"),
    (0xE2, "LALT"),
    (0xE3, "LGUI"),
    (0xE4, "RCTL"),
    (0xE5, "RSFT"),
    (0xE6, "RALT"),
    (0xE7, "RGUI"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ZmkBehaviorKind {
    KeyPress,
    KeyToggle,
    StickyKey,
    MomentaryLayer,
    ToggleLayer,
    ToLayer,
    StickyLayer,
    LayerTap,
    ModTap,
    Transparent,
    NoneBehavior,
    CapsWord,
    KeyRepeat,
    GraveEscape,
    StudioUnlock,
    Reset,
    Bootloader,
    SoftOff,
    Bluetooth,
    OutputSelection,
    Backlight,
    Underglow,
    MouseKeyPress,
    MouseMove,
    MouseScroll,
}

impl ZmkBehaviorKind {
    fn label(&self) -> &'static str {
        use ZmkBehaviorKind::*;
        match self {
            KeyPress => "Key Press",
            KeyToggle => "Key Toggle",
            StickyKey => "Sticky Key",
            MomentaryLayer => "Momentary Layer",
            ToggleLayer => "Toggle Layer",
            ToLayer => "To Layer",
            StickyLayer => "Sticky Layer",
            LayerTap => "Layer-Tap",
            ModTap => "Mod-Tap",
            Transparent => "Transparent",
            NoneBehavior => "None",
            CapsWord => "Caps Word",
            KeyRepeat => "Key Repeat",
            GraveEscape => "Grave Escape",
            StudioUnlock => "Studio Unlock",
            Reset => "Reset",
            Bootloader => "Bootloader",
            SoftOff => "Soft Off",
            Bluetooth => "Bluetooth",
            OutputSelection => "Output Selection",
            Backlight => "Backlight",
            Underglow => "Underglow",
            MouseKeyPress => "Mouse Key",
            MouseMove => "Mouse Move",
            MouseScroll => "Mouse Scroll",
        }
    }
}

/// Which ZMK behaviors carry parameters (and so need an Apply button); the
/// rest apply on selection.
fn needs_params(kind: ZmkBehaviorKind) -> bool {
    use ZmkBehaviorKind::*;
    matches!(
        kind,
        KeyPress
            | KeyToggle
            | StickyKey
            | MomentaryLayer
            | ToggleLayer
            | ToLayer
            | StickyLayer
            | LayerTap
            | ModTap
            | Bluetooth
            | OutputSelection
            | Backlight
            | Underglow
            | MouseKeyPress
            | MouseMove
            | MouseScroll
    )
}

const KEY_LIST: &[ZmkBehaviorKind] = &[
    ZmkBehaviorKind::KeyPress,
    ZmkBehaviorKind::KeyToggle,
    ZmkBehaviorKind::StickyKey,
];
const LAYER_LIST: &[ZmkBehaviorKind] = &[
    ZmkBehaviorKind::MomentaryLayer,
    ZmkBehaviorKind::ToggleLayer,
    ZmkBehaviorKind::ToLayer,
    ZmkBehaviorKind::StickyLayer,
    ZmkBehaviorKind::LayerTap,
];
const MOD_LIST: &[ZmkBehaviorKind] = &[ZmkBehaviorKind::ModTap];
const NO_PARAM_LIST: &[ZmkBehaviorKind] = &[
    ZmkBehaviorKind::Transparent,
    ZmkBehaviorKind::NoneBehavior,
    ZmkBehaviorKind::CapsWord,
    ZmkBehaviorKind::KeyRepeat,
    ZmkBehaviorKind::GraveEscape,
    ZmkBehaviorKind::StudioUnlock,
    ZmkBehaviorKind::Reset,
    ZmkBehaviorKind::Bootloader,
    ZmkBehaviorKind::SoftOff,
];
const COMMAND_LIST: &[ZmkBehaviorKind] = &[
    ZmkBehaviorKind::Bluetooth,
    ZmkBehaviorKind::OutputSelection,
    ZmkBehaviorKind::Backlight,
    ZmkBehaviorKind::Underglow,
    ZmkBehaviorKind::MouseKeyPress,
    ZmkBehaviorKind::MouseMove,
    ZmkBehaviorKind::MouseScroll,
];

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
    pub bt_command: u32,
    pub bt_profile: u32,
    pub out_value: u32,
    pub bl_command: u32,
    pub bl_value: u32,
    pub rgb_command: u32,
    pub mouse_button: u32,
    pub mouse_direction: u32,
}

impl Default for ZmkDraft {
    fn default() -> Self {
        Self {
            kind: ZmkBehaviorKind::KeyPress,
            usage: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0),
            modifiers: 0,
            hold_mod: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0xE1, 0),
            layer_id: 0,
            bt_command: 3,
            bt_profile: 0,
            out_value: 1,
            bl_command: 0,
            bl_value: 0,
            rgb_command: 0,
            mouse_button: 1,
            mouse_direction: 0,
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
            Behavior::Bluetooth { command, value } => {
                draft.bt_command = *command;
                draft.bt_profile = *value;
                K::Bluetooth
            }
            Behavior::OutputSelection { value } => {
                draft.out_value = *value;
                K::OutputSelection
            }
            Behavior::Backlight { command, value } => {
                draft.bl_command = *command;
                draft.bl_value = *value;
                K::Backlight
            }
            Behavior::Underglow { command, .. } => {
                draft.rgb_command = *command;
                K::Underglow
            }
            Behavior::MouseKeyPress { value } => {
                draft.mouse_button = *value;
                K::MouseKeyPress
            }
            Behavior::MouseMove { value } => {
                draft.mouse_direction = *value;
                K::MouseMove
            }
            Behavior::MouseScroll { value } => {
                draft.mouse_direction = *value;
                K::MouseScroll
            }
            Behavior::Custom { .. } | Behavior::Unknown { .. } | Behavior::ExternalPower { .. } => {
                K::KeyPress
            }
        };
        draft
    }

    /// Builds the `Behavior` the draft currently describes.
    pub fn to_behavior(&self) -> Option<Behavior> {
        use ZmkBehaviorKind as K;
        let usage = |d: &ZmkDraft| HidUsage::from_parts(d.usage.page(), d.usage.id(), d.modifiers);
        let result = match self.kind {
            K::KeyPress => Behavior::KeyPress(usage(self)),
            K::KeyToggle => Behavior::KeyToggle(usage(self)),
            K::StickyKey => Behavior::StickyKey(usage(self)),
            K::MomentaryLayer => Behavior::MomentaryLayer {
                layer_id: self.layer_id,
            },
            K::ToggleLayer => Behavior::ToggleLayer {
                layer_id: self.layer_id,
            },
            K::ToLayer => Behavior::ToLayer {
                layer_id: self.layer_id,
            },
            K::StickyLayer => Behavior::StickyLayer {
                layer_id: self.layer_id,
            },
            K::LayerTap => Behavior::LayerTap {
                layer_id: self.layer_id,
                tap: usage(self),
            },
            K::ModTap => Behavior::ModTap {
                hold: self.hold_mod,
                tap: usage(self),
            },
            K::Transparent => Behavior::Transparent,
            K::NoneBehavior => Behavior::None,
            K::CapsWord => Behavior::CapsWord,
            K::KeyRepeat => Behavior::KeyRepeat,
            K::GraveEscape => Behavior::GraveEscape,
            K::StudioUnlock => Behavior::StudioUnlock,
            K::Reset => Behavior::Reset,
            K::Bootloader => Behavior::Bootloader,
            K::SoftOff => Behavior::SoftOff,
            K::Bluetooth => Behavior::Bluetooth {
                command: self.bt_command,
                value: self.bt_profile,
            },
            K::OutputSelection => Behavior::OutputSelection {
                value: self.out_value,
            },
            K::Backlight => Behavior::Backlight {
                command: self.bl_command,
                value: self.bl_value,
            },
            K::Underglow => Behavior::Underglow {
                command: self.rgb_command,
                value: 0,
            },
            K::MouseKeyPress => Behavior::MouseKeyPress {
                value: self.mouse_button,
            },
            K::MouseMove => Behavior::MouseMove {
                value: self.mouse_direction,
            },
            K::MouseScroll => Behavior::MouseScroll {
                value: self.mouse_direction,
            },
        };
        Some(result)
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

        egui::ComboBox::from_id_salt("zmk_behavior_combo")
            .selected_text(draft.kind.label())
            .show_ui(ui, |ui| {
                ui.label("Keys");
                for kind in KEY_LIST {
                    ui.selectable_value(&mut draft.kind, *kind, kind.label());
                }
                ui.separator();
                ui.label("Layers");
                for kind in LAYER_LIST {
                    ui.selectable_value(&mut draft.kind, *kind, kind.label());
                }
                ui.separator();
                ui.label("Mods");
                for kind in MOD_LIST {
                    ui.selectable_value(&mut draft.kind, *kind, kind.label());
                }
                ui.separator();
                ui.label("No parameters");
                for kind in NO_PARAM_LIST {
                    ui.selectable_value(&mut draft.kind, *kind, kind.label());
                }
                ui.separator();
                ui.label("Commands");
                for kind in COMMAND_LIST {
                    ui.selectable_value(&mut draft.kind, *kind, kind.label());
                }
            });
        ui.separator();

        match draft.kind {
            ZmkBehaviorKind::KeyPress | ZmkBehaviorKind::KeyToggle | ZmkBehaviorKind::StickyKey => {
                self.draw_usage_picker(ui, draft, true);
            }
            ZmkBehaviorKind::MomentaryLayer
            | ZmkBehaviorKind::ToggleLayer
            | ZmkBehaviorKind::ToLayer
            | ZmkBehaviorKind::StickyLayer => {
                self.draw_layer_selector(ui, &layer_infos, &mut draft.layer_id);
            }
            ZmkBehaviorKind::LayerTap => {
                self.draw_layer_selector(ui, &layer_infos, &mut draft.layer_id);
                ui.label("Tap key:");
                self.draw_usage_picker(ui, draft, true);
            }
            ZmkBehaviorKind::ModTap => {
                ui.label("Hold modifier:");
                egui::ComboBox::from_id_salt("hold_mod_combo")
                    .selected_text(modifier_label(draft.hold_mod))
                    .show_ui(ui, |ui| {
                        for (id, name) in MODIFIER_KEYCODES {
                            ui.selectable_value(
                                &mut draft.hold_mod,
                                HidUsage::from_parts(HID_USAGE_KEYBOARD, id, 0),
                                name,
                            );
                        }
                    });
                ui.label("Tap key:");
                self.draw_usage_picker(ui, draft, true);
            }
            ZmkBehaviorKind::Transparent
            | ZmkBehaviorKind::NoneBehavior
            | ZmkBehaviorKind::CapsWord
            | ZmkBehaviorKind::KeyRepeat
            | ZmkBehaviorKind::GraveEscape
            | ZmkBehaviorKind::StudioUnlock
            | ZmkBehaviorKind::Reset
            | ZmkBehaviorKind::Bootloader
            | ZmkBehaviorKind::SoftOff => {}
            ZmkBehaviorKind::Bluetooth => {
                ui.horizontal(|ui| {
                    ui.label("Command");
                    egui::ComboBox::from_id_salt("bt_command_combo")
                        .selected_text(bt_command_label(draft.bt_command))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut draft.bt_command, 0, "Clear");
                            ui.selectable_value(&mut draft.bt_command, 1, "Next");
                            ui.selectable_value(&mut draft.bt_command, 2, "Previous");
                            ui.selectable_value(&mut draft.bt_command, 3, "Select");
                            ui.selectable_value(&mut draft.bt_command, 4, "Clear All");
                            ui.selectable_value(&mut draft.bt_command, 5, "Disconnect");
                        });
                });
                if draft.bt_command == 3 || draft.bt_command == 5 {
                    ui.horizontal(|ui| {
                        ui.label("Profile");
                        ui.add(egui::DragValue::new(&mut draft.bt_profile).range(0..=9));
                    });
                }
            }
            ZmkBehaviorKind::OutputSelection => {
                ui.horizontal(|ui| {
                    ui.label("Output");
                    egui::ComboBox::from_id_salt("out_combo")
                        .selected_text(output_label(draft.out_value))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut draft.out_value, 0, "Toggle");
                            ui.selectable_value(&mut draft.out_value, 1, "USB");
                            ui.selectable_value(&mut draft.out_value, 2, "BLE");
                            ui.selectable_value(&mut draft.out_value, 3, "None");
                        });
                });
            }
            ZmkBehaviorKind::Backlight => {
                self.draw_command_selector(
                    ui,
                    "Command",
                    &mut draft.bl_command,
                    &[
                        (0, "On"),
                        (1, "Off"),
                        (2, "Toggle"),
                        (3, "Increase"),
                        (4, "Decrease"),
                        (5, "Cycle"),
                        (6, "Set"),
                    ],
                );
                if draft.bl_command == 6 {
                    ui.horizontal(|ui| {
                        ui.label("Level");
                        ui.add(egui::DragValue::new(&mut draft.bl_value).range(0..=255));
                    });
                }
            }
            ZmkBehaviorKind::Underglow => {
                self.draw_command_selector(
                    ui,
                    "Command",
                    &mut draft.rgb_command,
                    &[
                        (0, "Toggle"),
                        (1, "On"),
                        (2, "Off"),
                        (3, "Hue +"),
                        (4, "Hue -"),
                        (5, "Saturation +"),
                        (6, "Saturation -"),
                        (7, "Brightness +"),
                        (8, "Brightness -"),
                        (9, "Speed +"),
                        (10, "Speed -"),
                        (11, "Effect +"),
                        (12, "Effect -"),
                        (13, "Effect Set"),
                        (14, "Color"),
                    ],
                );
            }
            ZmkBehaviorKind::MouseKeyPress => {
                self.draw_command_selector(
                    ui,
                    "Button",
                    &mut draft.mouse_button,
                    &[
                        (1, "Left"),
                        (2, "Right"),
                        (4, "Middle"),
                        (8, "Mouse 4"),
                        (16, "Mouse 5"),
                    ],
                );
            }
            ZmkBehaviorKind::MouseMove => {
                self.draw_command_selector(
                    ui,
                    "Direction",
                    &mut draft.mouse_direction,
                    &[
                        (0x0001_0000, "Up"),
                        (0xFFFF_FFFF, "Down"),
                        (0x0000_0001, "Right"),
                        (0x0000_FFFF, "Left"),
                    ],
                );
            }
            ZmkBehaviorKind::MouseScroll => {
                self.draw_command_selector(
                    ui,
                    "Direction",
                    &mut draft.mouse_direction,
                    &[
                        (0x0000_0001, "Up"),
                        (0x0000_FFFF, "Down"),
                        (0x0001_0000, "Right"),
                        (0xFFFF_FFFF, "Left"),
                    ],
                );
            }
        }

        if needs_params(draft.kind) {
            ui.add_space(8.0);
            if ui.button("Apply").clicked() {
                if let Some(behavior) = draft.to_behavior() {
                    self.apply_zmk_write(keyboard, target, behavior);
                }
            }
        } else if ui.button("Apply").clicked() {
            if let Some(behavior) = draft.to_behavior() {
                self.apply_zmk_write(keyboard, target, behavior);
            }
        }
    }

    fn draw_usage_picker(&mut self, ui: &mut egui::Ui, draft: &mut ZmkDraft, with_mods: bool) {
        // A compact modifier row above the picker.
        if with_mods {
            ui.horizontal(|ui| {
                let mut ctrl = draft.modifiers & MOD_LCTL != 0;
                let mut shift = draft.modifiers & MOD_LSFT != 0;
                let mut alt = draft.modifiers & MOD_LALT != 0;
                let mut gui = draft.modifiers & MOD_LGUI != 0;
                if ui.checkbox(&mut ctrl, "Ctrl").changed()
                    || ui.checkbox(&mut shift, "Shift").changed()
                    || ui.checkbox(&mut alt, "Alt").changed()
                    || ui.checkbox(&mut gui, "Gui").changed()
                {
                    draft.modifiers = 0;
                    draft.modifiers |= if ctrl { MOD_LCTL } else { 0 };
                    draft.modifiers |= if shift { MOD_LSFT } else { 0 };
                    draft.modifiers |= if alt { MOD_LALT } else { 0 };
                    draft.modifiers |= if gui { MOD_LGUI } else { 0 };
                }
            });
        }

        let selected =
            zmk_studio_api::HidUsage::from_parts(draft.usage.page(), draft.usage.id(), 0)
                .to_hid_usage();

        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                for category in zmk_catalog::categories() {
                    ui.label(category.name);
                    picker_grid_rows(
                        ui,
                        category.name,
                        &category.candidates,
                        Some(selected),
                        |code| {
                            let usage = HidUsage::from_encoded(code);
                            draft.usage =
                                HidUsage::from_parts(usage.page(), usage.id(), draft.modifiers);
                        },
                    );
                    ui.add_space(6.0);
                }
            });
    }

    fn draw_layer_selector(
        &mut self,
        ui: &mut egui::Ui,
        layer_infos: &[crate::key_action::LayerInfo],
        layer_id: &mut u32,
    ) {
        ui.horizontal(|ui| {
            ui.label("Layer");
            egui::ComboBox::from_id_salt("zmk_layer_combo")
                .selected_text(layer_label(layer_infos, *layer_id))
                .show_ui(ui, |ui| {
                    for (i, info) in layer_infos.iter().enumerate() {
                        let label = format!("{i}: {}", info.name.clone().unwrap_or_default());
                        ui.selectable_value(layer_id, info.id, label);
                    }
                });
        });
    }

    fn draw_command_selector(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        value: &mut u32,
        options: &[(u32, &str)],
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            egui::ComboBox::from_id_salt((ui.id(), label))
                .selected_text(
                    options
                        .iter()
                        .find(|(v, _)| *v == *value)
                        .map(|(_, name)| *name)
                        .unwrap_or(&format!("{value}")),
                )
                .show_ui(ui, |ui| {
                    for (v, name) in options {
                        ui.selectable_value(value, *v, *name);
                    }
                });
        });
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

fn layer_label(layer_infos: &[crate::key_action::LayerInfo], layer_id: u32) -> String {
    layer_infos
        .iter()
        .position(|info| info.id == layer_id)
        .map(|i| format!("{i}: {}", layer_infos[i].name.clone().unwrap_or_default()))
        .unwrap_or_else(|| format!("Layer {layer_id}"))
}

fn modifier_label(usage: HidUsage) -> String {
    MODIFIER_KEYCODES
        .iter()
        .find(|(id, _)| *id == usage.id())
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| format!("0x{:02X}", usage.id()))
}

fn bt_command_label(command: u32) -> &'static str {
    match command {
        0 => "Clear",
        1 => "Next",
        2 => "Previous",
        3 => "Select",
        4 => "Clear All",
        5 => "Disconnect",
        _ => "Unknown",
    }
}

fn output_label(value: u32) -> &'static str {
    match value {
        0 => "Toggle",
        1 => "USB",
        2 => "BLE",
        3 => "None",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(behavior: Behavior) {
        let draft = ZmkDraft::from_behavior(&behavior);
        let rebuilt = draft.to_behavior().expect("draft should encode");
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
    fn layer_behaviors_round_trip() {
        round_trip(Behavior::MomentaryLayer { layer_id: 2 });
        round_trip(Behavior::ToggleLayer { layer_id: 0 });
        round_trip(Behavior::ToLayer { layer_id: 3 });
        round_trip(Behavior::StickyLayer { layer_id: 1 });
    }

    #[test]
    fn layer_tap_and_mod_tap_round_trip() {
        round_trip(Behavior::LayerTap {
            layer_id: 1,
            tap: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x1C, 0),
        });
        round_trip(Behavior::ModTap {
            hold: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0xE1, 0),
            tap: HidUsage::from_parts(HID_USAGE_KEYBOARD, 0x04, 0),
        });
    }

    #[test]
    fn parameterless_behaviors_round_trip() {
        for behavior in [
            Behavior::Transparent,
            Behavior::None,
            Behavior::CapsWord,
            Behavior::KeyRepeat,
            Behavior::GraveEscape,
            Behavior::StudioUnlock,
            Behavior::Reset,
            Behavior::Bootloader,
            Behavior::SoftOff,
        ] {
            round_trip(behavior);
        }
    }

    #[test]
    fn command_behaviors_round_trip() {
        round_trip(Behavior::Bluetooth {
            command: 3,
            value: 2,
        });
        round_trip(Behavior::OutputSelection { value: 1 });
        round_trip(Behavior::Backlight {
            command: 6,
            value: 3,
        });
        round_trip(Behavior::Underglow {
            command: 7,
            value: 0,
        });
        round_trip(Behavior::MouseKeyPress { value: 1 });
        round_trip(Behavior::MouseMove { value: 0x0001_0000 });
        round_trip(Behavior::MouseScroll { value: 0x0000_0001 });
    }

    #[test]
    fn layer_parameter_uses_the_stable_layer_id() {
        // Stage 0 recorded ids == indices on the test board; the dropdown
        // writes `LayerInfo::id`, and the draft carries it verbatim.
        let draft = ZmkDraft {
            kind: ZmkBehaviorKind::MomentaryLayer,
            layer_id: 4,
            ..Default::default()
        };
        assert_eq!(
            draft.to_behavior(),
            Some(Behavior::MomentaryLayer { layer_id: 4 })
        );
    }
}
