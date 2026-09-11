use crate::key_spec::{KeySpec, KeymapSnapshot, LayerInfo};
use crate::protocols::qmk_codec;
use crate::protocols::{
    pump_hid_reader, DeviceError, DeviceEvent, KeyboardDefinition, KeyboardProtocol, WriteSupport,
};
use qmk_via_api::api::KeyboardApi;
pub use qmk_via_api::QmkFeatures;
use std::error::Error;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

const KEYPEEK_SUBSCRIBE_MARKER: u8 = 0xC0;
const KEYPEEK_SUBSCRIBE_ACTIVE: u8 = 0xA1;
const KEYPEEK_SUBSCRIBE_INACTIVE: u8 = 0xA0;

trait SubscriptionSender: Send {
    fn set_active(&self, active: bool) -> Result<(), Box<dyn Error>>;
}

struct RawHidSubscription {
    api: KeyboardApi,
}

impl RawHidSubscription {
    fn open(vid: u16, pid: u16) -> Result<Option<Box<dyn SubscriptionSender>>, DeviceError> {
        let api = KeyboardApi::new(vid, pid, 0xff60, None).map_err(|e| {
            DeviceError::Transport(format!(
                "Could not open the RAW HID interface ({vid:04x}:{pid:04x}) to subscribe to \
                 layer events: {e}. The overlay cannot follow layer changes without it."
            ))
        })?;
        Ok(Some(Box::new(Self { api })))
    }
}

impl SubscriptionSender for RawHidSubscription {
    fn set_active(&self, active: bool) -> Result<(), Box<dyn Error>> {
        let value = if active {
            KEYPEEK_SUBSCRIBE_ACTIVE
        } else {
            KEYPEEK_SUBSCRIBE_INACTIVE
        };
        self.api
            .hid_send(vec![KEYPEEK_SUBSCRIBE_MARKER, value])
            .map_err(|e| format!("Subscription keepalive write error: {e}").into())
    }
}

pub struct QmkSubscription {
    pub events: mpsc::Receiver<DeviceEvent>,
    pub keepalive: Option<mpsc::Sender<()>>,
}

/// Subscribes to live layer and key events from a QMK-based keyboard, managing the
/// keepalive heartbeat thread and raw HID reader loop internally.
pub fn qmk_subscribe_events(
    api: Arc<Mutex<KeyboardApi>>,
    vid: u16,
    pid: u16,
) -> Result<QmkSubscription, DeviceError> {
    // 1. Start keepalive loop if subscription interface is available
    let keepalive = RawHidSubscription::open(vid, pid)?.map(|sender| {
        let (tx, rx) = mpsc::channel::<()>();
        thread::spawn(move || {
            loop {
                let _ = sender.set_active(true);
                match rx.recv_timeout(Duration::from_millis(1000)) {
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    _ => break,
                }
            }
            let _ = sender.set_active(false);
        });
        tx
    });

    // 2. Spawn event reader thread using pump_hid_reader
    let (event_tx, event_rx) = mpsc::channel();
    thread::spawn(move || {
        pump_hid_reader(
            || {
                let api_guard = match api.lock() {
                    Ok(g) => g,
                    Err(_) => return Err("poisoned mutex".to_string()),
                };
                api_guard.hid_read().map(Some).map_err(|e| e.to_string())
            },
            event_tx,
            "QMK device disconnected",
        );
    });

    Ok(QmkSubscription {
        events: event_rx,
        keepalive,
    })
}

/// A connected QMK/VIA/Vial keyboard communicating over raw HID.
pub struct QmkProtocol {
    api: Arc<Mutex<KeyboardApi>>,
    definition: KeyboardDefinition,
    features: QmkFeatures,
    _keepalive: Option<mpsc::Sender<()>>,
}

impl QmkProtocol {
    pub fn new(api: KeyboardApi, definition: KeyboardDefinition, features: QmkFeatures) -> Self {
        Self {
            api: Arc::new(Mutex::new(api)),
            definition,
            features,
            _keepalive: None,
        }
    }
}

