pub mod codec;
pub mod common;
pub mod discovery;
pub mod json_parser;
pub(crate) mod keycode_labels;
pub mod kle_parser;
pub mod presenter;
pub mod profile;
pub mod via;
pub mod vial;

pub use presenter::QmkKeyPresenter;
pub use profile::QmkEditorProfile;

use std::sync::Arc;
use crate::device_discovery::DeviceDriverScanner;
use crate::firmware::FirmwareBundle;
use crate::key_presenter::KeyPresenter;
use crate::keymap_editor::EditorProfile;
use crate::protocols::{ConnectionSpec, DeviceError, KeyboardProtocol};

pub struct QmkBundle;

impl FirmwareBundle for QmkBundle {
    fn driver_id(&self) -> &'static str {
        "qmk"
    }

    fn create_presenter(&self) -> Arc<dyn KeyPresenter> {
        Arc::new(QmkKeyPresenter)
    }

    fn create_profile(&self) -> Arc<dyn EditorProfile> {
        Arc::new(QmkEditorProfile)
    }

    fn scanners(&self) -> Vec<Box<dyn DeviceDriverScanner>> {
        vec![Box::new(discovery::QmkScanner)]
    }

    fn connect(&self, spec: &ConnectionSpec) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
        match spec {
            ConnectionSpec::Via { json_path } => {
                let protocol = via::ViaProtocol::connect(json_path)?;
                Ok(Box::new(protocol))
            }
            ConnectionSpec::Vial { vid, pid } => {
                let protocol = vial::VialProtocol::connect(*vid, *pid)?;
                Ok(Box::new(protocol))
            }
            _ => Err(DeviceError::Unsupported("Unsupported spec for QMK bundle".to_string())),
        }
    }
}
