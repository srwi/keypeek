//! Interface to request window changes from the host platform.

/// Interface provided by the window host to control passthrough and window closing.
pub trait OverlayHost {
    fn set_passthrough(&mut self, enabled: bool);
    fn request_close(&mut self);
}
