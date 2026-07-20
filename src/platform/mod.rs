use crate::device_discovery::DiscoveredDevice;
use crate::settings::Settings;

mod eframe_host;

#[cfg(target_os = "linux")]
mod wayland;

/// Description of a physical or logical display available to the overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenInfo {
    /// Stable-enough identifier persisted in settings (typically the monitor name).
    pub id: String,
    /// Human-readable label shown in the UI.
    pub name: String,
}

// eframe (winit) can't do always-on-top/click-through on native Wayland, so on
// Linux Wayland sessions we drive a wlr-layer-shell surface directly instead.
pub trait OverlayHost {
    fn set_passthrough(&mut self, enabled: bool);
    fn request_close(&mut self);
    fn available_screens(&self) -> Vec<ScreenInfo>;
    fn current_screen(&self) -> Option<String>;
    fn move_to_screen(&mut self, screen_id: &str);
}

pub fn run(
    settings: Settings,
    devices: Vec<DiscoveredDevice>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        // `WAYLAND_DISPLAY` is unset under XWayland, so X11 falls through to eframe below.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            match wayland::run(settings.clone(), devices.clone()) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // No wlr-layer-shell (e.g. GNOME/Mutter): fall back to eframe on
                    // XWayland, since native Wayland ignores always-on-top.
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
