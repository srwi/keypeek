//! Application services for connection orchestration, device discovery, and sessions.

pub mod connection;
pub mod device_discovery;
pub mod session;

#[allow(unused_imports)]
pub use connection::{ConnectionRequest, ConnectionTask};
#[allow(unused_imports)]
pub use device_discovery::{
    discover_devices, DeviceDriverScanner, DiscoveredDevice, DiscoveryContext, HidDeviceInfo,
};
#[allow(unused_imports)]
pub use session::{KeyboardSession, KeymapCommand};
