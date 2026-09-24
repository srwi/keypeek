use super::codec as qmk_codec;
#[cfg(feature = "hidapi")]
use crate::protocols::pump_hid_reader;
use crate::protocols::ActionFilter;
#[cfg(any(feature = "hidapi", test))]
use crate::protocols::RawHidTransport;
use crate::protocols::{DeviceError, DeviceEvent, KeyboardProtocol, WriteSupport};
use keypeek_core::{KeySpec, KeyboardDefinition, KeymapSnapshot, LayerInfo};
#[cfg(feature = "hidapi")]
use qmk_via_api::api::KeyboardApi;
pub use qmk_via_api::QmkFeatures;
#[cfg(any(feature = "hidapi", test))]
use std::error::Error;
use std::sync::{mpsc, Arc, Mutex};
#[cfg(feature = "hidapi")]
use std::thread;
#[cfg(feature = "hidapi")]
use std::time::Duration;

#[allow(dead_code)]
const KEYPEEK_SUBSCRIBE_MARKER: u8 = 0xC0;
#[allow(dead_code)]
const KEYPEEK_SUBSCRIBE_ACTIVE: u8 = 0xA1;
#[allow(dead_code)]
const KEYPEEK_SUBSCRIBE_INACTIVE: u8 = 0xA0;

#[cfg(any(feature = "hidapi", test))]
trait SubscriptionSender: Send {
    fn set_active(&mut self, active: bool) -> Result<(), Box<dyn Error>>;
}

#[cfg(any(feature = "hidapi", test))]
struct RawHidSubscription {
    transport: Box<dyn RawHidTransport>,
}

#[cfg(any(feature = "hidapi", test))]
impl RawHidSubscription {
    #[cfg(feature = "desktop")]
    fn open(vid: u16, pid: u16) -> Result<Option<Box<dyn SubscriptionSender>>, DeviceError> {
        let transport =
            crate::platform::hid::open_hid_transport(vid, pid, 0xff60).map_err(|e| {
                DeviceError::Transport(format!(
                    "Could not open the RAW HID interface ({vid:04x}:{pid:04x}) to subscribe to \
                 layer events: {e}. The overlay cannot follow layer changes without it."
                ))
            })?;
        Ok(Some(Box::new(Self { transport })))
    }

    #[cfg(not(feature = "desktop"))]
    fn open(_vid: u16, _pid: u16) -> Result<Option<Box<dyn SubscriptionSender>>, DeviceError> {
        Ok(None)
    }
}

#[cfg(any(feature = "hidapi", test))]
impl SubscriptionSender for RawHidSubscription {
    fn set_active(&mut self, active: bool) -> Result<(), Box<dyn Error>> {
        let value = if active {
            KEYPEEK_SUBSCRIBE_ACTIVE
        } else {
            KEYPEEK_SUBSCRIBE_INACTIVE
        };
        self.transport
            .write_output_report(&[KEYPEEK_SUBSCRIBE_MARKER, value])
            .map_err(|e| format!("Subscription keepalive write error: {e}").into())
    }
}

#[cfg(feature = "hidapi")]
pub struct QmkSubscription {
    pub events: mpsc::Receiver<DeviceEvent>,
    pub keepalive: Option<mpsc::Sender<()>>,
}

