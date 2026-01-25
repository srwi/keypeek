#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod key_matrix;
mod keyboard;
mod keycode_labels;
mod overlay_window;
mod protocols;
mod settings;
mod settings_window;
mod tray;

use eframe::egui::{self, IconData};
use keyboard::Keyboard;
use overlay_window::Overlay;
use protocols::via::ViaProtocol;
use protocols::vial::VialProtocol;
use protocols::KeyboardProtocol;
use settings::{ProtocolType, Settings};
use settings_window::SettingsApp;
use std::sync::{Arc, Mutex};

const SETTINGS_FILE: &str = "settings.ini";

fn run_overlay_app(keyboard: Keyboard, settings: &Settings) -> Result<(), eframe::Error> {
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
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(Overlay::new(
                keyboard,
                settings.size,
                settings.margin,
                settings.position,
            )))
        }),
    )
}

fn show_settings_window() -> Option<Settings> {
    let shared = Arc::new(Mutex::new(Settings::default()));
    let icon = {
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
    let options = eframe::NativeOptions {
        run_and_return: true,
        viewport: egui::ViewportBuilder::default()
            .with_decorations(true)
            .with_inner_size([500.0, 420.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native("QMK Layout Helper", options, {
        let shared_settings = shared.clone();
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(SettingsApp::new(shared_settings)))
        })
    })
    .ok()?;
    let settings = shared.lock().unwrap().clone();
    if !settings.confirmed {
        return None;
    }
    if settings.save_settings {
        if let Err(e) = settings.save_to_file(SETTINGS_FILE) {
            eprintln!("Failed to save settings: {e}");
        }
    }
    Some(settings)
}

fn try_to_launch_overlay(settings: &Settings) -> bool {
    let protocol: Box<dyn KeyboardProtocol> = match settings.protocol_type {
        ProtocolType::Vial => {
            let parts: Vec<&str> = settings.device_identifier.split(':').collect();
            if parts.len() != 2 {
                eprintln!("Invalid VIAL device ID format: {}", settings.device_identifier);
                return false;
            }
            let vid = match u16::from_str_radix(parts[0], 16) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let pid = match u16::from_str_radix(parts[1], 16) {
                Ok(p) => p,
                Err(_) => return false,
            };
            match VialProtocol::connect(vid, pid) {
                Ok(p) => Box::new(p),
                Err(e) => {
                    eprintln!("Failed to connect to VIAL device: {e}");
                    return false;
                }
            }
        }
        ProtocolType::Via => {
            match ViaProtocol::connect(&settings.device_identifier) {
                Ok(p) => Box::new(p),
                Err(e) => {
                    eprintln!("Failed to connect to VIA device: {e}");
                    return false;
                }
            }
        }
    };

    let keyboard = match Keyboard::new(
        protocol,
        settings.layout_name.clone(),
        settings.timeout,
    ) {
        Ok(kb) => kb,
        Err(e) => {
            eprintln!("Failed to create keyboard: {e}");
            return false;
        }
    };

    let _ = run_overlay_app(keyboard, settings);

    true
}

fn main() -> Result<(), eframe::Error> {
    if let Some(settings) = Settings::load_from_file(SETTINGS_FILE) {
        if try_to_launch_overlay(&settings) {
            return Ok(());
        }
    }

    if let Some(settings) = show_settings_window() {
        let _ = try_to_launch_overlay(&settings);
    }

    Ok(())
}
