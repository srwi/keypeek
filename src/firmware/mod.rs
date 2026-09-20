pub mod qmk;
pub mod zmk;

use std::sync::Arc;

use crate::device_discovery::DeviceDriverScanner;
use crate::key_presenter::KeyPresenter;
use crate::keymap_editor::EditorProfile;
use crate::protocols::{ConnectionSpec, DeviceError, KeyboardProtocol};

/// Cohesive bundle providing discovery scanners, visual presenter, editor profile,
/// and hardware protocol connection for a firmware family.
pub trait FirmwareBundle: Send + Sync {
    fn create_presenter(&self) -> Arc<dyn KeyPresenter>;
    fn create_profile(&self) -> Arc<dyn EditorProfile>;
    fn scanners(&self) -> Vec<Box<dyn DeviceDriverScanner>>;
    fn connect(&self, spec: &ConnectionSpec) -> Result<Box<dyn KeyboardProtocol>, DeviceError>;
}

pub static QMK_BUNDLE: qmk::QmkBundle = qmk::QmkBundle;
pub static ZMK_BUNDLE: zmk::ZmkBundle = zmk::ZmkBundle;

/// Resolves the appropriate firmware bundle for a connection specification.
pub fn bundle_for_spec(spec: &ConnectionSpec) -> &'static dyn FirmwareBundle {
    match spec {
        ConnectionSpec::Via { .. } | ConnectionSpec::Vial { .. } => &QMK_BUNDLE,
        ConnectionSpec::Zmk { .. } => &ZMK_BUNDLE,
    }
}

pub static ALL_BUNDLES: [&'static dyn FirmwareBundle; 2] = [&ZMK_BUNDLE, &QMK_BUNDLE];

/// Returns all registered firmware bundles.
pub fn all_bundles() -> &'static [&'static dyn FirmwareBundle] {
    &ALL_BUNDLES
}

/// Aggregates hardware scanners from all registered firmware bundles.
pub fn default_scanners() -> Vec<Box<dyn DeviceDriverScanner>> {
    all_bundles().iter().flat_map(|b| b.scanners()).collect()
}

/// Establishes a connection to the specified hardware device through its firmware bundle.
pub fn connect_protocol(spec: &ConnectionSpec) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
    bundle_for_spec(spec).connect(spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_order_prioritizes_zmk_over_qmk() {
        let bundles = all_bundles();
        let zmk_idx = bundles
            .iter()
            .position(|b| b.create_profile().name() == "ZMK");
        let qmk_idx = bundles
            .iter()
            .position(|b| b.create_profile().name() == "QMK");
        assert!(
            matches!((zmk_idx, qmk_idx), (Some(z), Some(q)) if z < q),
            "ZMK scanner must precede QMK scanner to prevent double-detection"
        );
    }
}
