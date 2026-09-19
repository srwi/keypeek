//! Port interface through which the overlay requests windowing changes from its platform host.

/// Interface required by the presentation layer from the underlying windowing host.
///
/// Implemented by platform-specific window hosts (e.g. `EframeHost`, `WaylandHost`,
/// or a web canvas host) to provide click-through/mouse-passthrough control and window close requests.
pub trait OverlayHost {
    fn set_passthrough(&mut self, enabled: bool);
    fn request_close(&mut self);
}
