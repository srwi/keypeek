//! Pure domain aggregate models, key specifications, matrix, and layout geometry.

pub mod key_matrix;
pub mod key_spec;
pub mod layout;
pub mod visibility;

#[allow(unused_imports)]
pub use key_matrix::KeyMatrix;
#[allow(unused_imports)]
pub use key_spec::{
    BacklightAction, BluetoothAction, CustomBinding, CustomKind, CustomParam, HidKey, KeySpec,
    KeymapSnapshot, LayerActivation, LayerInfo, LightingAction, MouseAction, MouseButton,
    OutputTarget, PowerAction, RgbAction, RgbMatrixAction,
};
#[allow(unused_imports)]
pub use layout::{geometry, Key, KeyboardDefinition, KeyboardLayout};
#[allow(unused_imports)]
pub use visibility::{ActiveLayers, OverlayConfig, VisibilityStateMachine};
