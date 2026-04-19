#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod connection;
mod device_discovery;
mod key_matrix;
mod keyboard;
mod layout_key;
mod overlay_window;
mod protocols;
mod qmk_keycode_labels;
mod settings;
mod tray;
mod ui_wake;
mod zmk_keycode_labels;

use device_discovery::discover_devices;
use eframe::egui;
use overlay_window::OverlayApp;
use settings::Settings;
use ui_wake::UiWake;

fn main() -> Result<(), eframe::Error> {
    let settings = Settings::load().unwrap_or_default();
    let available_devices = discover_devices();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_taskbar(false)
            .with_maximized(true)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_always_on_top(),
        // Hide from macOS dock so the app only appears as a tray icon.
        #[cfg(target_os = "macos")]
        event_loop_builder: Some(Box::new(|builder| {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
            builder.with_activation_policy(ActivationPolicy::Accessory);
        })),
        ..Default::default()
    };

    eframe::run_native(
        "KeyPeek",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let tray_icon = tray::create_tray_icon();

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(OverlayApp::new(
                tray_icon,
                UiWake::from_ctx(&cc.egui_ctx),
                settings,
                available_devices,
            )))
        }),
    )
}
