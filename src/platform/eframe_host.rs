use super::OverlayHost;
use crate::device_discovery::DiscoveredDevice;
use crate::overlay_window::OverlayApp;
use crate::settings::Settings;
use crate::ui_wake::UiWake;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct EframeHost<'a> {
    ctx: &'a egui::Context,
}

impl OverlayHost for EframeHost<'_> {
    fn set_passthrough(&mut self, enabled: bool) {
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::MousePassthrough(enabled));
    }

    fn request_close(&mut self) {
        self.ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

struct EframeApp {
    app: OverlayApp,
    // Undecorated transparent windows don't reliably honor `with_maximized`, so we
    // size to the monitor explicitly once known. Linux never WM-maximizes at all,
    // since Mutter drops always-on-top on a maximized window.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    sized_to_monitor: bool,
    // winit's always-on-top request is sent before the window is mapped, which
    // EWMH WMs like Mutter ignore, so re-assert it for a few frames after mapping.
    #[cfg(target_os = "linux")]
    x11_above_ticks: u32,
}

impl eframe::App for EframeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color().to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Re-assert always-on-top now that the window is mapped (see field docs).
        #[cfg(target_os = "linux")]
        if self.x11_above_ticks > 0 {
            self.x11_above_ticks -= 1;
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
            ctx.request_repaint();
        }

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        if !self.sized_to_monitor {
            if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(0.0, 0.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(monitor_size));
                self.sized_to_monitor = true;
            }
        }

        let mut host = EframeHost { ctx: &ctx };
        self.app.ui(&ctx, &mut host);
    }
}

// Keep the overlay visible when switching Spaces or using fullscreen apps.
#[cfg(target_os = "macos")]
fn show_on_all_spaces(cc: &eframe::CreationContext<'_>) {
    use objc2_app_kit::{NSView, NSWindowCollectionBehavior};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = cc.window_handle() else {
        return;
    };
    if let RawWindowHandle::AppKit(handle) = handle.as_raw() {
        let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
        if let Some(window) = view.window() {
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
        }
    }
}

// `force_x11` (Linux only) makes winit use XWayland instead of native Wayland,
// since Mutter honors always-on-top for XWayland clients but not native ones.
pub fn run(
    settings: Settings,
    devices: Vec<DiscoveredDevice>,
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] force_x11: bool,
) -> Result<(), eframe::Error> {
    #[cfg(target_os = "linux")]
    if force_x11 {
        match run_inner(settings.clone(), devices.clone(), true) {
            Ok(()) => return Ok(()),
            Err(e) => {
                eprintln!(
                    "KeyPeek: XWayland/X11 backend unavailable ({e}); \
                     retrying on Wayland (overlay will not stay always-on-top)."
                );
            }
        }
    }
    run_inner(settings, devices, false)
}

fn run_inner(
    settings: Settings,
    devices: Vec<DiscoveredDevice>,
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] force_x11: bool,
) -> Result<(), eframe::Error> {
    #[allow(unused_mut)]
    let mut viewport = egui::ViewportBuilder::default()
        .with_decorations(false)
        .with_taskbar(false)
        .with_maximized(true)
        .with_transparent(true)
        .with_has_shadow(false)
        .with_always_on_top();

    #[cfg(target_os = "linux")]
    {
        viewport = viewport.with_window_type(egui::X11WindowType::Utility);
    }

    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow, // Glow is required for a transparent background (https://github.com/emilk/egui/issues/4451)
        viewport,
        ..Default::default()
    };

    // Hide from the macOS dock so the app only appears as a tray icon.
    #[cfg(target_os = "macos")]
    {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
            builder.with_activation_policy(ActivationPolicy::Accessory);
        }));
    }

    // Force XWayland so always-on-top is honored on GNOME (see `run`).
    #[cfg(target_os = "linux")]
    if force_x11 {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            builder.with_x11();
        }));
    }

    eframe::run_native(
        "KeyPeek",
        options,
        Box::new(move |cc| {
            #[cfg(target_os = "macos")]
            show_on_all_spaces(cc);

            egui_extras::install_image_loaders(&cc.egui_ctx);

            let ui_wake = UiWake::from_ctx(&cc.egui_ctx);
            let settings_requested = Arc::new(AtomicBool::new(false));
            let tray_icon = crate::tray::create_tray_icon({
                let settings_requested = settings_requested.clone();
                let ui_wake = ui_wake.clone();
                Arc::new(move || {
                    settings_requested.store(true, Ordering::Relaxed);
                    ui_wake.request_repaint();
                })
            });

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            let app = OverlayApp::new(tray_icon, settings_requested, ui_wake, settings, devices);
            Ok(Box::new(EframeApp {
                app,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                sized_to_monitor: false,
                #[cfg(target_os = "linux")]
                x11_above_ticks: 10,
            }))
        }),
    )
}
