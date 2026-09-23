//! Key presenter trait converting [`KeySpec`] domain objects into visual [`LayoutKey`] items.

use crate::key_spec::KeySpec;
use crate::layout_key::LayoutKey;

/// Converts a [`KeySpec`] into a visual [`LayoutKey`].
pub trait KeyPresenter: Send + Sync {
    fn present_key(&self, spec: &KeySpec, layer_names: &[String]) -> Option<LayoutKey>;
}
