use std::sync::Arc;

#[derive(Clone)]
pub struct UiWake(Arc<dyn Fn() + Send + Sync>);

impl UiWake {
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
