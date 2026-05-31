use crate::layout_key::LayoutKey;
use std::time::{Duration, Instant};

const MIN_HIGHLIGHT_DURATION: Duration = Duration::from_millis(50);

pub struct KeyMatrix {
    pub keys: Vec<Vec<Vec<Option<LayoutKey>>>>,
    pub pressed: Vec<Vec<bool>>,
    pub last_pressed_at: Vec<Vec<Option<Instant>>>,
}

impl KeyMatrix {
    pub fn from_layout_keys(
        keys: Vec<Vec<Vec<Option<LayoutKey>>>>,
        rows: usize,
        cols: usize,
    ) -> Self {
        KeyMatrix {
            keys,
            pressed: vec![vec![false; cols]; rows],
            last_pressed_at: vec![vec![None; cols]; rows],
        }
    }

    pub fn get_num_layers(&self) -> usize {
        self.keys.len()
    }

    pub fn get_key(&self, layer: usize, row: usize, col: usize) -> Option<&LayoutKey> {
        self.keys
            .get(layer)
            .and_then(|l| l.get(row))
            .and_then(|r| r.get(col))
            .and_then(|k| k.as_ref())
    }

    pub fn is_transparent(&self, layer: usize, row: usize, col: usize) -> bool {
        self.keys
            .get(layer)
            .and_then(|l| l.get(row))
            .and_then(|r| r.get(col))
            .map(|k| k.is_none())
            .unwrap_or(true)
    }

    /// Returns whether the key should currently be shown as pressed.
    ///
    /// This includes keys that are physically pressed as well as recently released keys
    /// that still fall within the minimum highlight duration window.
    pub fn is_pressed(&self, row: usize, col: usize) -> bool {
        let physically_pressed = self.pressed
            .get(row)
            .and_then(|r| r.get(col))
            .copied()
            .unwrap_or(false);

        if physically_pressed {
            return true;
        }

        // Check for minimum highlight duration to ensure fast taps are visible
        if let Some(Some(last_press)) = self.last_pressed_at.get(row).and_then(|r| r.get(col)) {
            if last_press.elapsed() < MIN_HIGHLIGHT_DURATION {
                return true;
            }
        }

        false
    }

    pub fn set_pressed(&mut self, row: usize, col: usize, value: bool) {
        if let Some(r) = self.pressed.get_mut(row) {
            if col < r.len() {
                r[col] = value;
                if value {
                    if let Some(times) = self.last_pressed_at.get_mut(row) {
                        if col < times.len() {
                            times[col] = Some(Instant::now());
                        }
                    }
                }
            }
        }
    }

    pub fn get_min_highlight_timeout(&self) -> Option<Duration> {
        let mut min_timeout: Option<Duration> = None;
        let now = Instant::now();
        let duration_50ms = MIN_HIGHLIGHT_DURATION;

        for (r, row_pressed) in self.pressed.iter().enumerate() {
            for (c, &physically_pressed) in row_pressed.iter().enumerate() {
                if !physically_pressed {
                    if let Some(Some(last_press)) =
                        self.last_pressed_at.get(r).and_then(|row| row.get(c))
                    {
                        if let Some(elapsed) = now.checked_duration_since(*last_press) {
                            if elapsed < duration_50ms {
                                let remaining = duration_50ms - elapsed;
                                min_timeout =
                                    Some(min_timeout.map_or(remaining, |min| min.min(remaining)));
                            }
                        }
                    }
                }
            }
        }
        min_timeout
    }
}
