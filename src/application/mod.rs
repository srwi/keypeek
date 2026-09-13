//! Application services for connection orchestration, device discovery, and sessions.

pub mod connection;
pub mod device_discovery;
pub mod keyboard;
pub mod session;

pub use connection::{ConnectionRequest, ConnectionTask};
pub use device_discovery::{
    discover_devices, DeviceDriverScanner, DiscoveredDevice, DiscoveryContext, HidDeviceInfo,
};
pub use keyboard::Keyboard;
pub use session::{KeyboardSession, KeymapCommand};
