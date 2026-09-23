pub mod codec;
pub mod common;
#[cfg(feature = "desktop")]
pub mod discovery;
pub mod driver;
#[cfg(test)]
pub(crate) mod keycode_labels;
pub mod presenter;
pub mod profile;
#[cfg(feature = "desktop")]
pub mod rpc;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use driver::ZmkProtocol;
pub use presenter::ZmkKeyPresenter;
pub use profile::ZmkEditorProfile;

use crate::device_discovery::DeviceDriverScanner;
use crate::firmware::FirmwareBundle;
use crate::key_presenter::KeyPresenter;
use crate::keymap_editor::EditorProfile;
#[cfg(feature = "desktop")]
use crate::protocols::ZmkTransportConfig;
use crate::protocols::{ConnectionSpec, DeviceError, KeyboardProtocol};
use std::sync::Arc;

pub struct ZmkBundle;

impl FirmwareBundle for ZmkBundle {
    fn create_presenter(&self) -> Arc<dyn KeyPresenter> {
        Arc::new(ZmkKeyPresenter)
    }

    fn create_profile(&self) -> Arc<dyn EditorProfile> {
        Arc::new(ZmkEditorProfile)
    }

    fn scanners(&self) -> Vec<Box<dyn DeviceDriverScanner>> {
        #[cfg(feature = "desktop")]
        {
            vec![Box::new(discovery::ZmkScanner)]
        }
        #[cfg(not(feature = "desktop"))]
        {
            vec![]
        }
    }

    fn connect(&self, spec: &ConnectionSpec) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
        #[cfg(feature = "desktop")]
        match spec {
            ConnectionSpec::Zmk {
                vid,
                pid,
                transport,
            } => {
                let zmk_transport = match transport {
                    ZmkTransportConfig::Serial(port_name) => {
                        rpc::ZmkTransport::SerialPort(port_name.clone())
                    }
                    ZmkTransportConfig::Ble(device_id) => {
                        rpc::ZmkTransport::BleDevice(device_id.clone())
                    }
                };
                let protocol = driver::ZmkProtocol::connect_live(*vid, *pid, &zmk_transport)?;
                Ok(Box::new(protocol))
            }
            _ => Err(DeviceError::Unsupported(
                "Unsupported spec for ZMK bundle".to_string(),
            )),
        }
        #[cfg(not(feature = "desktop"))]
        {
            let _ = spec;
            Err(DeviceError::Unsupported(
                "Native ZMK transport is not supported in this build".to_string(),
            ))
        }
    }
}
