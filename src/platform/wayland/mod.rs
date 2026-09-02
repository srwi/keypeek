// Drives the egui UI directly on a smithay-client-toolkit event loop, placing the
// overlay on a wlr-layer-shell overlay surface so it stays above other windows and
// click-through without WM cooperation (winit/eframe can't do this on Wayland).

mod egl;
mod input;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use smithay_client_toolkit::reexports::protocols::wp::{
    fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::{self, WpFractionalScaleV1},
    },
    viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    reexports::calloop::{ping::make_ping, EventLoop},
    reexports::calloop_wayland_source::WaylandSource,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use wayland_client::{
    delegate_noop,
    globals::registry_queue_init,
    protocol::{
        wl_keyboard::WlKeyboard, wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, QueueHandle,
};

use egui_glow::glow;

use crate::device_discovery::DiscoveredDevice;
use crate::overlay_window::OverlayApp;
use crate::platform::OverlayHost;
use crate::settings::Settings;
use crate::ui_wake::UiWake;

use egl::EglState;
use input::InputState;

/// Records the egui UI's host requests during one frame, applied after `ui()` returns
/// (we can't touch the Wayland objects while the app borrow is live).
#[derive(Default)]
struct WaylandHost {
    close: bool,
    passthrough: Option<bool>,
}

impl OverlayHost for WaylandHost {
    fn set_passthrough(&mut self, enabled: bool) {
        self.passthrough = Some(enabled);
    }

    fn request_close(&mut self) {
        self.close = true;
    }
}

struct WaylandApp {
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,
    compositor_state: CompositorState,

    layer: LayerSurface,
    /// Maps the buffer to the logical surface size under fractional scaling;
    /// `None` when the compositor lacks wp-fractional-scale/wp-viewporter.
    viewport: Option<WpViewport>,
    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,

    egui_ctx: egui::Context,
    app: OverlayApp,
    input: InputState,

    egl: Option<EglState>,
    painter: Option<egui_glow::Painter>,
    gl: Option<Arc<glow::Context>>,

    /// Surface size in logical points and the (possibly fractional) scale.
    width: i32,
    height: i32,
    scale: f64,

    configured: bool,
    needs_redraw: bool,
    exit: bool,
    repaint_at: Option<Instant>,
}

pub fn run(
    settings: Settings,
    devices: Vec<DiscoveredDevice>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init::<WaylandApp>(&conn)?;
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let shell = LayerShell::bind(&globals, &qh)?;

    // Build the overlay layer surface: above everything, covering the whole output,
    // initially interactive because the settings window opens on first launch.
    let surface = compositor_state.create_surface(&qh);
    let layer = shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("keypeek"), None);
    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    layer.set_size(0, 0); // 0,0 + all-edge anchors => compositor sizes it to the output
    layer.commit();

    // Fractional scaling needs both protocols: the scale event and a viewport to
    // map the scaled buffer back to the logical size. Otherwise integer scale.
    let viewporter = globals
        .bind::<WpViewporter, WaylandApp, _>(&qh, 1..=1, ())
        .ok();
    let fractional_manager = globals
        .bind::<WpFractionalScaleManagerV1, WaylandApp, _>(&qh, 1..=1, ())
        .ok();
    let viewport = match (&viewporter, &fractional_manager) {
        (Some(viewporter), Some(manager)) => {
            manager.get_fractional_scale(layer.wl_surface(), &qh, ());
            Some(viewporter.get_viewport(layer.wl_surface(), &qh, ()))
        }
        _ => None,
    };

    // calloop loop, plus a ping the worker threads use to request repaints.
    let mut event_loop: EventLoop<WaylandApp> = EventLoop::try_new()?;
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn.clone(), event_queue)
        .insert(loop_handle.clone())
        .map_err(|e| format!("failed to insert Wayland event source: {e}"))?;

    let (ping, ping_source) = make_ping()?;
    loop_handle
        .insert_source(ping_source, |_, _, app: &mut WaylandApp| {
            app.needs_redraw = true;
        })
        .map_err(|e| format!("failed to insert repaint ping source: {e}"))?;

    let egui_ctx = egui::Context::default();
    egui_extras::install_image_loaders(&egui_ctx);
    let mut fonts = egui::FontDefinitions::default();
    super::add_phosphor_to_fonts(&mut fonts);
    egui_ctx.set_fonts(fonts);

    let ui_wake = UiWake::new(Arc::new(move || ping.ping()));
    let settings_requested = Arc::new(AtomicBool::new(false));
    let tray_icon = crate::tray::create_tray_icon({
        let settings_requested = settings_requested.clone();
        let ui_wake = ui_wake.clone();
        Arc::new(move || {
            settings_requested.store(true, Ordering::Relaxed);
            ui_wake.request_repaint();
        })
    });
    let app = OverlayApp::new(tray_icon, settings_requested, ui_wake, settings, devices);

    let mut state = WaylandApp {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        compositor_state,
        layer,
        viewport,
        keyboard: None,
        pointer: None,
        egui_ctx,
        app,
        input: InputState::default(),
        egl: None,
        painter: None,
        gl: None,
        width: 0,
        height: 0,
        scale: 1.0,
        configured: false,
        needs_redraw: false,
        exit: false,
        repaint_at: None,
    };

    while !state.exit {
        // Sleep until the next scheduled repaint; wake immediately when a redraw is
        // already pending, and block indefinitely (until an event) when idle.
        let timeout = if state.needs_redraw && state.configured {
            Some(Duration::ZERO)
        } else {
            state
                .repaint_at
                .map(|at| at.saturating_duration_since(Instant::now()))
        };
        event_loop.dispatch(timeout, &mut state)?;

        if let Some(at) = state.repaint_at {
            if Instant::now() >= at {
                state.repaint_at = None;
                state.needs_redraw = true;
            }
        }
        if state.needs_redraw && state.configured {
            state.draw(&conn);
        }
    }

    Ok(())
}

