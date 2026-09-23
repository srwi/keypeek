use crate::key_presenter::{resolve_custom_named_key, StandardKeyPresenter};
use keypeek_core::{KeyPresenter, KeySpec, LayoutKey};

/// ZMK-specific key presenter.
///
/// Formats ZMK custom behaviors (with behavior name initials and parameter extraction)
/// and provides ZMK-native presentation.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZmkKeyPresenter;

impl KeyPresenter for ZmkKeyPresenter {
    fn present_key(&self, spec: &KeySpec, layer_names: &[String]) -> Option<LayoutKey> {
        match spec {
            KeySpec::Custom(binding) => {
                if let Some(display_name) = &binding.name {
                    Some(resolve_custom_named_key(
                        display_name,
                        binding.param1,
                        binding.param2,
                        layer_names,
                    ))
                } else {
                    StandardKeyPresenter.present_key(spec, layer_names)
                }
            }
            _ => StandardKeyPresenter.present_key(spec, layer_names),
        }
    }
}
