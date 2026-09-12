pub mod presenter;
pub mod profile;

pub use presenter::ZmkKeyPresenter;
pub use profile::ZmkEditorProfile;

use std::sync::Arc;
use crate::device_discovery::DeviceDriverScanner;
use crate::firmware::FirmwareBundle;
use crate::key_presenter::KeyPresenter;
use crate::keymap_editor::EditorProfile;

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
        vec![Box::new(crate::protocols::zmk_discovery::ZmkScanner)]
    }
}
