pub mod codec;
pub mod discovery;
pub mod driver;
pub(crate) mod keycode_labels;
pub mod presenter;
pub mod profile;
pub mod rpc;

#[allow(unused_imports)]
pub use driver::ZmkProtocol;
pub use presenter::ZmkKeyPresenter;
pub use profile::ZmkEditorProfile;

use std::sync::Arc;
use crate::device_discovery::DeviceDriverScanner;
use crate::firmware::FirmwareBundle;
use crate::key_presenter::KeyPresenter;
use crate::keymap_editor::EditorProfile;
use crate::protocols::{ConnectionSpec, DeviceError, KeyboardProtocol, ZmkTransportConfig};

pub struct ZmkBundle;

impl FirmwareBundle for ZmkBundle {
    fn driver_id(&self) -> &'static str {
        "zmk"
    }

    fn create_presenter(&self) -> Arc<dyn KeyPresenter> {
        Arc::new(ZmkKeyPresenter)
    }

    fn create_profile(&self) -> Arc<dyn EditorProfile> {
        Arc::new(ZmkEditorProfile)
    }

    fn scanners(&self) -> Vec<Box<dyn DeviceDriverScanner>> {
        vec![Box::new(discovery::ZmkScanner)]
    }

    fn connect(&self, spec: &ConnectionSpec) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
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
            _ => Err(DeviceError::Unsupported("Unsupported spec for ZMK bundle".to_string())),
        }
    }
}
