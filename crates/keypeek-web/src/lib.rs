#![cfg(target_arch = "wasm32")]

use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use keypeek_presentation::{add_phosphor_to_fonts, MemorySettingsStore, OverlayHost};
use keypeek_protocol::ui_wake::UiWake;

pub mod overlay_app;
pub use overlay_app::{clear_color, OverlayApp, WebOverlayApp};

struct WebHost;

impl OverlayHost for WebHost {
    fn set_passthrough(&mut self, _enabled: bool) {
        // Window mouse passthrough is not applicable to web canvas.
    }

    fn request_close(&mut self) {
        // Window close is not applicable to web canvas.
    }
}

pub struct WebApp {
    app: WebOverlayApp,
}

impl WebApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut fonts = egui::FontDefinitions::default();
        add_phosphor_to_fonts(&mut fonts);
        cc.egui_ctx.set_fonts(fonts);

        let ctx = cc.egui_ctx.clone();
        let ui_wake = UiWake::new(Arc::new(move || ctx.request_repaint()));
        let settings_store = Arc::new(MemorySettingsStore::default());
        let app = WebOverlayApp::new(ui_wake, settings_store);

        Self { app }
    }
}

impl eframe::App for WebApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color().to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut host = WebHost;
        self.app.ui(ui, &mut host);
    }
}

/// Automatically starts KeyPeek on the canvas with id `keypeek_canvas`.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    wasm_bindgen_futures::spawn_local(async {
        let document = match web_sys::window().and_then(|w| w.document()) {
            Some(doc) => doc,
            None => {
                log::error!("KeyPeek: No window or document found");
                return;
            }
        };

        let canvas = match document.get_element_by_id("keypeek_canvas") {
            Some(el) => match el.dyn_into::<web_sys::HtmlCanvasElement>() {
                Ok(c) => c,
                Err(_) => {
                    log::error!("KeyPeek: #keypeek_canvas is not a HtmlCanvasElement");
                    return;
                }
            },
            None => {
                log::error!("KeyPeek: Failed to find #keypeek_canvas");
                return;
            }
        };

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(WebApp::new(cc)))),
            )
            .await;

        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(()) => {
                    loading_text.remove();
                }
                Err(err) => {
                    loading_text.set_inner_html(
                        "<p>The app has crashed. See developer console for details.</p>",
                    );
                    log::error!("KeyPeek: Failed to start eframe: {err:?}");
                }
            }
        }
    });
}
