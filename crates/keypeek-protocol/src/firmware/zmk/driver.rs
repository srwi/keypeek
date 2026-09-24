use super::codec as zmk_codec;
#[cfg(feature = "desktop")]
use super::rpc::{self as zmk_rpc, ZmkStudioSession, ZmkTransport};
#[cfg(feature = "desktop")]
use crate::protocols::pump_hid_reader;
#[cfg(test)]
use crate::protocols::RawHidTransport;
use crate::protocols::{DeviceError, DeviceEvent, KeyboardProtocol, Reopener, WriteSupport};
use keypeek_core::{KeySpec, KeyboardDefinition, KeymapSnapshot, LayerInfo};
#[cfg(feature = "desktop")]
use std::error::Error;
use std::sync::{mpsc, Arc, Mutex};
#[cfg(feature = "desktop")]
use std::time::Duration;
#[cfg(feature = "desktop")]
use zmk_studio_api::ClientError;

#[cfg(feature = "desktop")]
const ZMK_USAGE_PAGE: u16 = 0xff60;

#[cfg(feature = "desktop")]
use super::common::build_from_zmk_data;
use super::common::{zmk_action_filter, ZmkLayout};

pub trait ZmkBackend: Send {
    fn set_key(
        &mut self,
        layer_id: u32,
        key_position: i32,
        behavior: &zmk_studio_api::Behavior,
    ) -> Result<(), DeviceError>;

    fn save_changes(&mut self) -> Result<(), DeviceError>;

    fn acquire_edit_lock(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn release_edit_lock(&mut self) {}
}

#[cfg(feature = "desktop")]
pub struct NativeZmkBackend {
    transport: ZmkTransport,
    session: Option<ZmkStudioSession>,
}

#[cfg(feature = "desktop")]
impl NativeZmkBackend {
    pub fn new(transport: ZmkTransport) -> Self {
        Self {
            transport,
            session: None,
        }
    }

    fn with_session<T>(
        &mut self,
        write: impl FnOnce(&mut ZmkStudioSession) -> Result<T, Box<dyn Error>>,
    ) -> Result<T, DeviceError> {
        if self.session.is_none() {
            self.session = Some(ZmkStudioSession::open(&self.transport)?);
        }
        let result = write(self.session.as_mut().unwrap());
        if let Err(ref err) = result {
            if should_drop_session(err.as_ref()) {
                self.session = None;
            }
        }
        result.map_err(DeviceError::from)
    }
}

#[cfg(feature = "desktop")]
impl ZmkBackend for NativeZmkBackend {
    fn set_key(
        &mut self,
        layer_id: u32,
        key_position: i32,
        behavior: &zmk_studio_api::Behavior,
    ) -> Result<(), DeviceError> {
        self.with_session(|session| session.set_key(layer_id, key_position, behavior.clone()))
    }

    fn save_changes(&mut self) -> Result<(), DeviceError> {
        self.with_session(|session| session.save())
    }

    fn acquire_edit_lock(&mut self) -> Result<(), DeviceError> {
        self.with_session(|_session| Ok(()))
    }

