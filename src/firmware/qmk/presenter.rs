use crate::key_presenter::{resolve_custom_named_key, KeyPresenter, StandardKeyPresenter};
use crate::key_spec::{CustomKind, KeySpec};
use crate::layout_key::{behavior_names, Label, LayoutKey};

/// QMK-specific key presenter.
///
/// Formats QMK custom extensions (macros, tap dances, user keycodes) and provides
/// QMK-native presentation.
#[derive(Debug, Clone, Copy, Default)]
pub struct QmkKeyPresenter;

impl KeyPresenter for QmkKeyPresenter {
    fn present_key(&self, spec: &KeySpec, layer_names: &[String]) -> Option<LayoutKey> {
        match spec {
            KeySpec::Custom(binding) => {
                if let Some(display_name) = &binding.name {
                    return Some(resolve_custom_named_key(
                        display_name,
                        binding.param1,
                        binding.param2,
                        layer_names,
                    ));
                }
                match binding.kind {
                    CustomKind::TapDance => Some(LayoutKey {
                        tap: Label::new(binding.id.to_string()),
                        behavior: Some(behavior_names::TAP_DANCE.label()),
                        ..Default::default()
                    }),
                    CustomKind::Macro => Some(LayoutKey {
                        tap: Label::new(binding.id.to_string()),
                        behavior: Some(behavior_names::MACRO.label()),
                        ..Default::default()
                    }),
                    CustomKind::Keyboard => Some(LayoutKey {
                        tap: Label::new(binding.id.to_string()),
                        behavior: Some(behavior_names::CUSTOM_KB.label()),
                        ..Default::default()
                    }),
                    CustomKind::User => Some(LayoutKey {
                        tap: Label::new(binding.id.to_string()),
                        behavior: Some(behavior_names::CUSTOM_USER.label()),
                        ..Default::default()
                    }),
                    CustomKind::Raw => Some(LayoutKey {
                        tap: Label::new(format!("0x{:04X}", binding.id)),
                        ..Default::default()
                    }),
                }
            }
            _ => StandardKeyPresenter.present_key(spec, layer_names),
        }
    }
}
