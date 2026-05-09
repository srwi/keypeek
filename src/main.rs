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
use cli::{parse_vid_pid, Cli};
use device_discovery::discover_devices;
use eframe::egui;
use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Stream};
use overlay_window::OverlayApp;
use protocols::{ConnectionSpec, ZmkTransportConfig};
use settings::Settings;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ui_wake::UiWake;

const SOCKET_BASENAME: &str = "keypeek.sock";

fn send_command_to_running_instance(cli: &Cli, mut stream: Stream) -> Result<(), eframe::Error> {
    if cli.settings {
        stream
            .write_all(b"settings")
            .map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
        stream
            .flush()
            .map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    } else if cli.toggle {
        stream
            .write_all(b"toggle")
            .map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
        stream
            .flush()
            .map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    } else {
        eprintln!(
            "App is already running but no command was provided; use --settings or --toggle."
        );
        std::process::exit(1);
    }

    Ok(())
}

fn main() -> Result<(), eframe::Error> {
    let cli = Cli::parse();

    let socket_name = SOCKET_BASENAME
        .to_ns_name::<GenericNamespaced>()
        .expect("Failed to create socket name");

    if let Ok(stream) = Stream::connect(socket_name.clone()) {
        send_command_to_running_instance(&cli, stream)?;
        return Ok(());
    }

    let listener = match ListenerOptions::new()
        .name(socket_name.clone())
        .create_sync()
    {
        Ok(listener) => listener,
        Err(err) if err.kind() == ErrorKind::AddrInUse => {
            if let Ok(stream) = Stream::connect(socket_name.clone()) {
                send_command_to_running_instance(&cli, stream)?;
                return Ok(());
            }

            #[cfg(unix)]
            {
                let stale_socket_path = std::path::Path::new("/tmp").join(SOCKET_BASENAME);
                if stale_socket_path.exists() {
                    std::fs::remove_file(&stale_socket_path).unwrap_or_else(|remove_err| {
                        panic!(
                            "Failed to bind local socket and could not remove stale socket '{}': {}",
                            stale_socket_path.display(),
                            remove_err
                        )
                    });

                    ListenerOptions::new()
                        .name(socket_name.clone())
                        .create_sync()
                        .unwrap_or_else(|retry_err| {
                            panic!(
                                "Failed to bind local socket after stale-socket cleanup: {}",
                                retry_err
                            )
                        })
                } else {
                    panic!("Failed to bind local socket: {}", err);
                }
            }

            #[cfg(not(unix))]
            {
                panic!("Failed to bind local socket: {}", err);
            }
        }
        Err(err) => panic!("Failed to bind local socket: {}", err),
    };

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
                    // We only expect a small command, but we should read what's there
                    // local_socket streams usually close or signal end of data when flushed/dropped
                    let mut temp_buf = [0u8; 32];
                    if let Ok(n) = stream.read(&mut temp_buf) {
                        let cmd = std::str::from_utf8(&temp_buf[..n]).unwrap_or("");
                        if cmd.starts_with("toggle") {
                            manual_visible.fetch_xor(true, Ordering::Relaxed);
                        } else if cmd.starts_with("settings") {
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
        match parse_vid_pid(&vial_str) {
            Ok((vid, pid)) => {
                settings.last_connection = Some(ConnectionSpec::Vial { vid, pid });
            }
            Err(err) => {
                eprintln!(
                    "Invalid Vial VID:PID '{}': {}. Expected VID:PID like 1234:5678",
                    vial_str, err
                );
                std::process::exit(1);
            }
        }
    } else if let Some(zmk_str) = cli.zmk {
        let (vid, pid) = match parse_vid_pid(&zmk_str) {
            Ok((vid, pid)) => (vid, pid),
            Err(err) => {
                eprintln!(
                    "Invalid ZMK VID:PID '{}': {}. Expected VID:PID like 1234:5678",
                    zmk_str, err
                );
                std::process::exit(1);
            }
        };

        let transport = if let Some(port) = cli.serial {
            ZmkTransportConfig::Serial(port)
        } else if let Some(id) = cli.ble {
            ZmkTransportConfig::Ble(id)
        } else {
            let serial_ports = protocols::zmk_rpc::scan_serial_ports();
            if let Some(port) = serial_ports
                .into_iter()
                .find(|p| p.vid == vid && p.pid == pid)
            {
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
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
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
