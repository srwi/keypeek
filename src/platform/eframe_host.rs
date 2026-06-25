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
    /// macOS won't honor `with_maximized` for an undecorated transparent window, so
    /// we size it to the monitor on the first frame. Tracked here (a host concern)
    /// rather than in the shared UI state.
    #[cfg(target_os = "macos")]
    macos_maximized: bool,
}

impl eframe::App for EframeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color().to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        #[cfg(target_os = "macos")]
        if !self.macos_maximized {
            if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(0.0, 0.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(monitor_size));
                self.macos_maximized = true;
            }
        }

        let mut host = EframeHost { ctx: &ctx };
        self.app.ui(&ctx, &mut host);
    }
}

pub fn run(settings: Settings, devices: Vec<DiscoveredDevice>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow, // Glow is required for a transparent background (https://github.com/emilk/egui/issues/4451)
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_taskbar(false)
            .with_maximized(true)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_always_on_top(),
        // Hide from the macOS dock so the app only appears as a tray icon.
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
            let tray_icon = crate::tray::create_tray_icon();

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            let app = OverlayApp::new(tray_icon, UiWake::from_ctx(&cc.egui_ctx), settings, devices);
            Ok(Box::new(EframeApp {
                app,
                #[cfg(target_os = "macos")]
                macos_maximized: false,
            }))
        }),
    )
}
