use super::OverlayHost;
use crate::device_discovery::DiscoveredDevice;
use crate::overlay_window::OverlayApp;
use crate::settings::{MonitorSelection, Settings};
use crate::ui_wake::UiWake;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct EframeHost<'a> {
    ctx: &'a egui::Context,
}

// Windows reports monitor names as the raw GDI device name (`\\.\DISPLAY1`).
fn clean_monitor_name(name: &str) -> &str {
    name.strip_prefix(r"\\.\").unwrap_or(name)
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
    last_applied_monitor: Option<MonitorSelection>,
    // winit's always-on-top request is sent before the window is mapped, which
    // EWMH WMs like Mutter ignore, so re-assert it for a few frames after mapping.
    #[cfg(target_os = "linux")]
    x11_above_ticks: u32,
}

impl EframeApp {
    fn apply_monitor_placement(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let target = self.app.active_monitor().clone();
        if self.last_applied_monitor.as_ref() == Some(&target) {
            return;
        }

        // Not available on the very first frame — retried next frame.
        let Some(window) = frame.winit_window() else {
            return;
        };

        let monitor = match &target {
            MonitorSelection::Primary => window.primary_monitor(),
            MonitorSelection::Named(name) => window
                .available_monitors()
                .find(|m| m.name().as_deref().map(clean_monitor_name) == Some(name.as_str())),
        }
        .or_else(|| window.primary_monitor())
        .or_else(|| window.available_monitors().next());

        let Some(monitor) = monitor else {
            return;
        };

        let scale = monitor.scale_factor();
        let pos = monitor.position().to_logical::<f32>(scale);

        // Windows: an explicit InnerSize breaks DWM's per-pixel-alpha
        // compositing on HDR systems. Un-maximize, move, re-maximize instead —
        // Windows does the sizing itself. Other platforms keep explicit sizing.
        #[cfg(target_os = "windows")]
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                pos.x, pos.y,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
        #[cfg(not(target_os = "windows"))]
        {
            let size = monitor.size().to_logical::<f32>(scale);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                pos.x, pos.y,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                size.width,
                size.height,
            )));
        }

        // Moving/maximizing can drop always-on-top — re-assert.
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));

        self.last_applied_monitor = Some(target);
    }
}

impl eframe::App for EframeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color().to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
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

        self.apply_monitor_placement(&ctx, frame);
        if let Some(window) = frame.winit_window() {
            let names: Vec<String> = window
                .available_monitors()
                .filter_map(|m| m.name())
                .map(|n| clean_monitor_name(&n).to_string())
                .collect();
            self.app.set_available_monitors(names);

            #[cfg(target_os = "windows")]
            enable_dwm_per_pixel_alpha(window.as_ref());
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

// DWM only composites a window's per-pixel alpha when asked to via
// `DwmEnableBlurBehindWindow`; otherwise the overlay can render opaque (black)
// on some systems. See https://github.com/srwi/keypeek/issues/16
#[cfg(target_os = "windows")]
fn enable_dwm_per_pixel_alpha(handle_source: &impl raw_window_handle::HasWindowHandle) {
    use raw_window_handle::RawWindowHandle;
    use windows::core::BOOL;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmEnableBlurBehindWindow, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
    };
    use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, HGDIOBJ};

    let Ok(RawWindowHandle::Win32(handle)) = handle_source.window_handle().map(|h| h.as_raw())
    else {
        return;
    };
    let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);

    // A region covering the whole window makes DWM honor the window's alpha.
    let region = unsafe { CreateRectRgn(0, 0, -1, -1) };
    let blur_behind = DWM_BLURBEHIND {
        dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
        fEnable: BOOL(1),
        hRgnBlur: region,
        ..Default::default()
    };
    unsafe {
        let _ = DwmEnableBlurBehindWindow(hwnd, &blur_behind);
        let _ = DeleteObject(HGDIOBJ(region.0));
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
            #[cfg(target_os = "windows")]
            enable_dwm_per_pixel_alpha(cc);

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
                last_applied_monitor: None,
                #[cfg(target_os = "linux")]
                x11_above_ticks: 10,
            }))
        }),
    )
}