impl WaylandApp {
    fn size_px(&self) -> [u32; 2] {
        [
            (self.width as f64 * self.scale).round().max(1.0) as u32,
            (self.height as f64 * self.scale).round().max(1.0) as u32,
        ]
    }

    /// (Re)create the EGL context + egui_glow painter for the current surface size.
    fn init_or_resize_gl(&mut self, conn: &Connection) {
        let [w, h] = self.size_px();
        if let Some(viewport) = &self.viewport {
            // Buffer scale stays 1; the viewport maps the scaled buffer to the
            // logical surface size.
            viewport.set_destination(self.width.max(1), self.height.max(1));
        } else {
            self.layer.wl_surface().set_buffer_scale(self.scale as i32);
        }

        // Already initialized: just resize the EGL window.
        if let Some(egl) = self.egl.as_ref() {
            egl.resize(w as i32, h as i32);
            return;
        }

        // First-time setup.
        let surface = self.layer.wl_surface();
        match EglState::new(conn, surface, w as i32, h as i32) {
            Ok(egl) => {
                let gl = egl.gl.clone();
                match egui_glow::Painter::new(gl.clone(), "", None, false) {
                    Ok(painter) => {
                        self.painter = Some(painter);
                        self.gl = Some(gl);
                        self.egl = Some(egl);
                    }
                    Err(e) => eprintln!("KeyPeek: egui_glow painter init failed: {e}"),
                }
            }
            Err(e) => eprintln!("KeyPeek: EGL init failed: {e}"),
        }
    }

    fn draw(&mut self, _conn: &Connection) {
        self.needs_redraw = false;
        let Some(egl) = self.egl.as_ref() else {
            return;
        };
        if egl.make_current().is_err() {
            return;
        }

        self.egui_ctx.set_pixels_per_point(self.scale as f32);
        let raw_input = self.input.take_raw_input((self.width, self.height));

        let ctx = self.egui_ctx.clone();
        let mut host = WaylandHost::default();
        let full_output = {
            let app = &mut self.app;
            ctx.begin_pass(raw_input);
            app.ui(&ctx, &mut host);
            ctx.end_pass()
        };

        let clear = self.app.clear_color().to_array();
        let primitives = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let size_px = self.size_px();

        if let (Some(painter), Some(gl)) = (self.painter.as_mut(), self.gl.as_ref()) {
            egui_glow::painter::clear(gl, size_px, clear);
            painter.paint_and_update_textures(
                size_px,
                full_output.pixels_per_point,
                &primitives,
                &full_output.textures_delta,
            );
        }
        // Apply host requests only after this, the last use of the `self.egl` borrow.
        let _ = egl.swap_buffers();

        // Honor egui's requested repaint cadence (e.g. the overlay fade-out animation).
        if let Some(viewport) = full_output.viewport_output.get(&egui::ViewportId::ROOT) {
            let delay = viewport.repaint_delay;
            if delay.is_zero() {
                self.needs_redraw = true;
            } else if delay < Duration::from_secs(24 * 60 * 60) {
                self.repaint_at = Some(Instant::now() + delay);
            }
        }

        if host.close {
            self.exit = true;
        }
        if let Some(passthrough) = host.passthrough {
            self.apply_passthrough(passthrough);
        }
    }

