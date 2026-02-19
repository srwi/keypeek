#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod device_discovery;
mod key_matrix;
mod keyboard;
mod layout_key;
mod overlay_window;
mod protocols;
mod qmk_keycode_labels;
mod settings;
mod tray;
mod zmk_keycode_labels;

use eframe::egui::{self, IconData};
use overlay_window::OverlayApp;
use settings::Settings;

const SETTINGS_FILE: &str = "settings.ini";

fn run_overlay_app(initial_settings: Option<Settings>) -> Result<(), eframe::Error> {
    let _tray_icon = tray::create_tray_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_taskbar(false)
            .with_maximized(true)
            .with_transparent(true)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "QMK Layout Helper",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(OverlayApp::new(initial_settings)))
        }),
    )
}

fn main() -> Result<(), eframe::Error> {
    let _icon = {
        let image = image::load_from_memory(include_bytes!("../resources/icon.ico"))
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        IconData {
            width,
            height,
            rgba: image.into_raw(),
        }
    };

    run_overlay_app(Settings::load_from_file(SETTINGS_FILE))
}
