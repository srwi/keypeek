use std::sync::Arc;

use crate::device_discovery::DeviceDriverScanner;
use crate::firmware::qmk::{QmkEditorProfile, QmkKeyPresenter};
use crate::firmware::FirmwareBundle;
use crate::key_presenter::KeyPresenter;
use crate::keymap_editor::EditorProfile;

pub struct MockBundle;

impl FirmwareBundle for MockBundle {
    fn driver_id(&self) -> &'static str {
        "mock"
    }

    fn create_presenter(&self) -> Arc<dyn KeyPresenter> {
        Arc::new(QmkKeyPresenter)
    }

    fn create_profile(&self) -> Arc<dyn EditorProfile> {
        Arc::new(QmkEditorProfile)
    }

    fn scanners(&self) -> Vec<Box<dyn DeviceDriverScanner>> {
        if cfg!(debug_assertions) {
            vec![Box::new(crate::protocols::mock::MockScanner)]
        } else {
            Vec::new()
        }
    }
}
