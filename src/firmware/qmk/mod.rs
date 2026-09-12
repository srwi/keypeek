pub mod presenter;
pub mod profile;

pub use presenter::QmkKeyPresenter;
pub use profile::QmkEditorProfile;

use std::sync::Arc;
use crate::device_discovery::DeviceDriverScanner;
use crate::firmware::FirmwareBundle;
use crate::key_presenter::KeyPresenter;
use crate::keymap_editor::EditorProfile;

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
        vec![Box::new(crate::protocols::qmk_discovery::QmkScanner)]
    }
}
