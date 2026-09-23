pub mod codec;
pub mod common;
pub mod discovery;
pub mod json_parser;
pub(crate) mod keycode_labels;
pub mod kle_parser;
pub mod presenter;
pub mod profile;
#[cfg(feature = "hidapi")]
pub mod via;
#[cfg(feature = "hidapi")]
pub mod vial;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use common::QmkProtocol;
pub use presenter::QmkKeyPresenter;
pub use profile::QmkEditorProfile;

use crate::device_discovery::DeviceDriverScanner;
use crate::firmware::FirmwareBundle;
use keypeek_core::keymap_editor::EditorProfile;
use keypeek_core::KeyPresenter;
use crate::protocols::{ConnectionSpec, DeviceError, KeyboardProtocol};
use std::sync::Arc;

pub struct QmkBundle;

impl FirmwareBundle for QmkBundle {
    fn create_presenter(&self) -> Arc<dyn KeyPresenter> {
        Arc::new(QmkKeyPresenter)
    }

    fn create_profile(&self) -> Arc<dyn EditorProfile> {
        Arc::new(QmkEditorProfile)
    }

    fn scanners(&self) -> Vec<Box<dyn DeviceDriverScanner>> {
        #[cfg(feature = "hidapi")]
        {
            vec![Box::new(discovery::QmkScanner)]
        }
        #[cfg(not(feature = "hidapi"))]
        {
            vec![]
        }
    }

    fn connect(&self, spec: &ConnectionSpec) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
        #[cfg(feature = "hidapi")]
        match spec {
            ConnectionSpec::Via { json_path } => {
                let protocol = via::ViaProtocol::connect(json_path)?;
                Ok(Box::new(protocol))
            }
            ConnectionSpec::Vial { vid, pid } => {
                let protocol = vial::VialProtocol::connect(*vid, *pid)?;
                Ok(Box::new(protocol))
            }
            _ => Err(DeviceError::Unsupported(
                "Unsupported spec for QMK bundle".to_string(),
            )),
        }
        #[cfg(not(feature = "hidapi"))]
        {
            let _ = spec;
            Err(DeviceError::Unsupported(
                "Native QMK HID is not supported in this build".to_string(),
            ))
        }
    }
}
