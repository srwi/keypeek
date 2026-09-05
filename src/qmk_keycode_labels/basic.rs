use crate::layout_key::{Label, LayoutKey};

use qmk_via_api::keycodes::Keycode;

pub fn get_basic_layout_key(keycode_bytes: u16) -> Option<LayoutKey> {
    let keycode = Keycode::try_from(keycode_bytes).ok()?;

    if let Some(key) = get_basic_layout_key_static(keycode) {
        return Some(key);
    }

    if keycode_bytes <= 0x00A4 || (0x00E0..=0x00E7).contains(&keycode_bytes) {
        return crate::hid_labels::hid_keyboard_key(keycode_bytes);
    }

    None
}

fn get_basic_layout_key_static(keycode: Keycode) -> Option<LayoutKey> {
    match keycode {
        Keycode::KC_SYSTEM_POWER => crate::hid_labels::hid_system_key(0x81),
        Keycode::KC_SYSTEM_SLEEP => crate::hid_labels::hid_system_key(0x82),
        Keycode::KC_SYSTEM_WAKE => crate::hid_labels::hid_system_key(0x83),
        Keycode::KC_AUDIO_MUTE => crate::hid_labels::hid_consumer_key(0xE2),
        Keycode::KC_AUDIO_VOL_UP => crate::hid_labels::hid_consumer_key(0xE9),
        Keycode::KC_AUDIO_VOL_DOWN => crate::hid_labels::hid_consumer_key(0xEA),
        Keycode::KC_MEDIA_NEXT_TRACK => crate::hid_labels::hid_consumer_key(0xB5),
        Keycode::KC_MEDIA_PREV_TRACK => crate::hid_labels::hid_consumer_key(0xB6),
        Keycode::KC_MEDIA_STOP => crate::hid_labels::hid_consumer_key(0xB7),
        Keycode::KC_MEDIA_PLAY_PAUSE => crate::hid_labels::hid_consumer_key(0xCD),
        Keycode::KC_MEDIA_SELECT => Some(LayoutKey {
            tap: Label::with_short("Select", "Sel"),
            ..Default::default()
        }),
        Keycode::KC_MEDIA_EJECT => crate::hid_labels::hid_consumer_key(0xB8),
        Keycode::KC_MAIL => crate::hid_labels::hid_consumer_key(0x18A),
        Keycode::KC_CALCULATOR => crate::hid_labels::hid_consumer_key(0x192),
        Keycode::KC_MY_COMPUTER => crate::hid_labels::hid_consumer_key(0x194),
        Keycode::KC_WWW_SEARCH => crate::hid_labels::hid_consumer_key(0x221),
        Keycode::KC_WWW_HOME => crate::hid_labels::hid_consumer_key(0x223),
        Keycode::KC_WWW_BACK => crate::hid_labels::hid_consumer_key(0x224),
        Keycode::KC_WWW_FORWARD => crate::hid_labels::hid_consumer_key(0x225),
        Keycode::KC_WWW_STOP => crate::hid_labels::hid_consumer_key(0x226),
        Keycode::KC_WWW_REFRESH => crate::hid_labels::hid_consumer_key(0x227),
        Keycode::KC_WWW_FAVORITES => crate::hid_labels::hid_consumer_key(0x22A),
        Keycode::KC_MEDIA_FAST_FORWARD => crate::hid_labels::hid_consumer_key(0xB3),
        Keycode::KC_MEDIA_REWIND => crate::hid_labels::hid_consumer_key(0xB4),
        Keycode::KC_BRIGHTNESS_UP => crate::hid_labels::hid_consumer_key(0x6F),
        Keycode::KC_BRIGHTNESS_DOWN => crate::hid_labels::hid_consumer_key(0x70),
        Keycode::KC_CONTROL_PANEL => crate::hid_labels::hid_consumer_key(0x19F),
        Keycode::KC_ASSISTANT => Some(LayoutKey {
            tap: Label::with_short("Assistant", "Asst"),
            ..Default::default()
        }),
        Keycode::KC_MISSION_CONTROL => Some(LayoutKey {
            tap: Label::with_short("Mission Control", "MC"),
            ..Default::default()
        }),
        Keycode::KC_LAUNCHPAD => Some(LayoutKey {
            tap: Label::with_short("Launchpad", "LP"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_CURSOR_UP => Some(LayoutKey {
            tap: Label::new(egui_phosphor::regular::ARROW_UP),
            symbol: Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_CURSOR_DOWN => Some(LayoutKey {
            tap: Label::new(egui_phosphor::regular::ARROW_DOWN),
            symbol: Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_CURSOR_LEFT => Some(LayoutKey {
            tap: Label::new(egui_phosphor::regular::ARROW_LEFT),
            symbol: Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_CURSOR_RIGHT => Some(LayoutKey {
            tap: Label::new(egui_phosphor::regular::ARROW_RIGHT),
            symbol: Some(egui_phosphor::regular::MOUSE_SIMPLE.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_1 => Some(LayoutKey {
            tap: Label::new(""),
            symbol: Some(egui_phosphor::regular::MOUSE_LEFT_CLICK.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_2 => Some(LayoutKey {
            tap: Label::new(""),
            symbol: Some(egui_phosphor::regular::MOUSE_RIGHT_CLICK.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_3 => Some(LayoutKey {
            tap: Label::new(""),
            symbol: Some(egui_phosphor::regular::MOUSE_MIDDLE_CLICK.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_4 => Some(LayoutKey {
            tap: Label::new("Mouse Btn4"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_5 => Some(LayoutKey {
            tap: Label::new("Mouse Btn5"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_6 => Some(LayoutKey {
            tap: Label::new("Mouse Btn6"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_7 => Some(LayoutKey {
            tap: Label::new("Mouse Btn7"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_BUTTON_8 => Some(LayoutKey {
            tap: Label::new("Mouse Btn8"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_WHEEL_UP => Some(LayoutKey {
            tap: Label::new(egui_phosphor::regular::ARROW_UP),
            symbol: Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_WHEEL_DOWN => Some(LayoutKey {
            tap: Label::new(egui_phosphor::regular::ARROW_DOWN),
            symbol: Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_WHEEL_LEFT => Some(LayoutKey {
            tap: Label::new(egui_phosphor::regular::ARROW_LEFT),
            symbol: Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_WHEEL_RIGHT => Some(LayoutKey {
            tap: Label::new(egui_phosphor::regular::ARROW_RIGHT),
            symbol: Some(egui_phosphor::regular::MOUSE_SCROLL.to_string()),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_ACCELERATION_0 => Some(LayoutKey {
            tap: Label::new("Mouse Acc0"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_ACCELERATION_1 => Some(LayoutKey {
            tap: Label::new("Mouse Acc1"),
            ..Default::default()
        }),
        Keycode::QK_MOUSE_ACCELERATION_2 => Some(LayoutKey {
            tap: Label::new("Mouse Acc2"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Swap Hands Toggle", "SwpHT"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_TAP_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Swap Hands Tap Toggle", "SwpTT"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_MOMENTARY_ON => Some(LayoutKey {
            tap: Label::with_short("Swap Hands On", "SwpOn"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_MOMENTARY_OFF => Some(LayoutKey {
            tap: Label::with_short("Swap Hands Off", "SwpOff"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_OFF => Some(LayoutKey {
            tap: Label::with_short("Swap Hands Off", "SwpOff"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_ON => Some(LayoutKey {
            tap: Label::with_short("Swap Hands On", "SwpOn"),
            ..Default::default()
        }),
        Keycode::QK_SWAP_HANDS_ONE_SHOT => Some(LayoutKey {
            tap: Label::with_short("Swap Hands One Shot", "SwpOS"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_CONTROL_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Swap Ctrl Caps", "SwCtCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_CONTROL_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Unswap Ctrl Caps", "UnCtCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_CONTROL_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Toggle Ctrl Caps", "TgCtCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_CAPS_LOCK_AS_CONTROL_OFF => Some(LayoutKey {
            tap: Label::with_short("Caps as Ctrl Off", "CpCtOf"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_CAPS_LOCK_AS_CONTROL_ON => Some(LayoutKey {
            tap: Label::with_short("Caps as Ctrl On", "CpCtOn"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_LALT_LGUI => Some(LayoutKey {
            tap: Label::with_short("Swap LAlt LGui", "SwAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_LALT_LGUI => Some(LayoutKey {
            tap: Label::with_short("Unswap LAlt LGui", "UnAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_RALT_RGUI => Some(LayoutKey {
            tap: Label::with_short("Swap RAlt RGui", "SwAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_RALT_RGUI => Some(LayoutKey {
            tap: Label::with_short("Unswap RAlt RGui", "UnAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_GUI_ON => Some(LayoutKey {
            tap: Label::with_short("GUI On", "GuiOn"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_GUI_OFF => Some(LayoutKey {
            tap: Label::with_short("GUI Off", "GuiOff"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_GUI => Some(LayoutKey {
            tap: Label::with_short("Toggle GUI", "TgGui"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_GRAVE_ESC => Some(LayoutKey {
            tap: Label::with_short("Swap ` Esc", "Sw`Esc"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_GRAVE_ESC => Some(LayoutKey {
            tap: Label::with_short("Unswap ` Esc", "Un`Esc"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_BACKSLASH_BACKSPACE => Some(LayoutKey {
            tap: Label::with_short("Swap \\ Bksp", "Sw\\Bk"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_BACKSLASH_BACKSPACE => Some(LayoutKey {
            tap: Label::with_short("Unswap \\ Bksp", "Un\\Bk"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_BACKSLASH_BACKSPACE => Some(LayoutKey {
            tap: Label::with_short("Toggle \\ Bksp", "Tg\\Bk"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_NKRO_ON => Some(LayoutKey {
            tap: Label::with_short("NKRO On", "NKROOn"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_NKRO_OFF => Some(LayoutKey {
            tap: Label::with_short("NKRO Off", "NKROOf"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_NKRO => Some(LayoutKey {
            tap: Label::with_short("Toggle NKRO", "NKRO"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_ALT_GUI => Some(LayoutKey {
            tap: Label::with_short("Swap Alt GUI", "SwAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_ALT_GUI => Some(LayoutKey {
            tap: Label::with_short("Unswap Alt GUI", "UnAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_ALT_GUI => Some(LayoutKey {
            tap: Label::with_short("Toggle Alt GUI", "TgAltG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_LCTL_LGUI => Some(LayoutKey {
            tap: Label::with_short("Swap LCtl LGui", "SwCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_LCTL_LGUI => Some(LayoutKey {
            tap: Label::with_short("Unswap LCtl LGui", "UnCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_RCTL_RGUI => Some(LayoutKey {
            tap: Label::with_short("Swap RCtl RGui", "SwCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_RCTL_RGUI => Some(LayoutKey {
            tap: Label::with_short("Unswap RCtl RGui", "UnCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_CTL_GUI => Some(LayoutKey {
            tap: Label::with_short("Swap Ctl GUI", "SwCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_CTL_GUI => Some(LayoutKey {
            tap: Label::with_short("Unswap Ctl GUI", "UnCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_CTL_GUI => Some(LayoutKey {
            tap: Label::with_short("Toggle Ctl GUI", "TgCtlG"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_EE_HANDS_LEFT => Some(LayoutKey {
            tap: Label::with_short("EE Hands Left", "EEHndL"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_EE_HANDS_RIGHT => Some(LayoutKey {
            tap: Label::with_short("EE Hands Right", "EEHndR"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_SWAP_ESCAPE_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Swap Esc Caps", "SwEsCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_UNSWAP_ESCAPE_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Unswap Esc Caps", "UnEsCp"),
            ..Default::default()
        }),
        Keycode::QK_MAGIC_TOGGLE_ESCAPE_CAPS_LOCK => Some(LayoutKey {
            tap: Label::with_short("Toggle Esc Caps", "TgEsCp"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_ON => Some(LayoutKey {
            tap: Label::with_short("MIDI On", "MDOn"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OFF => Some(LayoutKey {
            tap: Label::with_short("MIDI Off", "MDOff"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("MIDI Toggle", "MDTg"),
            ..Default::default()
        }),
        k if (Keycode::QK_MIDI_NOTE_C_0 as u16..=Keycode::QK_MIDI_NOTE_B_5 as u16)
            .contains(&(k as u16)) =>
        {
            const NOTE_NAMES: [&str; 12] = [
                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
            ];
            let offset = (k as u16) - (Keycode::QK_MIDI_NOTE_C_0 as u16);
            let octave = offset / 12;
            let note = NOTE_NAMES[(offset % 12) as usize];
            Some(LayoutKey {
                tap: Label::with_short(format!("MIDI {note}{octave}"), format!("MD{note}{octave}")),
                ..Default::default()
            })
        }
        k if (Keycode::QK_MIDI_OCTAVE_N2 as u16..=Keycode::QK_MIDI_OCTAVE_7 as u16)
            .contains(&(k as u16)) =>
        {
            let oct = (k as i32) - (Keycode::QK_MIDI_OCTAVE_0 as i32);
            Some(LayoutKey {
                tap: Label::with_short(format!("MIDI Oct {oct}"), format!("MDO{oct}")),
                ..Default::default()
            })
        }
        Keycode::QK_MIDI_OCTAVE_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct Down", "MDO-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_OCTAVE_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Oct Up", "MDO+"),
            ..Default::default()
        }),
        k if (Keycode::QK_MIDI_TRANSPOSE_N6 as u16..=Keycode::QK_MIDI_TRANSPOSE_6 as u16)
            .contains(&(k as u16)) =>
        {
            let step = (k as i32) - (Keycode::QK_MIDI_TRANSPOSE_0 as i32);
            Some(LayoutKey {
                tap: Label::with_short(format!("MIDI Trans {step}"), format!("MDT{step}")),
                ..Default::default()
            })
        }
        Keycode::QK_MIDI_TRANSPOSE_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans Down", "MDT-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_TRANSPOSE_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Trans Up", "MDT+"),
            ..Default::default()
        }),
        k if (Keycode::QK_MIDI_VELOCITY_0 as u16..=Keycode::QK_MIDI_VELOCITY_10 as u16)
            .contains(&(k as u16)) =>
        {
            let vel = (k as u16) - (Keycode::QK_MIDI_VELOCITY_0 as u16);
            Some(LayoutKey {
                tap: Label::with_short(format!("MIDI Vel {vel}"), format!("MDV{vel}")),
                ..Default::default()
            })
        }
        Keycode::QK_MIDI_VELOCITY_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel Down", "MDV-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_VELOCITY_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Vel Up", "MDV+"),
            ..Default::default()
        }),
        k if (Keycode::QK_MIDI_CHANNEL_1 as u16..=Keycode::QK_MIDI_CHANNEL_16 as u16)
            .contains(&(k as u16)) =>
        {
            let ch = (k as u16) - (Keycode::QK_MIDI_CHANNEL_1 as u16) + 1;
            Some(LayoutKey {
                tap: Label::with_short(format!("MIDI Ch {ch}"), format!("MDC{ch}")),
                ..Default::default()
            })
        }
        Keycode::QK_MIDI_CHANNEL_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch Down", "MDC-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_CHANNEL_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Ch Up", "MDC+"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_ALL_NOTES_OFF => Some(LayoutKey {
            tap: Label::with_short("MIDI All Off", "MDAOff"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_SUSTAIN => Some(LayoutKey {
            tap: Label::with_short("MIDI Sustain", "MDSus"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_PORTAMENTO => Some(LayoutKey {
            tap: Label::with_short("MIDI Portamento", "MDPort"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_SOSTENUTO => Some(LayoutKey {
            tap: Label::with_short("MIDI Sostenuto", "MDSost"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_SOFT => Some(LayoutKey {
            tap: Label::with_short("MIDI Soft", "MDSoft"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_LEGATO => Some(LayoutKey {
            tap: Label::with_short("MIDI Legato", "MDLeg"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_MODULATION => Some(LayoutKey {
            tap: Label::with_short("MIDI Modulation", "MDMod"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_MODULATION_SPEED_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Mod Speed -", "MDM-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_MODULATION_SPEED_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Mod Speed +", "MDM+"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_PITCH_BEND_DOWN => Some(LayoutKey {
            tap: Label::with_short("MIDI Pitch -", "MDP-"),
            ..Default::default()
        }),
        Keycode::QK_MIDI_PITCH_BEND_UP => Some(LayoutKey {
            tap: Label::with_short("MIDI Pitch +", "MDP+"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_ON => Some(LayoutKey {
            tap: Label::with_short("Sequencer On", "SeqOn"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_OFF => Some(LayoutKey {
            tap: Label::with_short("Sequencer Off", "SeqOff"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Sequencer Toggle", "SeqTg"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_TEMPO_DOWN => Some(LayoutKey {
            tap: Label::with_short("Seq Tempo -", "SeqT-"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_TEMPO_UP => Some(LayoutKey {
            tap: Label::with_short("Seq Tempo +", "SeqT+"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_RESOLUTION_DOWN => Some(LayoutKey {
            tap: Label::with_short("Seq Res -", "SeqR-"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_RESOLUTION_UP => Some(LayoutKey {
            tap: Label::with_short("Seq Res +", "SeqR+"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_STEPS_ALL => Some(LayoutKey {
            tap: Label::with_short("Seq All Steps", "SeqAll"),
            ..Default::default()
        }),
        Keycode::QK_SEQUENCER_STEPS_CLEAR => Some(LayoutKey {
            tap: Label::with_short("Seq Clear Steps", "SeqClr"),
            ..Default::default()
        }),
        k if (Keycode::QK_JOYSTICK_BUTTON_0 as u16..=Keycode::QK_JOYSTICK_BUTTON_31 as u16)
            .contains(&(k as u16)) =>
        {
            let n = (k as u16) - (Keycode::QK_JOYSTICK_BUTTON_0 as u16);
            Some(LayoutKey {
                tap: Label::with_short(format!("Joy Btn {n}"), format!("JoyB{n}")),
                ..Default::default()
            })
        }
        k if (Keycode::QK_PROGRAMMABLE_BUTTON_1 as u16
            ..=Keycode::QK_PROGRAMMABLE_BUTTON_32 as u16)
            .contains(&(k as u16)) =>
        {
            let n = (k as u16) - (Keycode::QK_PROGRAMMABLE_BUTTON_1 as u16) + 1;
            Some(LayoutKey {
                tap: Label::with_short(format!("Prog Btn {n}"), format!("PB{n}")),
                ..Default::default()
            })
        }
        Keycode::QK_AUDIO_ON => Some(LayoutKey {
            tap: Label::with_short("Audio On", "AudOn"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_OFF => Some(LayoutKey {
            tap: Label::with_short("Audio Off", "AudOff"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Audio Toggle", "AudTg"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Clicky Toggle", "ClkTg"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_ON => Some(LayoutKey {
            tap: Label::with_short("Clicky Enable", "ClkOn"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_OFF => Some(LayoutKey {
            tap: Label::with_short("Clicky Disable", "ClkOff"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_UP => Some(LayoutKey {
            tap: Label::with_short("Clicky Up", "Clk+"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_DOWN => Some(LayoutKey {
            tap: Label::with_short("Clicky Down", "Clk-"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_CLICKY_RESET => Some(LayoutKey {
            tap: Label::with_short("Clicky Reset", "ClkRst"),
            ..Default::default()
        }),
        Keycode::QK_MUSIC_ON => Some(LayoutKey {
            tap: Label::with_short("Music On", "MusicOn"),
            ..Default::default()
        }),
        Keycode::QK_MUSIC_OFF => Some(LayoutKey {
            tap: Label::with_short("Music Off", "MusicOf"),
            ..Default::default()
        }),
        Keycode::QK_MUSIC_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Music Toggle", "MusicTg"),
            ..Default::default()
        }),
        Keycode::QK_MUSIC_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("Music Mode", "MusicMd"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_VOICE_NEXT => Some(LayoutKey {
            tap: Label::with_short("Voice Next", "Voice+"),
            ..Default::default()
        }),
        Keycode::QK_AUDIO_VOICE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("Voice Prev", "Voice-"),
            ..Default::default()
        }),
        Keycode::QK_STENO_BOLT => Some(LayoutKey {
            tap: Label::with_short("Steno Bolt", "StBolt"),
            ..Default::default()
        }),
        Keycode::QK_STENO_GEMINI => Some(LayoutKey {
            tap: Label::with_short("Steno Gemini", "StGem"),
            ..Default::default()
        }),
        Keycode::QK_STENO_COMB => Some(LayoutKey {
            tap: Label::with_short("Steno Comb", "StComb"),
            ..Default::default()
        }),
        Keycode::QK_STENO_COMB_MAX => Some(LayoutKey {
            tap: Label::with_short("Steno Comb Max", "StCMax"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_ON => Some(LayoutKey {
            tap: Label::new("BL On"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_OFF => Some(LayoutKey {
            tap: Label::new("BL Off"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("BL Toggle", "BLTog"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_DOWN => Some(LayoutKey {
            tap: Label::with_short("BL Dec", "BL-"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_UP => Some(LayoutKey {
            tap: Label::with_short("BL Inc", "BL+"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_STEP => Some(LayoutKey {
            tap: Label::with_short("BL Cycle", "BLCyc"),
            ..Default::default()
        }),
        Keycode::QK_BACKLIGHT_TOGGLE_BREATHING => Some(LayoutKey {
            tap: Label::with_short("BL Breathe", "BLBr"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_ON => Some(LayoutKey {
            tap: Label::with_short("LED On", "LEDOn"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_OFF => Some(LayoutKey {
            tap: Label::with_short("LED Off", "LEDOff"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("RGB Toggle", "RGBTg"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_MODE_NEXT => Some(LayoutKey {
            tap: Label::new("RGB Mode +"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::new("RGB Mode -"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_BRIGHTNESS_UP => Some(LayoutKey {
            tap: Label::with_short("LED Bri +", "LED+"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_BRIGHTNESS_DOWN => Some(LayoutKey {
            tap: Label::with_short("LED Bri -", "LED-"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_SPEED_UP => Some(LayoutKey {
            tap: Label::with_short("LED Spd +", "LEDSp+"),
            ..Default::default()
        }),
        Keycode::QK_LED_MATRIX_SPEED_DOWN => Some(LayoutKey {
            tap: Label::with_short("LED Spd -", "LEDSp-"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("UG Toggle", "UGTg"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("UG Mode +", "UGM+"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("UG Mode -", "UGM-"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_HUE_UP => Some(LayoutKey {
            tap: Label::with_short("Hue +", "Hue+"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_HUE_DOWN => Some(LayoutKey {
            tap: Label::with_short("Hue -", "Hue-"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_SATURATION_UP => Some(LayoutKey {
            tap: Label::with_short("Sat +", "Sat+"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_SATURATION_DOWN => Some(LayoutKey {
            tap: Label::with_short("Sat -", "Sat-"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_VALUE_UP => Some(LayoutKey {
            tap: Label::with_short("Bright +", "Bri+"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_VALUE_DOWN => Some(LayoutKey {
            tap: Label::with_short("Bright -", "Bri-"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_SPEED_UP => Some(LayoutKey {
            tap: Label::with_short("Speed +", "Spd+"),
            ..Default::default()
        }),
        Keycode::QK_UNDERGLOW_SPEED_DOWN => Some(LayoutKey {
            tap: Label::with_short("Speed -", "Spd-"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_PLAIN => Some(LayoutKey {
            tap: Label::new("RGB Mode P"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_BREATHE => Some(LayoutKey {
            tap: Label::new("RGB Mode B"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_RAINBOW => Some(LayoutKey {
            tap: Label::new("RGB Mode R"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_SWIRL => Some(LayoutKey {
            tap: Label::new("RGB Mode SW"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_SNAKE => Some(LayoutKey {
            tap: Label::new("RGB Mode SN"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_KNIGHT => Some(LayoutKey {
            tap: Label::new("RGB Mode K"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_XMAS => Some(LayoutKey {
            tap: Label::new("RGB Mode X"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_GRADIENT => Some(LayoutKey {
            tap: Label::new("RGB Mode G"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_RGBTEST => Some(LayoutKey {
            tap: Label::new("RGB Mode Test"),
            ..Default::default()
        }),
        Keycode::RGB_MODE_TWINKLE => Some(LayoutKey {
            tap: Label::new("RGB Mode T"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_ON => Some(LayoutKey {
            tap: Label::with_short("RGB On", "RGBOn"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_OFF => Some(LayoutKey {
            tap: Label::with_short("RGB Off", "RGBOff"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("RGB Toggle", "RGBTg"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("RGB Mode +", "RGBM+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("RGB Mode -", "RGBM-"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_HUE_UP => Some(LayoutKey {
            tap: Label::with_short("RGB Hue +", "RGBH+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_HUE_DOWN => Some(LayoutKey {
            tap: Label::with_short("RGB Hue -", "RGBH-"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_SATURATION_UP => Some(LayoutKey {
            tap: Label::with_short("RGB Sat +", "RGBS+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_SATURATION_DOWN => Some(LayoutKey {
            tap: Label::with_short("RGB Sat -", "RGBS-"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_VALUE_UP => Some(LayoutKey {
            tap: Label::with_short("RGB Val +", "RGBV+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_VALUE_DOWN => Some(LayoutKey {
            tap: Label::with_short("RGB Val -", "RGBV-"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_SPEED_UP => Some(LayoutKey {
            tap: Label::with_short("RGB Spd +", "RGBSp+"),
            ..Default::default()
        }),
        Keycode::QK_RGB_MATRIX_SPEED_DOWN => Some(LayoutKey {
            tap: Label::with_short("RGB Spd -", "RGBSp-"),
            ..Default::default()
        }),
        Keycode::QK_BOOTLOADER => Some(LayoutKey {
            tap: Label::with_short("Bootloader", "Boot"),
            ..Default::default()
        }),
        Keycode::QK_REBOOT => Some(LayoutKey {
            tap: Label::with_short("Reboot", "Reboot"),
            ..Default::default()
        }),
        Keycode::QK_DEBUG_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Debug Toggle", "DbgTg"),
            ..Default::default()
        }),
        Keycode::QK_CLEAR_EEPROM => Some(LayoutKey {
            tap: Label::with_short("Clear EEPROM", "ClrEE"),
            ..Default::default()
        }),
        Keycode::QK_MAKE => Some(LayoutKey {
            tap: Label::with_short("Make", "Make"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_DOWN => Some(LayoutKey {
            tap: Label::with_short("AutoShift -", "AS -"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_UP => Some(LayoutKey {
            tap: Label::with_short("AutoShift +", "AS +"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_REPORT => Some(LayoutKey {
            tap: Label::with_short("AutoShift Rep", "AS R"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_ON => Some(LayoutKey {
            tap: Label::with_short("AutoShift On", "AS On"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_OFF => Some(LayoutKey {
            tap: Label::with_short("AutoShift Off", "ASOff"),
            ..Default::default()
        }),
        Keycode::QK_AUTO_SHIFT_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("AutoShift Tog", "AS Tg"),
            ..Default::default()
        }),
        Keycode::QK_GRAVE_ESCAPE => Some(LayoutKey {
            tap: Label::new("Esc `"),
            ..Default::default()
        }),
        Keycode::QK_VELOCIKEY_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Velocikey", "VelKey"),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_LEFT_CTRL_PARENTHESIS_OPEN => Some(LayoutKey {
            tap: Label::new("LC ("),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_RIGHT_CTRL_PARENTHESIS_CLOSE => Some(LayoutKey {
            tap: Label::new("RC )"),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_LEFT_SHIFT_PARENTHESIS_OPEN => Some(LayoutKey {
            tap: Label::new("LS ("),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_RIGHT_SHIFT_PARENTHESIS_CLOSE => Some(LayoutKey {
            tap: Label::new("RS )"),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_LEFT_ALT_PARENTHESIS_OPEN => Some(LayoutKey {
            tap: Label::new("LA ("),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_RIGHT_ALT_PARENTHESIS_CLOSE => Some(LayoutKey {
            tap: Label::new("RA )"),
            ..Default::default()
        }),
        Keycode::QK_SPACE_CADET_RIGHT_SHIFT_ENTER => Some(LayoutKey {
            tap: Label::new("SftEnt"),
            ..Default::default()
        }),
        Keycode::QK_OUTPUT_AUTO => Some(LayoutKey {
            tap: Label::with_short("Out Auto", "OutAuto"),
            ..Default::default()
        }),
        Keycode::QK_OUTPUT_USB => Some(LayoutKey {
            tap: Label::with_short("Out USB", "OutUSB"),
            ..Default::default()
        }),
        Keycode::QK_OUTPUT_BLUETOOTH => Some(LayoutKey {
            tap: Label::with_short("Out BT", "OutBT"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("Unicode +", "Uni +"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("Unicode -", "Uni -"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_MACOS => Some(LayoutKey {
            tap: Label::with_short("Unicode macOS", "UniMac"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_LINUX => Some(LayoutKey {
            tap: Label::with_short("Unicode Linux", "UniLnx"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_WINDOWS => Some(LayoutKey {
            tap: Label::with_short("Unicode Win", "UniWin"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_BSD => Some(LayoutKey {
            tap: Label::with_short("Unicode BSD", "UniBSD"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_WINCOMPOSE => Some(LayoutKey {
            tap: Label::with_short("Unicode WinC", "UniWinC"),
            ..Default::default()
        }),
        Keycode::QK_UNICODE_MODE_EMACS => Some(LayoutKey {
            tap: Label::with_short("Unicode Emacs", "UniEm"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_ON => Some(LayoutKey {
            tap: Label::with_short("Haptic On", "HapOn"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_OFF => Some(LayoutKey {
            tap: Label::with_short("Haptic Off", "HapOff"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Haptic Toggle", "HapTg"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_RESET => Some(LayoutKey {
            tap: Label::with_short("Haptic Reset", "HapRst"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_FEEDBACK_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Haptic FB Tog", "HapFBTg"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_BUZZ_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Haptic Buzz", "HapBuzz"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_MODE_NEXT => Some(LayoutKey {
            tap: Label::with_short("Haptic +", "Hap +"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_MODE_PREVIOUS => Some(LayoutKey {
            tap: Label::with_short("Haptic -", "Hap -"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_CONTINUOUS_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Haptic Cont", "HapCont"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_CONTINUOUS_UP => Some(LayoutKey {
            tap: Label::with_short("Haptic + ", "HapC+"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_CONTINUOUS_DOWN => Some(LayoutKey {
            tap: Label::with_short("Haptic -", "HapC-"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_DWELL_UP => Some(LayoutKey {
            tap: Label::with_short("Haptic Dwell +", "HapDw+"),
            ..Default::default()
        }),
        Keycode::QK_HAPTIC_DWELL_DOWN => Some(LayoutKey {
            tap: Label::with_short("Haptic Dwell -", "HapDw-"),
            ..Default::default()
        }),
        Keycode::QK_COMBO_ON => Some(LayoutKey {
            tap: Label::with_short("Combo On", "CombOn"),
            ..Default::default()
        }),
        Keycode::QK_COMBO_OFF => Some(LayoutKey {
            tap: Label::with_short("Combo Off", "CombOff"),
            ..Default::default()
        }),
        Keycode::QK_COMBO_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Combo Toggle", "CombTg"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_RECORD_START_1 => Some(LayoutKey {
            tap: Label::with_short("DM Rec 1", "DMRec1"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_RECORD_START_2 => Some(LayoutKey {
            tap: Label::with_short("DM Rec 2", "DMRec2"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_RECORD_STOP => Some(LayoutKey {
            tap: Label::with_short("DM Stop", "DMStop"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_PLAY_1 => Some(LayoutKey {
            tap: Label::with_short("DM Play 1", "DMPlay1"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_MACRO_PLAY_2 => Some(LayoutKey {
            tap: Label::with_short("DM Play 2", "DMPlay2"),
            ..Default::default()
        }),
        Keycode::QK_LEADER => Some(LayoutKey {
            tap: Label::with_short("Leader", "Lead"),
            ..Default::default()
        }),
        Keycode::QK_LOCK => Some(LayoutKey {
            tap: Label::with_short("Lock", "Lock"),
            ..Default::default()
        }),
        Keycode::QK_ONE_SHOT_ON => Some(LayoutKey {
            tap: Label::with_short("OneShot On", "1ShotOn"),
            ..Default::default()
        }),
        Keycode::QK_ONE_SHOT_OFF => Some(LayoutKey {
            tap: Label::with_short("OneShot Off", "1ShotOf"),
            ..Default::default()
        }),
        Keycode::QK_ONE_SHOT_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("OneShot Toggle", "1ShotTg"),
            ..Default::default()
        }),
        Keycode::QK_KEY_OVERRIDE_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("KO Toggle", "KOTg"),
            ..Default::default()
        }),
        Keycode::QK_KEY_OVERRIDE_ON => Some(LayoutKey {
            tap: Label::with_short("KO On", "KOOn"),
            ..Default::default()
        }),
        Keycode::QK_KEY_OVERRIDE_OFF => Some(LayoutKey {
            tap: Label::with_short("KO Off", "KOOff"),
            ..Default::default()
        }),
        Keycode::QK_SECURE_LOCK => Some(LayoutKey {
            tap: Label::with_short("Secure Lock", "SecLock"),
            ..Default::default()
        }),
        Keycode::QK_SECURE_UNLOCK => Some(LayoutKey {
            tap: Label::with_short("Secure Unlock", "SecUnlk"),
            ..Default::default()
        }),
        Keycode::QK_SECURE_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Secure Toggle", "SecTg"),
            ..Default::default()
        }),
        Keycode::QK_SECURE_REQUEST => Some(LayoutKey {
            tap: Label::with_short("Secure Request", "SecReq"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_TAPPING_TERM_PRINT => Some(LayoutKey {
            tap: Label::with_short("DT Term", "DTTerm"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_TAPPING_TERM_UP => Some(LayoutKey {
            tap: Label::with_short("DT Term +", "DTTerm+"),
            ..Default::default()
        }),
        Keycode::QK_DYNAMIC_TAPPING_TERM_DOWN => Some(LayoutKey {
            tap: Label::with_short("DT Term -", "DTTerm-"),
            ..Default::default()
        }),
        Keycode::QK_CAPS_WORD_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Caps Word", "CW"),
            ..Default::default()
        }),
        Keycode::QK_AUTOCORRECT_ON => Some(LayoutKey {
            tap: Label::with_short("Autocorrect On", "ACOn"),
            ..Default::default()
        }),
        Keycode::QK_AUTOCORRECT_OFF => Some(LayoutKey {
            tap: Label::with_short("Autocorrect Off", "ACOff"),
            ..Default::default()
        }),
        Keycode::QK_AUTOCORRECT_TOGGLE => Some(LayoutKey {
            tap: Label::with_short("Autocorrect Tog", "ACTg"),
            ..Default::default()
        }),
        Keycode::QK_TRI_LAYER_LOWER => Some(LayoutKey {
            tap: Label::with_short("Tri Lower", "TriLow"),
            ..Default::default()
        }),
        Keycode::QK_TRI_LAYER_UPPER => Some(LayoutKey {
            tap: Label::with_short("Tri Upper", "TriUp"),
            ..Default::default()
        }),
        Keycode::QK_REPEAT_KEY => Some(LayoutKey {
            tap: Label::with_short("Repeat Key", "Rep"),
            ..Default::default()
        }),
        Keycode::QK_ALT_REPEAT_KEY => Some(LayoutKey {
            tap: Label::with_short("Alt Repeat", "ARep"),
            ..Default::default()
        }),
        _ => None,
    }
}
