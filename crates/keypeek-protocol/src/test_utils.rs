//! Shared testing utilities, in-memory keyboard protocols, and test domain fixtures.

use std::sync::mpsc;
use std::sync::Arc;

use crate::application::Keyboard;
use keypeek_core::{
    HidKey, Key, KeyMatrix, KeySpec, KeyboardDefinition, KeyboardDomain, KeyboardLayout,
    KeymapSnapshot, LayerActivation, LayerInfo, Modifiers, OverlayConfig,
};
use crate::firmware::qmk::QmkEditorProfile;
use crate::key_presenter::StandardKeyPresenter;
use crate::protocols::{ConnectionSpec, DeviceError, DeviceEvent, KeyboardProtocol, WriteSupport};
use crate::ui_wake::UiWake;

/// A lightweight in-memory `KeyboardProtocol` for unit tests.
pub struct TestProtocol {
    pub definition: KeyboardDefinition,
    pub snapshot: KeymapSnapshot,
    pub write_support: WriteSupport,
}

impl Default for TestProtocol {
    fn default() -> Self {
        Self {
            definition: default_test_definition(),
            snapshot: default_test_snapshot(),
            write_support: WriteSupport::None,
        }
    }
}

impl TestProtocol {
    pub fn new(definition: KeyboardDefinition, snapshot: KeymapSnapshot) -> Self {
        Self {
            definition,
            snapshot,
            write_support: WriteSupport::None,
        }
    }

    pub fn with_write_support(mut self, write_support: WriteSupport) -> Self {
        self.write_support = write_support;
        self
    }
}

impl KeyboardProtocol for TestProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn read_keymap(&self) -> Result<KeymapSnapshot, DeviceError> {
        Ok(self.snapshot.clone())
    }

    fn subscribe_events(&mut self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError> {
        let (_tx, rx) = mpsc::channel();
        Ok(rx)
    }

    fn write_support(&self) -> WriteSupport {
        self.write_support
    }
}

/// Constructs a default 4x4 test keyboard definition with two layouts.
pub fn default_test_definition() -> KeyboardDefinition {
    KeyboardDefinition {
        vid: 0x1234,
        pid: 0x5678,
        rows: 4,
        cols: 4,
        layouts: vec![
            KeyboardLayout {
                name: "Default".to_string(),
                keys: vec![
                    Key {
                        row: 0,
                        col: 1,
                        x: 1.0,
                        y: 0.0,
                        w: 1.0,
                        h: 1.0,
                        r: 0.0,
                    },
                    Key {
                        row: 3,
                        col: 3,
                        x: 3.0,
                        y: 3.0,
                        w: 1.0,
                        h: 1.0,
                        r: 0.0,
                    },
                ],
            },
            KeyboardLayout {
                name: "Alternative".to_string(),
                keys: vec![],
            },
        ],
    }
}

/// Constructs a default test keymap snapshot with Base and Nav layers.
pub fn default_test_snapshot() -> KeymapSnapshot {
    let mut row0 = vec![None; 4];
    row0[1] = Some(KeySpec::KeyPress {
        key: HidKey::keyboard(0x14), // 'Q'
        modifiers: Modifiers::default(),
    });
    let mut row3 = vec![None; 4];
    row3[3] = Some(KeySpec::Layer {
        layer: 1,
        activation: LayerActivation::Momentary,
    });

    KeymapSnapshot {
        layers: vec![
            LayerInfo {
                id: 0,
                name: Some("Base".to_string()),
            },
            LayerInfo {
                id: 1,
                name: Some("Nav".to_string()),
            },
        ],
        actions: vec![vec![row0, vec![None; 4], vec![None; 4], row3]],
    }
}

/// Constructs a test `Keyboard` instance initialized with an in-memory `TestProtocol`.
pub fn create_test_keyboard() -> Keyboard {
    let protocol = Box::new(TestProtocol::default());
    Keyboard::new(
        protocol,
        "Default".to_string(),
        OverlayConfig {
            timeout_ms: 2000,
            activation_delay_ms: 300,
            visible_layers: u32::MAX,
        },
        UiWake::new(Arc::new(|| ())),
        Arc::new(QmkEditorProfile),
    )
    .unwrap()
}

/// Constructs a test `KeyboardDomain`.
pub fn create_test_domain() -> KeyboardDomain {
    let definition = default_test_definition();
    let layout = definition.get_layout("Default").unwrap();
    let matrix = KeyMatrix::from_snapshot(default_test_snapshot(), 4, 4, &StandardKeyPresenter);
    let config = OverlayConfig {
        timeout_ms: 2000,
        activation_delay_ms: 300,
        visible_layers: u32::MAX,
    };
    KeyboardDomain::new(definition, "Default".to_string(), layout, matrix, config)
}

/// Constructs a dummy `ConnectionSpec` for unit tests.
pub fn test_spec() -> ConnectionSpec {
    ConnectionSpec::Vial {
        vid: 0x1234,
        pid: 0x5678,
    }
}
