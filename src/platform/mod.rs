#[cfg(feature = "desktop")]
use std::sync::Arc;

#[cfg(feature = "desktop")]
use crate::device_discovery::DiscoveredDevice;
#[cfg(feature = "desktop")]
use crate::settings::SettingsStore;

#[cfg(feature = "desktop")]
mod eframe_host;
#[cfg(feature = "desktop")]
pub mod hid;
#[cfg(feature = "desktop")]
pub(crate) mod tray;

#[cfg(feature = "desktop")]
pub use hid::scan_all_hid;

#[cfg(target_os = "linux")]
mod wayland;

/// Registers Phosphor icons into the egui font definitions.
#[allow(dead_code)]
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

#[cfg(feature = "desktop")]
pub fn run(
    settings_store: Arc<dyn SettingsStore>,
    devices: Vec<DiscoveredDevice>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        // `WAYLAND_DISPLAY` is unset under XWayland, so X11 falls through to eframe below.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            match wayland::run(settings_store.clone(), devices.clone()) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // No wlr-layer-shell (e.g. GNOME/Mutter): fall back to eframe on
                    // XWayland, since native Wayland ignores always-on-top.
                    eprintln!(
                        "KeyPeek: Wayland layer-shell host unavailable ({e}); \
                         falling back to eframe on XWayland for always-on-top."
                    );
                    return Ok(eframe_host::run(settings_store, devices, true)?);
                }
            }
        }
    }

    eframe_host::run(settings_store, devices, false)?;
    Ok(())
}