    /// Toggle click-through + focus to match the eframe `MousePassthrough` behavior.
    fn apply_passthrough(&mut self, passthrough: bool) {
        let surface = self.layer.wl_surface();
        if passthrough {
            // An empty input region makes every pointer/touch event fall through.
            if let Ok(region) = Region::new(&self.compositor_state) {
                surface.set_input_region(Some(region.wl_region()));
            }
            self.layer
                .set_keyboard_interactivity(KeyboardInteractivity::None);
        } else {
            // `None` input region = the whole surface receives input again.
            surface.set_input_region(None);
            self.layer
                .set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        }
        surface.commit();
    }
}

impl CompositorHandler for WaylandApp {
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        new_factor: i32,
    ) {
        // With fractional scaling active, the wp-fractional-scale event is authoritative.
        if self.viewport.is_some() {
            return;
        }
        let new_factor = new_factor as f64;
        if new_factor != self.scale && new_factor > 0.0 {
            self.scale = new_factor;
            if self.configured {
                self.init_or_resize_gl(conn);
                self.needs_redraw = true;
            }
        }
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: wayland_client::protocol::wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlSurface, _time: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &WlOutput,
    ) {
    }
}

impl LayerShellHandler for WaylandApp {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (mut w, mut h) = configure.new_size;
        // 0 means "pick your own size"; fall back to a sane default until an output
        // reports real dimensions.
        if w == 0 {
            w = if self.width > 0 {
                self.width as u32
            } else {
                1280
            };
        }
        if h == 0 {
            h = if self.height > 0 {
                self.height as u32
            } else {
                720
            };
        }
        self.width = w as i32;
        self.height = h as i32;

        self.init_or_resize_gl(conn);
        self.configured = true;
        self.needs_redraw = true;
    }
}

impl SeatHandler for WaylandApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            if let Ok(kbd) = self.seat_state.get_keyboard(qh, &seat, None) {
                self.keyboard = Some(kbd);
            }
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(ptr) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(ptr);
            }
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(kbd) = self.keyboard.take() {
                kbd.release();
            }
        }
        if capability == Capability::Pointer {
            if let Some(ptr) = self.pointer.take() {
                ptr.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

impl KeyboardHandler for WaylandApp {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: &WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: &WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.input.key(event.keysym, event.utf8.as_deref(), true);
        self.needs_redraw = true;
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.input.key(event.keysym, event.utf8.as_deref(), false);
        self.needs_redraw = true;
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.input.set_modifiers(modifiers);
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }
}

impl PointerHandler for WaylandApp {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let pos = egui::pos2(event.position.0 as f32, event.position.1 as f32);
            match event.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                    self.input.pointer_moved(pos);
                }
                PointerEventKind::Leave { .. } => self.input.pointer_left(),
                PointerEventKind::Press { button, .. } => {
                    if let Some(b) = map_button(button) {
                        self.input.pointer_moved(pos);
                        self.input.pointer_button(b, true);
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(b) = map_button(button) {
                        self.input.pointer_button(b, false);
                    }
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    // Wayland axis is positive-down; egui scroll is positive-up.
                    let delta = egui::vec2(-horizontal.absolute as f32, -vertical.absolute as f32);
                    self.input.scroll(delta);
                }
            }
        }
        self.needs_redraw = true;
    }
}

/// Linux evdev button codes -> egui pointer buttons.
fn map_button(code: u32) -> Option<egui::PointerButton> {
    match code {
        0x110 => Some(egui::PointerButton::Primary), // BTN_LEFT
        0x111 => Some(egui::PointerButton::Secondary), // BTN_RIGHT
        0x112 => Some(egui::PointerButton::Middle),  // BTN_MIDDLE
        _ => None,
    }
}

impl OutputHandler for WaylandApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
}

impl Dispatch<WpFractionalScaleV1, ()> for WaylandApp {
    fn event(
        state: &mut Self,
        _: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &(),
        conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // The compositor reports the preferred scale in 120ths.
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            let new_scale = scale as f64 / 120.0;
            if new_scale > 0.0 && new_scale != state.scale {
                state.scale = new_scale;
                if state.configured {
                    state.init_or_resize_gl(conn);
                    state.needs_redraw = true;
                }
            }
        }
    }
}

delegate_noop!(WaylandApp: ignore WpFractionalScaleManagerV1);
delegate_noop!(WaylandApp: ignore WpViewporter);
delegate_noop!(WaylandApp: ignore WpViewport);

impl ProvidesRegistryState for WaylandApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(WaylandApp);
delegate_output!(WaylandApp);
delegate_seat!(WaylandApp);
delegate_keyboard!(WaylandApp);
delegate_pointer!(WaylandApp);
delegate_layer!(WaylandApp);
delegate_registry!(WaylandApp);
