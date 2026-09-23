use std::time::Duration;
use web_time::Instant;

/// Active layers seen through the visible-layer bitmask.
/// Bit `i` selects layer `i` (see `Settings::visible_layers`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveLayers {
    /// A selected layer above the base layer is held.
    Selected,
    /// An active layer is hidden by the mask.
    Excluded,
    /// No selected layer is held and nothing is masked out.
    /// The timeout controls how long the overlay stays visible.
    Base,
}

impl ActiveLayers {
    pub fn classify(layer_state: u32, default_layer_state: u32, visible_layers: u32) -> Self {
        // The base layer is always active below momentary and default layers.
        let active = layer_state | default_layer_state | 1;

        // Holding the base layer does not keep the overlay visible.
        // The timeout controls base layer visibility instead.
        let held_visible = layer_state & visible_layers & !1 != 0;
        let any_hidden = active & !visible_layers != 0;
        match (held_visible, any_hidden) {
            (true, _) => Self::Selected,
            (_, true) => Self::Excluded,
            _ => Self::Base,
        }
    }
}

/// Overlay timing and layer filter settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayConfig {
    /// Time in ms the overlay stays visible after layer release. Negative values keep it visible.
    pub timeout_ms: i64,
    /// Time in ms a layer must be held before the overlay appears.
    pub activation_delay_ms: u32,
    /// Bit `i` keeps the overlay visible while layer `i` is active.
    pub visible_layers: u32,
}

/// Time interval during which the overlay is visible.
/// `until = None` keeps the overlay visible until the layer state changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibilityWindow {
    pub from: Instant,
    pub until: Option<Instant>,
}

impl VisibilityWindow {
    /// Hidden window until the next layer state update.
    pub fn hidden(now: Instant) -> Self {
        Self {
            from: now,
            until: Some(now),
        }
    }

    pub fn is_visible(&self, now: Instant) -> bool {
        now >= self.from && self.until.is_none_or(|until| now < until)
    }

    /// Time until the overlay visibility state changes.
    pub fn changes_in(&self, now: Instant) -> Option<Duration> {
        let next = if now < self.from {
            Some(self.from)
        } else {
            self.until
        };
        next.filter(|at| now < *at).map(|at| at - now)
    }
}

/// Computes the new visibility window when layer state changes.
pub fn next_visibility_window(
    active: ActiveLayers,
    previous: ActiveLayers,
    current: VisibilityWindow,
    now: Instant,
    config: OverlayConfig,
) -> VisibilityWindow {
    // True if a layer is held but its activation delay has not elapsed.
    let pending = now < current.from;

    match active {
        ActiveLayers::Selected => VisibilityWindow {
            // Keep existing start time if already arming or visible.
            from: if pending || current.is_visible(now) {
                current.from
            } else {
                now + Duration::from_millis(config.activation_delay_ms as u64)
            },
            until: None,
        },
        ActiveLayers::Excluded => VisibilityWindow::hidden(now),
        // Do not show the base layer when leaving an excluded layer
        // or releasing before the activation delay elapses.
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
