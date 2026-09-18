//! Pure domain aggregate models, key specifications, matrix, and layout geometry.

pub mod key_matrix;
pub mod key_spec;
pub mod keyboard;
pub mod layout;
pub mod visibility;

pub use key_matrix::KeyMatrix;
pub use key_spec::{
    BacklightAction, BluetoothAction, CustomBinding, CustomKind, CustomParam, HidKey, KeySpec,
    KeymapSnapshot, LayerActivation, LayerInfo, LightingAction, MouseAction, MouseButton,
    OutputTarget, PowerAction, RgbAction, RgbMatrixAction,
};
pub use keyboard::KeyboardDomain;
pub use layout::{geometry, Key, KeyboardDefinition, KeyboardLayout};
pub use visibility::{ActiveLayers, OverlayConfig, VisibilityStateMachine};
