pub mod driver;

#[cfg(test)]
pub use driver::mock_device;
pub use driver::MockProtocol;
use driver::MockScanner;

use std::sync::Arc;

use crate::device_discovery::DeviceDriverScanner;
use crate::firmware::qmk::{QmkEditorProfile, QmkKeyPresenter};
use crate::firmware::FirmwareBundle;
use crate::key_presenter::KeyPresenter;
use crate::keymap_editor::EditorProfile;
use crate::protocols::{ConnectionSpec, DeviceError, KeyboardProtocol};

pub struct MockBundle;

impl FirmwareBundle for MockBundle {
    fn create_presenter(&self) -> Arc<dyn KeyPresenter> {
        Arc::new(QmkKeyPresenter)
    }

    fn create_profile(&self) -> Arc<dyn EditorProfile> {
        Arc::new(QmkEditorProfile)
    }

    fn scanners(&self) -> Vec<Box<dyn DeviceDriverScanner>> {
        if cfg!(debug_assertions) {
            vec![Box::new(MockScanner)]
        } else {
            Vec::new()
        }
    }

    fn connect(&self, spec: &ConnectionSpec) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
        match spec {
            ConnectionSpec::Mock => {
                let protocol = MockProtocol::connect()?;
                Ok(Box::new(protocol))
            }
            _ => Err(DeviceError::Unsupported(
                "Unsupported spec for Mock bundle".to_string(),
            )),
        }
    }
}
