use std::ffi::c_void;
use std::sync::Arc;

use egui_glow::glow;
use khronos_egl as egl;
use wayland_client::{Connection, Proxy};
use wayland_egl::WlEglSurface;

type Egl = egl::Instance<egl::Static>;

pub struct EglState {
    egl: Egl,
    display: egl::Display,
    surface: egl::Surface,
    context: egl::Context,
    wl_egl_surface: WlEglSurface,
    pub gl: Arc<glow::Context>,
}

impl EglState {
    pub fn new(
        conn: &Connection,
        wl_surface: &wayland_client::protocol::wl_surface::WlSurface,
        width: i32,
        height: i32,
    ) -> Result<Self, String> {
        let egl = egl::Instance::new(egl::Static);

        // The EGL display is derived from the Wayland display pointer.
        let display_ptr = conn.backend().display_ptr() as *mut c_void;
        let display = unsafe { egl.get_display(display_ptr) }.ok_or("eglGetDisplay failed")?;
        egl.initialize(display)
            .map_err(|e| format!("eglInitialize failed: {e}"))?;
        egl.bind_api(egl::OPENGL_ES_API)
            .map_err(|e| format!("eglBindAPI failed: {e}"))?;

        let config_attribs = [
            egl::SURFACE_TYPE,
            egl::WINDOW_BIT,
            egl::RENDERABLE_TYPE,
            egl::OPENGL_ES2_BIT,
            egl::RED_SIZE,
            8,
            egl::GREEN_SIZE,
            8,
            egl::BLUE_SIZE,
            8,
            egl::ALPHA_SIZE,
            8,
            egl::NONE,
        ];
        let config = egl
            .choose_first_config(display, &config_attribs)
            .map_err(|e| format!("eglChooseConfig failed: {e}"))?
            .ok_or("no matching EGL config (need ARGB for transparency)")?;

        let context_attribs = [egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE];
        let context = egl
            .create_context(display, config, None, &context_attribs)
            .map_err(|e| format!("eglCreateContext failed: {e}"))?;

        let wl_egl_surface = WlEglSurface::new(wl_surface.id(), width.max(1), height.max(1))
            .map_err(|e| format!("WlEglSurface::new failed: {e}"))?;

        let surface = unsafe {
            egl.create_window_surface(
                display,
                config,
                wl_egl_surface.ptr() as egl::NativeWindowType,
                None,
            )
        }
        .map_err(|e| format!("eglCreateWindowSurface failed: {e}"))?;

        egl.make_current(display, Some(surface), Some(surface), Some(context))
            .map_err(|e| format!("eglMakeCurrent failed: {e}"))?;
        // Sync swaps to the compositor; avoids tearing on the overlay.
        let _ = egl.swap_interval(display, 1);

        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                egl.get_proc_address(name)
                    .map_or(std::ptr::null(), |p| p as *const c_void)
            })
        };

        Ok(Self {
            egl,
            display,
            surface,
            context,
            wl_egl_surface,
            gl: Arc::new(gl),
        })
    }

    pub fn make_current(&self) -> Result<(), String> {
        self.egl
            .make_current(
                self.display,
                Some(self.surface),
                Some(self.surface),
                Some(self.context),
            )
            .map_err(|e| format!("eglMakeCurrent failed: {e}"))
    }

    pub fn resize(&self, width: i32, height: i32) {
        self.wl_egl_surface
            .resize(width.max(1), height.max(1), 0, 0);
    }

    pub fn swap_buffers(&self) -> Result<(), String> {
        self.egl
            .swap_buffers(self.display, self.surface)
            .map_err(|e| format!("eglSwapBuffers failed: {e}"))
    }
}
