use std::sync::Arc;

/// Thread-safe callback handle used by background workers to request an event loop wakeup.
#[derive(Clone)]
pub struct UiWake(Arc<dyn Fn() + Send + Sync>);

impl UiWake {
    pub fn new(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self(wake)
    }

    pub fn request_repaint(&self) {
        (self.0)();
    }
}
