//! Windowing hosts.
//!
//! The UI in [`crate::overlay_window`] is pure egui and never names a windowing
//! toolkit. Everything it needs from the host is expressed through [`OverlayHost`].
//! Two hosts implement it:
//!
//! - [`eframe_host`] — wraps `eframe::run_native`; used on Windows, macOS, and
//!   Linux/X11 (and Linux/XWayland). This is the original, proven path.
//! - [`wayland`] — a `smithay-client-toolkit` event loop that puts the overlay on a
//!   `wlr-layer-shell` overlay surface. Used only on Linux Wayland sessions, where
//!   xdg-shell (and therefore winit/eframe) cannot do always-on-top + click-through.
//!
//! Dependency direction is strictly `platform -> overlay_window -> domain`; the UI
//! never reaches back into a host.

use crate::device_discovery::DiscoveredDevice;
use crate::settings::Settings;

mod eframe_host;

#[cfg(target_os = "linux")]
mod wayland;

/// The small surface the egui UI needs from whatever windowing loop drives it.
pub trait OverlayHost {
    /// Toggle click-through. eframe maps this to `ViewportCommand::MousePassthrough`;
    /// the Wayland host swaps the surface input region between full and empty.
    fn set_passthrough(&mut self, enabled: bool);

    /// Ask the application to quit (tray Quit, or closing settings before any
    /// successful connection).
    fn request_close(&mut self);
}

/// Pick a host and run until the app exits.
///
/// Linux Wayland sessions get the layer-shell host; everything else (including
/// Linux/X11 and XWayland) uses eframe.
pub fn run(
    settings: Settings,
    devices: Vec<DiscoveredDevice>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        // `WAYLAND_DISPLAY` is set for native Wayland sessions but not under
        // XWayland, so X11 and XWayland correctly fall through to eframe.
        //
        // If the layer-shell host can't start (e.g. GNOME/Mutter, which does not
        // implement wlr-layer-shell), fall back to eframe — which will run under
        // XWayland if available. We clone the inputs so the fallback can reuse them.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            match wayland::run(settings.clone(), devices.clone()) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // No wlr-layer-shell (e.g. GNOME/Mutter). Fall back to eframe,
                    // forcing the XWayland backend so the overlay can stay
                    // always-on-top (native Wayland ignores that request).
                    eprintln!(
                        "KeyPeek: Wayland layer-shell host unavailable ({e}); \
                         falling back to eframe on XWayland for always-on-top."
                    );
                    return Ok(eframe_host::run(settings, devices, true)?);
                }
            }
        }
    }

    eframe_host::run(settings, devices, false)?;
    Ok(())
}
