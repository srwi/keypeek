//! Application services for connection orchestration, device discovery, and sessions.

pub mod connection;
pub mod connection_manager;
pub mod device_discovery;
pub mod keyboard;
pub mod session;
pub mod ui_wake;

pub use connection::{ConnectionRequest, ConnectionTask};
pub use connection_manager::{
    ConnectOutcome, ConnectionEvent, ConnectionStatus, DeviceConnectionManager,
};
pub use device_discovery::{
    discover_devices, DeviceDriverScanner, DiscoveredDevice, DiscoveryContext, HidDeviceInfo,
};
pub use keyboard::Keyboard;
pub use session::{KeyboardSession, KeymapCommand};
pub use ui_wake::UiWake;