impl KeyboardProtocol for QmkProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn read_keymap(&self) -> Result<KeymapSnapshot, DeviceError> {
        qmk_read_snapshot(&self.api.lock().unwrap(), &self.definition)
    }

    fn subscribe_events(&mut self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError> {
        let subscription = qmk_subscribe_events(
            Arc::clone(&self.api),
            self.definition.vid,
            self.definition.pid,
        )?;
        self._keepalive = subscription.keepalive;
        Ok(subscription.events)
    }

    fn write_support(&self) -> WriteSupport {
        WriteSupport::Immediate
    }

    fn set_key(
        &mut self,
        _layer: &LayerInfo,
        layer_index: usize,
        row: usize,
        col: usize,
        spec: &KeySpec,
    ) -> Result<(), DeviceError> {
        qmk_set_key(&self.api.lock().unwrap(), layer_index, row, col, spec)
    }

    fn action_filter(&self) -> Option<crate::protocols::ActionFilter> {
        qmk_action_filter(self.features)
    }

    fn supports_raw_keycode_entry(&self) -> bool {
        true
    }

    fn supports_live_layout_switching(&self) -> bool {
        true
    }

    fn terminology(&self) -> super::Terminology {
        super::Terminology::Qmk
    }

    fn parse_raw_keycode(&self, code: u16) -> Option<KeySpec> {
        Some(qmk_codec::qmk_to_keyspec(code))
    }
}

/// Returns an action filter that disables keycodes not supported by the keyboard's features.
pub fn qmk_action_filter(features: QmkFeatures) -> Option<super::ActionFilter> {
    Some(Arc::new(move |spec| {
        qmk_codec::keyspec_to_qmk(spec)
            .map(|code| features.is_keycode_supported(code))
            .unwrap_or(false)
    }))
}

/// Reads a complete keymap snapshot across all dynamic layers from a QMK/VIA/VIAL keyboard.
pub fn qmk_read_snapshot(
    api: &KeyboardApi,
    definition: &super::KeyboardDefinition,
) -> Result<KeymapSnapshot, DeviceError> {
    let layer_count = api
        .get_layer_count()
        .map_err(|e| DeviceError::Protocol(format!("Failed to get layer count: {e}")))?
        as usize;
    let (rows, cols) = (definition.rows, definition.cols);
    let matrix_info = qmk_via_api::api::MatrixInfo {
        rows: rows as u8,
        cols: cols as u8,
    };

    let mut actions = vec![vec![vec![None; cols]; rows]; layer_count];
    for (layer, layer_actions) in actions.iter_mut().enumerate() {
        let raw_matrix = api.read_raw_matrix(matrix_info, layer as u8).map_err(|e| {
            DeviceError::Protocol(format!("Failed to read layer {layer} keymap: {e}"))
        })?;
        for (i, &keycode) in raw_matrix.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            if row < rows && col < cols {
                layer_actions[row][col] = Some(qmk_codec::qmk_to_keyspec(keycode));
            }
        }
    }

    Ok(KeymapSnapshot {
        layers: LayerInfo::indexed(layer_count),
        actions,
    })
}

/// Writes a key binding to a QMK keyboard.
pub fn qmk_set_key(
    api: &KeyboardApi,
    layer_index: usize,
    row: usize,
    col: usize,
    spec: &KeySpec,
) -> Result<(), DeviceError> {
    let code = qmk_codec::keyspec_to_qmk(spec)?;
    qmk_set_key_with_retry(api, layer_index, row, col, code)
}

/// Writes a keycode via the VIA protocol with readback verification on error.
pub(crate) fn qmk_set_key_with_retry(
    api: &KeyboardApi,
    layer_index: usize,
    row: usize,
    col: usize,
    keycode: u16,
) -> Result<(), DeviceError> {
    let max_retries = 3;
    for attempt in 0..max_retries {
        match api.set_key(layer_index as u8, row as u8, col as u8, keycode) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if attempt == max_retries - 1 {
                    // Check if write actually took effect despite the error
                    if let Ok(actual) = api.get_key(layer_index as u8, row as u8, col as u8) {
                        if actual == keycode {
                            return Ok(());
                        }
                    }
                    return Err(DeviceError::Protocol(format!(
                        "Failed to write key ({e}); write did not take effect"
                    )));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use qmk_via_api::keycodes::Keycode;

    #[test]
    fn test_qmk_action_filter() {
        let features = QmkFeatures {
            has_backlight: false,
            has_rgblight: true,
            has_rgb_matrix: false,
            has_audio: false,
        };

        let filter = qmk_action_filter(features).expect("filter should be Some");
        assert!(filter(&qmk_codec::qmk_to_keyspec(Keycode::KC_A as u16)));
        assert!(filter(&qmk_codec::qmk_to_keyspec(
            Keycode::QK_UNDERGLOW_TOGGLE as u16
        )));
        assert!(!filter(&qmk_codec::qmk_to_keyspec(
            Keycode::QK_BACKLIGHT_TOGGLE as u16
        )));
        assert!(!filter(&qmk_codec::qmk_to_keyspec(
            Keycode::QK_RGB_MATRIX_TOGGLE as u16
        )));
        assert!(!filter(&qmk_codec::qmk_to_keyspec(
            Keycode::QK_AUDIO_TOGGLE as u16
        )));
    }
}
