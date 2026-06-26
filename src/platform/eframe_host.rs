//! The eframe/winit host: Windows, macOS, Linux/X11, Linux/XWayland.
//!
//! This is the original windowing path. It wraps the egui UI in `eframe::run_native`
//! and adapts the host requests ([`OverlayHost`]) onto egui `ViewportCommand`s.

use super::OverlayHost;
use crate::device_discovery::DiscoveredDevice;
use crate::overlay_window::OverlayApp;
use crate::settings::Settings;
use crate::ui_wake::UiWake;

/// Issues host requests as egui viewport commands on the active context.
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

/// Thin `eframe::App` adapter around [`OverlayApp`].
struct EframeApp {
    app: OverlayApp,
    /// macOS and Linux/X11 won't reliably honor `with_maximized` for an undecorated
    /// transparent window, so we size it to the monitor explicitly once the monitor
    /// size is known. On Linux we deliberately avoid WM maximize entirely: under
    /// Mutter a maximized window loses always-on-top. Tracked here (a host concern)
    /// rather than in the shared UI state.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    sized_to_monitor: bool,
    /// On X11, winit only requests `_NET_WM_STATE_ABOVE` via a `_NET_WM_STATE`
    /// ClientMessage sent during window *creation* — before the window is mapped —
    /// which EWMH-compliant WMs (Mutter) ignore, so `with_always_on_top()` is lost.
    /// We re-assert the level via `ViewportCommand::WindowLevel` for the first few
    /// frames, once the window is mapped, so the message actually takes effect.
    #[cfg(target_os = "linux")]
    x11_above_ticks: u32,
}

impl eframe::App for EframeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color().to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Re-assert always-on-top now that the window is mapped, since winit's
        // pre-map request was dropped by the WM (see field docs). We never WM-maximize
        // on Linux — under Mutter that drops always-on-top — so force unmaximized too.
        #[cfg(target_os = "linux")]
        if self.x11_above_ticks > 0 {
            self.x11_above_ticks -= 1;
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
            ctx.request_repaint();
        }

        // Fill the monitor by sizing explicitly (undecorated windows don't maximize
        // reliably; on Linux WM-maximize also breaks always-on-top — see field docs).
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

/// Run the eframe host.
///
/// `force_x11` (Linux only) makes winit use the XWayland/X11 backend instead of
/// native Wayland. We do this in the GNOME fallback: GNOME/Mutter has no
/// wlr-layer-shell, and native Wayland silently ignores `with_always_on_top()`,
/// so the overlay drops behind the foreground window once it goes click-through.
/// On X11, `always_on_top` becomes `_NET_WM_STATE_ABOVE`, which Mutter honors for
/// XWayland clients independent of focus — keeping the overlay on top. If the X11
/// backend can't start (no XWayland), we retry on Wayland (without always-on-top).
pub fn run(
    settings: Settings,
    devices: Vec<DiscoveredDevice>,
    force_x11: bool,
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
    let mut options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow, // Glow is required for a transparent background (https://github.com/emilk/egui/issues/4451)
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_taskbar(false)
            .with_maximized(true)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_always_on_top(),
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
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let tray_icon = crate::tray::create_tray_icon();

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            let app = OverlayApp::new(tray_icon, UiWake::from_ctx(&cc.egui_ctx), settings, devices);
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