/// Subscribes to live layer and key events from a QMK-based keyboard, managing the
/// keepalive heartbeat thread and raw HID reader loop internally.
#[cfg(feature = "hidapi")]
pub fn qmk_subscribe_events(
    api: Arc<Mutex<KeyboardApi>>,
    vid: u16,
    pid: u16,
) -> Result<QmkSubscription, DeviceError> {
    // 1. Start keepalive loop if subscription interface is available
    let keepalive = RawHidSubscription::open(vid, pid)?.map(|mut sender| {
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

pub trait QmkBackend: Send {
    fn set_key(
        &mut self,
        layer: usize,
        row: usize,
        col: usize,
        keycode: u16,
    ) -> Result<(), DeviceError>;
}

#[cfg(feature = "hidapi")]
struct NativeQmkBackend {
    api: Arc<Mutex<KeyboardApi>>,
}

#[cfg(feature = "hidapi")]
impl QmkBackend for NativeQmkBackend {
    fn set_key(
        &mut self,
        layer: usize,
        row: usize,
        col: usize,
        keycode: u16,
    ) -> Result<(), DeviceError> {
        let api = self.api.lock().unwrap();
        qmk_set_key_with_retry(&api, layer, row, col, keycode)
    }
}

#[cfg(target_arch = "wasm32")]
pub struct WebQmkBackend {
    transport: Arc<crate::platform::web_hid::WebHidTransport>,
    alive: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(target_arch = "wasm32")]
impl WebQmkBackend {
    pub fn new(
        transport: Arc<crate::platform::web_hid::WebHidTransport>,
        alive: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self { transport, alive }
    }
}

#[cfg(target_arch = "wasm32")]
impl QmkBackend for WebQmkBackend {
    fn set_key(
        &mut self,
        layer: usize,
        row: usize,
        col: usize,
        keycode: u16,
    ) -> Result<(), DeviceError> {
        let mut report = vec![0u8; 32];
        report[0] = 0x05; // VIA_CMD_SET_KEY
        report[1] = layer as u8;
        report[2] = row as u8;
        report[3] = col as u8;
        report[4] = (keycode >> 8) as u8;
        report[5] = (keycode & 0xFF) as u8;
        self.transport.fire_and_forget_report(&report);
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for WebQmkBackend {
    fn drop(&mut self) {
        self.alive
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let mut report = [0u8; crate::platform::web_hid::RAW_REPORT_SIZE];
        report[0] = KEYPEEK_SUBSCRIBE_MARKER;
        report[1] = KEYPEEK_SUBSCRIBE_INACTIVE;
        self.transport.fire_and_forget_report(&report);
    }
}

/// A connected QMK/VIA/Vial keyboard communicating over raw HID.
pub struct QmkProtocol {
    backend: Box<dyn QmkBackend>,
    definition: KeyboardDefinition,
    snapshot: Mutex<KeymapSnapshot>,
    features: QmkFeatures,
    event_rx: Mutex<Option<mpsc::Receiver<DeviceEvent>>>,
    _keepalive: Option<mpsc::Sender<()>>,
}

#[cfg(feature = "hidapi")]
impl QmkProtocol {
    pub fn new(api: KeyboardApi, definition: KeyboardDefinition, features: QmkFeatures) -> Self {
        let snapshot = qmk_read_snapshot(&api, &definition).unwrap_or_else(|_| KeymapSnapshot {
            layers: LayerInfo::indexed(4),
            actions: vec![vec![vec![None; definition.cols]; definition.rows]; 4],
        });
        let api = Arc::new(Mutex::new(api));
        let subscription =
            qmk_subscribe_events(Arc::clone(&api), definition.vid, definition.pid).ok();

        let (event_rx, keepalive) = match subscription {
            Some(sub) => (Some(sub.events), sub.keepalive),
            None => (None, None),
        };

        Self {
            backend: Box::new(NativeQmkBackend { api }),
            definition,
            snapshot: Mutex::new(snapshot),
            features,
            event_rx: Mutex::new(event_rx),
            _keepalive: keepalive,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl QmkProtocol {
    pub fn new_web(
        transport: Arc<crate::platform::web_hid::WebHidTransport>,
        definition: KeyboardDefinition,
        snapshot: KeymapSnapshot,
        features: QmkFeatures,
        event_rx: mpsc::Receiver<DeviceEvent>,
        alive: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            backend: Box::new(WebQmkBackend::new(transport, alive)),
            definition,
            snapshot: Mutex::new(snapshot),
            features,
            event_rx: Mutex::new(Some(event_rx)),
            _keepalive: None,
        }
    }
}

impl KeyboardProtocol for QmkProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn read_keymap(&self) -> Result<KeymapSnapshot, DeviceError> {
        Ok(self.snapshot.lock().unwrap().clone())
    }

    fn subscribe_events(&mut self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError> {
        self.event_rx
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| DeviceError::Protocol("Already subscribed to QMK events".to_string()))
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
        let code = qmk_codec::keyspec_to_qmk(spec)?;
        self.backend.set_key(layer_index, row, col, code)?;
        self.snapshot
            .lock()
            .unwrap()
            .set_action(layer_index, row, col, Some(spec.clone()));
        Ok(())
    }

    fn action_filter(&self) -> Option<crate::protocols::ActionFilter> {
        qmk_action_filter(self.features)
    }

    fn supports_live_layout_switching(&self) -> bool {
        true
    }
}

/// Returns an action filter that disables keycodes not supported by the keyboard's features.
pub fn qmk_action_filter(features: QmkFeatures) -> Option<ActionFilter> {
    Some(Arc::new(move |spec| {
        qmk_codec::keyspec_to_qmk(spec)
            .map(|code| features.is_keycode_supported(code))
            .unwrap_or(false)
    }))
}

/// Reads a complete keymap snapshot across all dynamic layers from a QMK/VIA/VIAL keyboard.
#[cfg(feature = "hidapi")]
pub fn qmk_read_snapshot(
    api: &KeyboardApi,
    definition: &KeyboardDefinition,
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
#[cfg(feature = "hidapi")]
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
#[cfg(feature = "hidapi")]
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

    #[test]
    fn test_raw_hid_subscription_keepalive() {
        let mock_transport = crate::protocols::MockHidTransport::new();
        let mut sub = RawHidSubscription {
            transport: Box::new(mock_transport.clone()),
        };

        sub.set_active(true).unwrap();
        assert_eq!(
            mock_transport.written_packets(),
            vec![vec![KEYPEEK_SUBSCRIBE_MARKER, KEYPEEK_SUBSCRIBE_ACTIVE]]
        );

        sub.set_active(false).unwrap();
        assert_eq!(
            mock_transport.written_packets(),
            vec![
                vec![KEYPEEK_SUBSCRIBE_MARKER, KEYPEEK_SUBSCRIBE_ACTIVE],
                vec![KEYPEEK_SUBSCRIBE_MARKER, KEYPEEK_SUBSCRIBE_INACTIVE],
            ]
        );
    }
}
