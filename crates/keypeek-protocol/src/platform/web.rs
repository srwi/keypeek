use std::sync::Arc;

use crate::application::keyboard::Keyboard;
use crate::device_discovery::DiscoveredDevice;
use keypeek_core::keymap_editor::EditorProfile;
use keypeek_core::OverlayConfig;
use crate::protocols::{DeviceError, KeyboardProtocol};
use crate::ui_wake::UiWake;

pub struct ConnectedWebDevice {
    pub device: DiscoveredDevice,
    pub keyboard: Arc<Keyboard>,
    pub profile: Arc<dyn EditorProfile>,
}

impl ConnectedWebDevice {
    /// Constructs a connected web device, selecting the correct presenter and editor profile
    /// via the firmware bundle registered for the device's connection spec.
    pub fn new(
        device: DiscoveredDevice,
        protocol: impl KeyboardProtocol + 'static,
        overlay_config: OverlayConfig,
        ui_wake: UiWake,
    ) -> Result<Self, DeviceError> {
        let bundle = crate::firmware::bundle_for_spec(&device.spec);
        let selected_layout = protocol
            .get_layout_definition()
            .get_layout_names()
            .first()
            .cloned()
            .unwrap_or_else(|| "default".to_string());

        let keyboard = Keyboard::new(
            Box::new(protocol),
            selected_layout,
            overlay_config,
            ui_wake,
            bundle.create_presenter(),
        )
        .map_err(DeviceError::Protocol)?;

        Ok(Self {
            device,
            keyboard: Arc::new(keyboard),
            profile: bundle.create_profile(),
        })
    }
}
