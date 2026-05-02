#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod cli;
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

use clap::Parser;
use cli::{Cli, parse_vid_pid};
use device_discovery::discover_devices;
use eframe::egui;
use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Stream};
use overlay_window::OverlayApp;
use protocols::{ConnectionSpec, ZmkTransportConfig};
use settings::Settings;
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use ui_wake::UiWake;

fn main() -> Result<(), eframe::Error> {
    let cli = Cli::parse();

    let socket_name = "keypeek.sock"
        .to_ns_name::<GenericNamespaced>()
        .expect("Failed to create socket name");

    if let Ok(mut stream) = Stream::connect(socket_name.clone()) {
        if cli.settings {
            let _ = stream.write_all(b"settin");
            let _ = stream.flush();
        } else if cli.toggle {
            let _ = stream.write_all(b"toggle");
            let _ = stream.flush();
        }
        return Ok(());
    }

    let listener = ListenerOptions::new()
        .name(socket_name)
        .create_sync()
        .expect("Failed to bind local socket");

    let manual_visible = Arc::new(AtomicBool::new(cli.toggle));
    let force_settings = Arc::new(AtomicBool::new(cli.settings));
    let ui_wake_cell = Arc::new(std::sync::Mutex::new(None::<UiWake>));

    {
        let manual_visible = Arc::clone(&manual_visible);
        let force_settings = Arc::clone(&force_settings);
        let ui_wake_cell = Arc::clone(&ui_wake_cell);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let mut buf = [0u8; 6];
                    if stream.read_exact(&mut buf).is_ok() {
                        let cmd = std::str::from_utf8(&buf).unwrap_or("");
                        if cmd == "toggle" {
                            let current = manual_visible.load(Ordering::Relaxed);
                            manual_visible.store(!current, Ordering::Relaxed);
                        } else if cmd == "settin" {
                            force_settings.store(true, Ordering::Relaxed);
                        }

                        if let Some(wake) = ui_wake_cell.lock().unwrap().as_ref() {
                            wake.request_repaint();
                        }
                    }
                }
            }
        });
    }

    let mut settings = Settings::load().unwrap_or_default();

    // Override settings with CLI flags
    if let Some(json_path) = cli.via {
        settings.last_connection = Some(ConnectionSpec::Via { json_path });
    } else if let Some(vial_str) = cli.vial {
        if let Ok((vid, pid)) = parse_vid_pid(&vial_str) {
            settings.last_connection = Some(ConnectionSpec::Vial { vid, pid });
        }
    } else if let Some(zmk_str) = cli.zmk {
        if let Ok((vid, pid)) = parse_vid_pid(&zmk_str) {
            let transport = if let Some(port) = cli.serial {
                ZmkTransportConfig::Serial(port)
            } else if let Some(id) = cli.ble {
                ZmkTransportConfig::Ble(id)
            } else {
                let serial_ports = protocols::zmk_rpc::scan_serial_ports();
                if let Some(port) = serial_ports.into_iter().find(|p| p.vid == vid && p.pid == pid) {
                    ZmkTransportConfig::Serial(port.port_name)
                } else {
                    ZmkTransportConfig::Serial("AUTO".to_string())
                }
            };
            settings.last_connection = Some(ConnectionSpec::Zmk {
                vid,
                pid,
                transport,
            });
        }
    }

    let available_devices = discover_devices();

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow, // Use glow renderer for transparent background (https://github.com/emilk/egui/issues/4451)
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

            let ui_wake = UiWake::from_ctx(&cc.egui_ctx);
            *ui_wake_cell.lock().unwrap() = Some(ui_wake.clone());

            Ok(Box::new(OverlayApp::new(
                tray_icon,
                ui_wake,
                settings,
                available_devices,
                manual_visible,
                force_settings,
            )))
        }),
    )
}
