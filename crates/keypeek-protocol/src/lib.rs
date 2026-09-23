pub mod key_presenter;

pub mod application;
pub mod firmware;
pub mod hid_labels;
pub mod os_layout;
pub mod platform {
    #[cfg(feature = "desktop")]
    pub mod hid;
    #[cfg(target_arch = "wasm32")]
    pub mod web;
    #[cfg(target_arch = "wasm32")]
    pub mod web_hid;
    #[cfg(target_arch = "wasm32")]
    pub mod web_serial;
}
pub mod protocols;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use application::{
    connection::{self, ConnectedState, ConnectionRequest, ConnectionTask},
    connection_manager::{self, ConnectionState, DeviceConnectionManager},
    device_discovery::{self, discover_devices, DeviceDriverScanner, DiscoveredDevice},
    keyboard::{self, Keyboard},
    session::{self, KeyboardSession},
    ui_wake::{self, UiWake},
};

pub use firmware::{
    all_bundles, bundle_for_spec, connect_protocol, default_scanners, FirmwareBundle,
};
pub use protocols::{
    ActionFilter, ConnectionSpec, DeviceError, DeviceEvent, KeyboardProtocol, RawHidTransport,
    Reopener, WriteSupport,
};

#[cfg(feature = "desktop")]
pub use platform::hid::scan_all_hid;

