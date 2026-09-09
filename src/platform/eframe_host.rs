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
        // ViewportCommand::Close alone doesn't reliably end the process on X11.
        std::process::exit(0);
    }
}

struct EframeApp {
    app: OverlayApp,
    last_applied_monitor: Option<MonitorSelection>,
    settings_was_visible: bool,
    // winit's always-on-top request is sent before the window is mapped, which
    // EWMH WMs like Mutter ignore, so re-assert it for a few frames after mapping.
    #[cfg(target_os = "linux")]
    x11_above_ticks: u32,
}

#[cfg(target_os = "macos")]
fn get_macos_display_names(monitors: &[winit::monitor::MonitorHandle]) -> Vec<String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;

    let screens = MainThreadMarker::new().map(NSScreen::screens);

    monitors
        .iter()
        .map(|m| {
            if let Some(screens) = &screens {
                let scale = m.scale_factor();
                let log_x = (m.position().x as f64 / scale).round() as i32;
                let log_w = (m.size().width as f64 / scale).round() as i32;
                let log_h = (m.size().height as f64 / scale).round() as i32;

                if let Some(screen) = screens.iter().find(|s| {
                    let f = s.frame();
                    f.origin.x.round() as i32 == log_x
                        && f.size.width.round() as i32 == log_w
                        && f.size.height.round() as i32 == log_h
                }) {
                    return screen.localizedName().to_string();
                }

                if screens.len() == 1 && monitors.len() == 1 {
                    if let Some(first) = screens.iter().next() {
                        return first.localizedName().to_string();
                    }
                }
            }

            m.name().unwrap_or_else(|| "Display".to_string())
        })
        .collect()
}

fn disambiguate_names(base_names: &[String]) -> Vec<String> {
    use std::collections::HashMap;

    let mut counts = HashMap::new();
    for name in base_names {
        *counts.entry(name.as_str()).or_insert(0usize) += 1;
    }

    let mut seen = HashMap::new();
    base_names
        .iter()
        .map(|name| {
            if counts.get(name.as_str()).copied().unwrap_or(0) > 1 {
                let index = seen.entry(name.as_str()).or_insert(0usize);
                *index += 1;
                format!("{name} ({index})")
            } else {
                name.clone()
            }
        })
        .collect()
}

fn get_available_monitors(
    window: &winit::window::Window,
) -> Vec<(String, winit::monitor::MonitorHandle)> {
    let monitors: Vec<winit::monitor::MonitorHandle> = window.available_monitors().collect();
    if monitors.is_empty() {
        return Vec::new();
    }

    #[cfg(target_os = "macos")]
    let base_names = get_macos_display_names(&monitors);

    #[cfg(not(target_os = "macos"))]
    let base_names: Vec<String> = monitors
        .iter()
        .map(|m| {
            m.name()
                .map(|n| clean_monitor_name(&n).to_string())
                .unwrap_or_else(|| "Display".to_string())
        })
        .collect();

    let unique_names = disambiguate_names(&base_names);
    unique_names.into_iter().zip(monitors).collect()
}

fn base_monitor_name(name: &str) -> &str {
    name.strip_suffix(" (1)").unwrap_or(name)
}

fn find_target_monitor(
    available: &[(String, winit::monitor::MonitorHandle)],
    window: &winit::window::Window,
    target: &MonitorSelection,
) -> Option<winit::monitor::MonitorHandle> {
    let target_handle = match target {
        MonitorSelection::Primary => None,
        MonitorSelection::Named(name) => available
            .iter()
            .find(|(n, _)| n == name)
            .or_else(|| {
                let base_target = base_monitor_name(name);
                available
                    .iter()
                    .find(|(n, _)| base_monitor_name(n) == base_target)
            })
            .map(|(_, m)| m.clone()),
    };

    target_handle
        .or_else(|| window.primary_monitor())
        .or_else(|| available.first().map(|(_, m)| m.clone()))
}

/// Position the overlay window to cover the selected monitor.
#[cfg(target_os = "linux")]
fn place_overlay_window(window: &winit::window::Window, monitor: &winit::monitor::MonitorHandle) {
    // On X11, WMs drop AlwaysOnTop when a window is WM-maximized, so we manually
    // size and position it. When moving between different resolutions, order matters
    // to avoid WM boundary clamping: shrink before moving, or move before expanding.
    let current = window.inner_size();
    let target = monitor.size();
    let pos = monitor.position();

    if target.width < current.width || target.height < current.height {
        let _ = window.request_inner_size(target);
        window.set_outer_position(pos);
    } else {
        window.set_outer_position(pos);
        let _ = window.request_inner_size(target);
    }
}

/// Position the overlay window to cover the selected monitor.
#[cfg(not(target_os = "linux"))]
fn place_overlay_window(window: &winit::window::Window, monitor: &winit::monitor::MonitorHandle) {
    // Windows/macOS: window maximization respects the work area (taskbar/dock).
    // On Windows, unmaximize -> move -> maximize also preserves DWM HDR alpha compositing.
    if window.current_monitor().as_ref() != Some(monitor) || !window.is_maximized() {
        window.set_maximized(false);
        window.set_outer_position(monitor.position());
        window.set_maximized(true);
    }
}

impl EframeApp {
    fn apply_monitor_placement(&mut self, ctx: &egui::Context, frame: &eframe::Frame) -> bool {
        let target = self.app.active_monitor().clone();
        if self.last_applied_monitor.as_ref() == Some(&target) {
            return false;
        }

        let Some(window) = frame.winit_window() else {
            return false;
        };

        let available = get_available_monitors(window.as_ref());
        let Some(monitor) = find_target_monitor(&available, window.as_ref(), &target) else {
            return false;
        };

        place_overlay_window(window.as_ref(), &monitor);

        // Moving/maximizing can drop always-on-top — re-assert.
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));

        let names: Vec<String> = available.into_iter().map(|(name, _)| name).collect();
        self.app.set_available_monitors(names);

        #[cfg(target_os = "windows")]
        enable_dwm_per_pixel_alpha(window.as_ref());

        self.last_applied_monitor = Some(target);
        true
    }

    fn update_available_monitors(&mut self, window: &winit::window::Window) {
        let names: Vec<String> = get_available_monitors(window)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        self.app.set_available_monitors(names);
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
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
            ctx.request_repaint();
        }

        if self.apply_monitor_placement(&ctx, frame) {
            ctx.request_repaint();
            return;
        }

        let settings_open = self.app.ui.settings_visible;
        if settings_open && !self.settings_was_visible {
            if let Some(window) = frame.winit_window() {
                self.update_available_monitors(window.as_ref());
            }
        }
        self.settings_was_visible = settings_open;

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
        .with_transparent(true)
        .with_has_shadow(false)
        .with_always_on_top();

    #[cfg(not(target_os = "linux"))]
    {
        viewport = viewport.with_maximized(true);
    }

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
            super::add_phosphor_to_fonts(&mut fonts);
            cc.egui_ctx.set_fonts(fonts);

            let app = OverlayApp::new(tray_icon, settings_requested, ui_wake, settings, devices);
            let settings_was_visible = app.ui.settings_visible;
            Ok(Box::new(EframeApp {
                app,
                last_applied_monitor: None,
                settings_was_visible,
                #[cfg(target_os = "linux")]
                x11_above_ticks: 10,
            }))
        }),
    )
}
