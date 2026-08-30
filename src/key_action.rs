use crate::layout_key::LayoutKey;
use zmk_studio_api::Behavior;

/// The firmware-level assignment of one key on one layer. This is what the
/// device stores; `LayoutKey` labels are derived from it.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    /// A VIA/Vial dynamic-keymap keycode (protocol v12 encoding).
    Qmk(u16),
    /// A resolved ZMK binding.
    Zmk(Behavior),
}

impl KeyAction {
    /// Derive the display label. `None` = transparent (falls through to lower layers).
    pub fn resolve_label(&self, layer_names: &[String]) -> Option<LayoutKey> {
        match self {
            KeyAction::Qmk(code) => match crate::qmk_keycode_labels::resolve_qmk_key(*code) {
                crate::qmk_keycode_labels::KeyResolution::Transparent => None,
                crate::qmk_keycode_labels::KeyResolution::Key(key) => Some(key),
                crate::qmk_keycode_labels::KeyResolution::Unknown => {
                    Some(crate::qmk_keycode_labels::get_hex_layout_key(*code))
                }
            },
            KeyAction::Zmk(b) => crate::zmk_keycode_labels::behavior_to_layout_key(b, layer_names),
        }
    }
}

/// Identity of one layer as the device reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerInfo {
    /// Stable ZMK layer id, used by ZMK Studio write RPCs. Equals the index for QMK/mock.
    pub id: u32,
    /// ZMK layer name. `None` for QMK (display falls back to "Layer {index}").
    pub name: Option<String>,
}

impl LayerInfo {
    /// Layers identified by index alone, as QMK/Vial/mock report them.
    pub fn indexed(count: usize) -> Vec<Self> {
        (0..count as u32)
            .map(|id| Self { id, name: None })
            .collect()
    }
}

/// Everything the protocol knows about the keymap, actions included.
#[derive(Clone)]
pub struct KeymapSnapshot {
    pub layers: Vec<LayerInfo>,
    /// `[layer][row][col]`. `None` = no binding at this position (padding).
    pub actions: Vec<Vec<Vec<Option<KeyAction>>>>,
}
