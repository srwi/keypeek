use std::time::Duration;
use web_time::Instant;

/// The active layers as seen through the visible-layer bitmask (bit `i` selects layer
/// `i`; see `Settings::visible_layers`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveLayers {
    /// A selected layer above the base layer is held.
    Selected,
    /// An active layer is masked out.
    Excluded,
    /// Nothing is masked out and no selected layer is held, so the timeout decides how
    /// long the overlay lingers.
    Base,
}

impl ActiveLayers {
    pub fn classify(layer_state: u32, default_layer_state: u32, visible_layers: u32) -> Self {
        // The base layer is always active underneath the momentary and default layers.
        let active = layer_state | default_layer_state | 1;

        // Holding the base layer is not a reason to keep the overlay up; the timeout
        // governs that instead, so it never counts as a selected layer.
        let held_visible = layer_state & visible_layers & !1 != 0;
        let any_hidden = active & !visible_layers != 0;
        match (held_visible, any_hidden) {
            (true, _) => Self::Selected,
            (_, true) => Self::Excluded,
            _ => Self::Base,
        }
    }
}

/// The overlay's tuning knobs, all changeable while connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayConfig {
    /// How long the overlay lingers once no selected layer is held; negative never hides.
    pub timeout_ms: i64,
    /// How long a layer has to be held before the overlay appears.
    pub activation_delay_ms: u32,
    /// Bit `i` keeps the overlay up while layer `i` is active; see `ActiveLayers`.
    pub visible_layers: u32,
}

/// The stretch of time the overlay is shown for: from `from` until `until`, where `None`
/// keeps it up until the layer state changes again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibilityWindow {
    pub from: Instant,
    pub until: Option<Instant>,
}

impl VisibilityWindow {
    /// An empty window, keeping the overlay hidden until the next layer state arrives.
    pub fn hidden(now: Instant) -> Self {
        Self {
            from: now,
            until: Some(now),
        }
    }

    pub fn is_visible(&self, now: Instant) -> bool {
        now >= self.from && self.until.is_none_or(|until| now < until)
    }

    /// How long until the overlay appears or disappears on its own.
    pub fn changes_in(&self, now: Instant) -> Option<Duration> {
        let next = if now < self.from {
            Some(self.from)
        } else {
            self.until
        };
        next.filter(|at| now < *at).map(|at| at - now)
    }
}

/// The window a freshly arrived layer state puts the overlay in.
pub fn next_visibility_window(
    active: ActiveLayers,
    previous: ActiveLayers,
    current: VisibilityWindow,
    now: Instant,
    config: OverlayConfig,
) -> VisibilityWindow {
    // A held layer whose activation delay has not elapsed yet.
    let pending = now < current.from;

    match active {
        ActiveLayers::Selected => VisibilityWindow {
            // A window still arming or already up keeps its start: layers added mid-hold
            // must not restart the countdown, nor blink a visible overlay away.
            from: if pending || current.is_visible(now) {
                current.from
            } else {
                now + Duration::from_millis(config.activation_delay_ms as u64)
            },
            until: None,
        },
        ActiveLayers::Excluded => VisibilityWindow::hidden(now),
        // Neither leaving an excluded layer nor releasing a layer before its activation
        // delay elapsed may surface the base layer.
        ActiveLayers::Base if previous == ActiveLayers::Excluded || pending => {
            VisibilityWindow::hidden(now)
        }
        ActiveLayers::Base => VisibilityWindow {
            from: now,
            until: (config.timeout_ms >= 0)
                .then(|| now + Duration::from_millis(config.timeout_ms as u64)),
        },
    }
}

/// Pure state machine tracking overlay visibility across layer updates and timer expirations.
#[derive(Clone, Debug)]
pub struct VisibilityStateMachine {
    config: OverlayConfig,
    window: VisibilityWindow,
    previous_layers: ActiveLayers,
}

impl VisibilityStateMachine {
    pub fn new(config: OverlayConfig, now: Instant) -> Self {
        Self {
            config,
            window: VisibilityWindow::hidden(now),
            previous_layers: ActiveLayers::Base,
        }
    }

    pub fn set_config(&mut self, config: OverlayConfig) {
        self.config = config;
    }

    pub fn is_visible(&self, now: Instant) -> bool {
        self.window.is_visible(now)
    }

    pub fn changes_in(&self, now: Instant) -> Option<Duration> {
        self.window.changes_in(now)
    }

    /// Updates visibility when layer bitmasks change. Returns `true` if a repaint is needed.
    pub fn on_layers_changed(
        &mut self,
        active_layers: u32,
        default_layers: u32,
        now: Instant,
    ) -> bool {
        let active =
            ActiveLayers::classify(active_layers, default_layers, self.config.visible_layers);
        self.window =
            next_visibility_window(active, self.previous_layers, self.window, now, self.config);
        self.previous_layers = active;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::ActiveLayers::{Base, Excluded, Selected};
    use super::*;

    const CONFIG: OverlayConfig = OverlayConfig {
        timeout_ms: 2000,
        activation_delay_ms: 300,
        visible_layers: u32::MAX,
    };

    /// Walks the layer-state transitions the activation delay has to survive.
    #[test]
    fn activation_delay_gates_the_overlay() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let hidden = VisibilityWindow::hidden(start);

        // Holding a layer arms the delay; the overlay only shows once it has elapsed.
        let held = next_visibility_window(Selected, Base, hidden, start, CONFIG);
        assert!(!held.is_visible(at(299)));
        assert!(held.is_visible(at(300)));
        assert_eq!(held.changes_in(start), Some(Duration::from_millis(300)));

        // A second layer added mid-hold keeps the original countdown.
        let more = next_visibility_window(Selected, Selected, held, at(200), CONFIG);
        assert!(more.is_visible(at(300)));

        // Releasing before the delay elapsed shows nothing at all.
        let tapped = next_visibility_window(Base, Selected, held, at(100), CONFIG);
        assert!(!tapped.is_visible(at(100)));

        // Releasing after it elapsed lingers for the display duration.
        let released = next_visibility_window(Base, Selected, held, at(400), CONFIG);
        assert!(released.is_visible(at(2399)));
        assert!(!released.is_visible(at(2400)));

        // A layer held while the overlay is still up must not blink it away.
        let again = next_visibility_window(Selected, Base, released, at(500), CONFIG);
        assert!(again.is_visible(at(500)));
    }

    /// Without a delay the overlay behaves as it does with the feature turned off.
    #[test]
    fn zero_delay_shows_the_overlay_right_away() {
        let start = Instant::now();
        let hidden = VisibilityWindow::hidden(start);
        let no_delay = OverlayConfig {
            activation_delay_ms: 0,
            ..CONFIG
        };

        let held = next_visibility_window(Selected, Base, hidden, start, no_delay);
        assert!(held.is_visible(start));
        assert_eq!(held.changes_in(start), None);

        // Leaving an excluded layer still must not surface the base layer.
        let excluded = next_visibility_window(Excluded, Selected, held, start, no_delay);
        assert!(!excluded.is_visible(start));
        let base = next_visibility_window(Base, Excluded, excluded, start, no_delay);
        assert!(!base.is_visible(start));
    }
}
