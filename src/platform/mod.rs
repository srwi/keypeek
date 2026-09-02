use crate::device_discovery::DiscoveredDevice;
use crate::settings::Settings;

mod eframe_host;

#[cfg(target_os = "linux")]
mod wayland;

// eframe (winit) can't do always-on-top/click-through on native Wayland, so on
// Linux Wayland sessions we drive a wlr-layer-shell surface directly instead.
pub trait OverlayHost {
    fn set_passthrough(&mut self, enabled: bool);
    fn request_close(&mut self);
}

/// Registers Phosphor icons into the egui font definitions.
pub(crate) fn add_phosphor_to_fonts(fonts: &mut egui::FontDefinitions) {
    fonts.font_data.insert(
        "phosphor".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(
            egui_phosphor::Variant::Regular.font_bytes(),
        )),
    );

    if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        let index = 1.min(font_keys.len());
        font_keys.insert(index, "phosphor".to_owned());
    }
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
