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
mod zmk_keycode_labels;

use connection::build_connected_state;
use device_discovery::discover_devices;
use eframe::egui::{self, IconData};
use overlay_window::OverlayApp;
use settings::Settings;

const SETTINGS_FILE: &str = "settings.ini";

fn bootstrap_startup(
    initial_settings: Option<Settings>,
) -> (
    Settings,
    Vec<device_discovery::DiscoveredDevice>,
    Option<connection::ConnectedState>,
    Option<String>,
    bool,
) {
    let mut base_settings = initial_settings.clone().unwrap_or_default();
    let mut available_devices = Vec::new();
    let mut initial_connected = None;
    let mut initial_error = None;
    let mut settings_visible = true;

    if let Some(saved) = initial_settings.filter(|settings| settings.save_settings) {
        match build_connected_state(saved) {
            Ok(connected) => {
                base_settings = connected.settings.clone();
                initial_connected = Some(connected);
                settings_visible = false;
                return (
                    base_settings,
                    available_devices,
                    initial_connected,
                    initial_error,
                    settings_visible,
                );
            }
            Err(e) => {
                initial_error = Some(format!("Failed to connect using saved settings: {e}"));
            }
        }
    }

    available_devices = discover_devices();
    (
        base_settings,
        available_devices,
        initial_connected,
        initial_error,
        settings_visible,
    )
}

fn run_overlay_app(initial_settings: Option<Settings>) -> Result<(), eframe::Error> {
    let _tray_icon = tray::create_tray_icon();
    let (base_settings, available_devices, initial_connected, initial_error, settings_visible) =
        bootstrap_startup(initial_settings);

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

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(OverlayApp::new(
                base_settings,
                available_devices,
                initial_connected,
                initial_error,
                settings_visible,
            )))
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
