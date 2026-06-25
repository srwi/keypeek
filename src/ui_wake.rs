use std::sync::Arc;

/// A host-agnostic way for worker threads to ask the UI to repaint.
///
/// On the eframe host this wraps `egui::Context::request_repaint`; on the Wayland
/// host it pings the calloop event loop so the next frame is drawn.
#[derive(Clone)]
pub struct UiWake(Arc<dyn Fn() + Send + Sync>);

impl UiWake {
    /// Build a waker from an arbitrary callback (used by the Wayland host).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn new(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self(wake)
    }

    pub fn from_ctx(ctx: &egui::Context) -> Self {
        let ctx = ctx.clone();
        Self(Arc::new(move || ctx.request_repaint()))
    }

    pub fn request_repaint(&self) {
        (self.0)();
    }
}