    fn release_edit_lock(&mut self) {
        self.session = None;
    }
}

#[cfg(target_arch = "wasm32")]
pub struct WebZmkBackend {
    transport: Arc<crate::platform::web_serial::WebSerialTransport>,
    behavior_id_by_role: std::collections::HashMap<zmk_studio_api::BehaviorRole, u32>,
}

#[cfg(target_arch = "wasm32")]
impl WebZmkBackend {
    pub fn new(
        transport: Arc<crate::platform::web_serial::WebSerialTransport>,
        behavior_id_by_role: std::collections::HashMap<zmk_studio_api::BehaviorRole, u32>,
    ) -> Self {
        Self {
            transport,
            behavior_id_by_role,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl ZmkBackend for WebZmkBackend {
    fn set_key(
        &mut self,
        layer_id: u32,
        key_position: i32,
        behavior: &zmk_studio_api::Behavior,
    ) -> Result<(), DeviceError> {
        let behavior_id = match behavior.role() {
            Some(role) => self
                .behavior_id_by_role
                .get(&role)
                .copied()
                .ok_or_else(|| {
                    DeviceError::Unsupported(format!("Missing role {role:?} on device"))
                })?,
            None => match behavior {
                zmk_studio_api::Behavior::Custom { behavior_id, .. } => *behavior_id,
                zmk_studio_api::Behavior::Unknown { behavior_id, .. } => *behavior_id as u32,
                _ => {
                    return Err(DeviceError::Unsupported(
                        "Cannot resolve behavior ID for key binding".into(),
                    ))
                }
            },
        };

        let (param1, param2) = behavior.raw_params();
        let binding = zmk_studio_api::proto::zmk::keymap::BehaviorBinding {
            behavior_id: behavior_id as i32,
            param1,
            param2,
        };

        let transport = Arc::clone(&self.transport);
        wasm_bindgen_futures::spawn_local(async move {
            let req = zmk_studio_api::proto::zmk::studio::request::Subsystem::Keymap(
                zmk_studio_api::proto::zmk::keymap::Request {
                    request_type: Some(
                        zmk_studio_api::proto::zmk::keymap::request::RequestType::SetLayerBinding(
                            zmk_studio_api::proto::zmk::keymap::SetLayerBindingRequest {
                                layer_id,
                                key_position,
                                binding: Some(binding),
                            },
                        ),
                    ),
                },
            );
            if let Err(e) = transport.call(req).await {
                log::error!("Failed to write ZMK key binding: {e:?}");
            }
        });

        Ok(())
    }

    fn save_changes(&mut self) -> Result<(), DeviceError> {
        let transport = Arc::clone(&self.transport);
        wasm_bindgen_futures::spawn_local(async move {
            let req = zmk_studio_api::proto::zmk::studio::request::Subsystem::Keymap(
                zmk_studio_api::proto::zmk::keymap::Request {
                    request_type: Some(
                        zmk_studio_api::proto::zmk::keymap::request::RequestType::SaveChanges(true),
                    ),
                },
            );
            if let Err(e) = transport.call(req).await {
                log::error!("Failed to save ZMK changes: {e:?}");
            }
        });
        Ok(())
    }
}

pub struct ZmkProtocol {
    event_rx: Mutex<Option<mpsc::Receiver<DeviceEvent>>>,
    layout: Arc<ZmkLayout>,
    backend: Box<dyn ZmkBackend>,
    reopener: Option<Arc<dyn Reopener>>,
}

#[cfg(feature = "desktop")]
struct ZmkReopener {
    layout: Arc<ZmkLayout>,
    transport: ZmkTransport,
}

#[cfg(feature = "desktop")]
impl Reopener for ZmkReopener {
    fn reopen(&self) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
        Ok(Box::new(ZmkProtocol::open_hid(
            Arc::clone(&self.layout),
            self.transport.clone(),
        )?))
    }
}

impl ZmkProtocol {
    #[cfg(feature = "desktop")]
    pub fn connect_live(vid: u16, pid: u16, transport: &ZmkTransport) -> Result<Self, DeviceError> {
        let zmk_data = zmk_rpc::fetch_zmk_data(transport)?;
        let layout = build_from_zmk_data(vid, pid, zmk_data)
            .map_err(|e| DeviceError::Protocol(e.to_string()))?;
        Self::open_hid(Arc::new(layout), transport.clone())
    }

    #[cfg(feature = "desktop")]
    fn open_hid(layout: Arc<ZmkLayout>, transport: ZmkTransport) -> Result<Self, DeviceError> {
        let (vid, pid) = (layout.definition.vid, layout.definition.pid);
        let mut hid_transport = crate::platform::hid::wait_and_open_hid_transport(
            vid,
            pid,
            ZMK_USAGE_PAGE,
            Duration::from_secs(8),
        )?;

        let (event_tx, event_rx) = mpsc::channel();
        std::thread::spawn(move || {
            pump_hid_reader(
                || {
                    hid_transport
                        .read_input_report(Duration::from_millis(200))
                        .map_err(|e| e.to_string())
                },
                event_tx,
                "ZMK HID device disconnected",
            );
        });

        let backend = Box::new(NativeZmkBackend::new(transport.clone()));
        let reopener = Some(Arc::new(ZmkReopener {
            layout: Arc::clone(&layout),
            transport,
        }) as Arc<dyn Reopener>);

        Ok(Self {
            event_rx: Mutex::new(Some(event_rx)),
            layout,
            backend,
            reopener,
        })
    }

    /// Constructs a ZmkProtocol instance from its components (used in tests).
    #[cfg(all(test, feature = "desktop"))]
    pub(crate) fn from_parts(
        layout: Arc<ZmkLayout>,
        transport: ZmkTransport,
        hid_transport: Option<Box<dyn RawHidTransport>>,
    ) -> Self {
        let backend = Box::new(NativeZmkBackend::new(transport));
        let event_rx = hid_transport.map(|mut transport| {
            let (event_tx, event_rx) = mpsc::channel();
            std::thread::spawn(move || {
                pump_hid_reader(
                    || {
                        transport
                            .read_input_report(Duration::from_millis(200))
                            .map_err(|e| e.to_string())
                    },
                    event_tx,
                    "ZMK HID device disconnected",
                );
            });
            event_rx
        });
        Self {
            event_rx: Mutex::new(event_rx),
            layout,
            backend,
            reopener: None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new_web(
        layout: Arc<ZmkLayout>,
        backend: Box<dyn ZmkBackend>,
        event_rx: mpsc::Receiver<DeviceEvent>,
    ) -> Self {
        Self {
            event_rx: Mutex::new(Some(event_rx)),
            layout,
            backend,
            reopener: None,
        }
    }
}

/// Returns true if the RPC error requires dropping the session.
#[cfg(feature = "desktop")]
fn should_drop_session(err: &(dyn Error + 'static)) -> bool {
    let Some(client_err) = err.downcast_ref::<ClientError>() else {
        return true;
    };
    !matches!(
        client_err,
        ClientError::SetLayerBindingFailed(_)
            | ClientError::SaveChangesFailed(_)
            | ClientError::SetActivePhysicalLayoutFailed(_)
            | ClientError::MoveLayerFailed(_)
            | ClientError::AddLayerFailed(_)
            | ClientError::RemoveLayerFailed(_)
            | ClientError::RestoreLayerFailed(_)
            | ClientError::SetLayerPropsFailed(_)
            | ClientError::InvalidLayerOrPosition { .. }
            | ClientError::MissingBehaviorRole(_)
            | ClientError::BehaviorIdOutOfRange { .. }
    )
}

impl KeyboardProtocol for ZmkProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.layout.definition
    }

    fn read_keymap(&self) -> Result<KeymapSnapshot, DeviceError> {
        Ok(self.layout.snapshot.lock().unwrap().clone())
    }

    fn subscribe_events(&mut self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError> {
        self.event_rx
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| DeviceError::Protocol("Already subscribed to ZMK events".to_string()))
    }

    fn write_support(&self) -> WriteSupport {
        WriteSupport::Staged
    }

    fn set_key(
        &mut self,
        layer: &LayerInfo,
        layer_index: usize,
        row: usize,
        col: usize,
        spec: &KeySpec,
    ) -> Result<(), DeviceError> {
        let behavior = zmk_codec::keyspec_to_zmk(spec)?;
        if row != 0 {
            return Err(DeviceError::Unsupported(format!(
                "Invalid ZMK key position {row}:{col}"
            )));
        }

        // ZMK matrix is 1xN: column is key position, and write
        // RPC addresses layers by stable id.
        self.backend.set_key(layer.id, col as i32, &behavior)?;
        self.layout
            .snapshot
            .lock()
            .unwrap()
            .set_action(layer_index, row, col, Some(spec.clone()));
        Ok(())
    }

    fn save_keymap(&mut self) -> Result<(), DeviceError> {
        self.backend.save_changes()
    }

    fn acquire_edit_lock(&mut self) -> Result<(), DeviceError> {
        self.backend.acquire_edit_lock()
    }

    fn release_edit_lock(&mut self) {
        self.backend.release_edit_lock();
    }

    fn reopener(&self) -> Option<Arc<dyn Reopener>> {
        self.reopener.clone()
    }

    fn action_filter(&self) -> Option<crate::protocols::ActionFilter> {
        zmk_action_filter(
            self.layout.supported_behaviors.clone(),
            self.layout.behavior_metadata.clone(),
        )
    }

    fn supports_live_layout_switching(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keypeek_core::{CustomBinding, CustomKind, HidKey, Modifiers};
    use std::collections::HashSet;
    use std::sync::Mutex;
    use zmk_studio_api::BehaviorRole;

    impl ZmkLayout {
        fn test_new(supported_behaviors: HashSet<BehaviorRole>) -> Arc<Self> {
            Arc::new(Self {
                definition: KeyboardDefinition {
                    vid: 0x1234,
                    pid: 0x5678,
                    rows: 1,
                    cols: 1,
                    layouts: vec![],
                },
                snapshot: Mutex::new(KeymapSnapshot {
                    layers: vec![],
                    actions: vec![],
                }),
                supported_behaviors,
                behavior_metadata: Default::default(),
            })
        }
    }

    #[test]
    fn test_zmk_action_filter_rejects_unknown_custom_behaviors() {
        let mut supported_behaviors = HashSet::new();
        supported_behaviors.insert(BehaviorRole::KeyPress);
        supported_behaviors.insert(BehaviorRole::KeyToggle);

        let layout = ZmkLayout::test_new(supported_behaviors);
        let proto = ZmkProtocol::from_parts(
            layout,
            ZmkTransport::SerialPort("test_port".to_string()),
            None,
        );

        let filter = proto.action_filter().expect("filter should be present");

        // KeyPress is supported
        let key_press = KeySpec::KeyPress {
            key: HidKey::keyboard(0x04),
            modifiers: Modifiers::default(),
        };
        assert!(filter(&key_press));

        // QMK Raw / Audio / RGB Matrix custom code should be rejected
        let qmk_raw = KeySpec::Custom(CustomBinding {
            kind: CustomKind::Raw,
            id: 0x5C01, // e.g. QMK Audio / RGB matrix code
            name: None,
            param1: None,
            param2: None,
        });
        assert!(!filter(&qmk_raw));
    }

    #[test]
    fn test_zmk_subscribe_events_with_mock_transport() {
        let layout = ZmkLayout::test_new(HashSet::new());
        let mock_transport = crate::protocols::MockHidTransport::new();
        // Queue an input packet: 0xF1, row=2, col=3, pressed=true
        mock_transport.push_incoming(vec![0xF1, 2, 3, 1]);

        let mut proto = ZmkProtocol::from_parts(
            layout,
            ZmkTransport::SerialPort("test_port".to_string()),
            Some(Box::new(mock_transport)),
        );

        let rx = proto.subscribe_events().expect("subscribe should succeed");
        let event = rx
            .recv_timeout(Duration::from_millis(500))
            .expect("should receive event");
        assert_eq!(
            event,
            DeviceEvent::KeyPressed {
                row: 2,
                col: 3,
                pressed: true
            }
        );

        // Subscribing again should return an error as the transport has been taken
        assert!(proto.subscribe_events().is_err());
    }
}
