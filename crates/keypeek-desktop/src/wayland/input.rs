use smithay_client_toolkit::seat::keyboard::{Keysym, Modifiers as SctkModifiers};

#[derive(Default)]
pub struct InputState {
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    pointer_pos: Option<egui::Pos2>,
}

impl InputState {
    pub fn set_modifiers(&mut self, m: SctkModifiers) {
        self.modifiers = egui::Modifiers {
            alt: m.alt,
            ctrl: m.ctrl,
            shift: m.shift,
            mac_cmd: false,
            command: m.ctrl,
        };
        self.events
            .push(egui::Event::ModifiersChanged(self.modifiers));
    }

    /// `pos` is in surface-local logical coordinates (egui points).
    pub fn pointer_moved(&mut self, pos: egui::Pos2) {
        self.pointer_pos = Some(pos);
        self.events.push(egui::Event::PointerMoved(pos));
    }

    pub fn pointer_left(&mut self) {
        self.pointer_pos = None;
        self.events.push(egui::Event::PointerGone);
    }

    pub fn pointer_button(&mut self, button: egui::PointerButton, pressed: bool) {
        if let Some(pos) = self.pointer_pos {
            self.events.push(egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: self.modifiers,
            });
        }
    }

    /// Scroll deltas in surface units; converted to egui's smooth scroll.
    pub fn scroll(&mut self, delta: egui::Vec2) {
        self.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            modifiers: self.modifiers,
        });
    }

    pub fn key(&mut self, keysym: Keysym, utf8: Option<&str>, pressed: bool) {
        if let Some(key) = keysym_to_egui_key(keysym) {
            self.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: self.modifiers,
            });
        }

        // Emit text only on press, and only for actual printable input (not control
        // characters or when ctrl/alt are held, which indicate shortcuts).
        if pressed && !self.modifiers.ctrl && !self.modifiers.alt {
            if let Some(text) = utf8 {
                if !text.is_empty() && !text.chars().any(|c| c.is_control()) {
                    self.events.push(egui::Event::Text(text.to_owned()));
                }
            }
        }
    }

    /// Drain accumulated events into a fresh `RawInput`.
    ///
    /// `size_pts` is the logical surface size, which is already in egui points.
    pub fn take_raw_input(&mut self, size_pts: (i32, i32)) -> egui::RawInput {
        let size_pts = egui::vec2(size_pts.0 as f32, size_pts.1 as f32);
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size_pts)),
            focused: true,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        }
    }
}

fn keysym_to_egui_key(keysym: Keysym) -> Option<egui::Key> {
    use egui::Key;
    Some(match keysym {
        Keysym::Return | Keysym::KP_Enter => Key::Enter,
        Keysym::Escape => Key::Escape,
        Keysym::Tab => Key::Tab,
        Keysym::BackSpace => Key::Backspace,
        Keysym::Delete => Key::Delete,
        Keysym::Insert => Key::Insert,
        Keysym::Home => Key::Home,
        Keysym::End => Key::End,
        Keysym::Page_Up => Key::PageUp,
        Keysym::Page_Down => Key::PageDown,
        Keysym::Left => Key::ArrowLeft,
        Keysym::Right => Key::ArrowRight,
        Keysym::Up => Key::ArrowUp,
        Keysym::Down => Key::ArrowDown,
        Keysym::space => Key::Space,
        _ => return None,
    })
}
