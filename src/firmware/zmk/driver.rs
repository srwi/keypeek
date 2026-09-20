use super::codec as zmk_codec;
use super::rpc::{self as zmk_rpc, ZmkStudioSession, ZmkTransport};
use crate::key_spec::{KeySpec, KeymapSnapshot, LayerInfo};
use crate::layout::KeyboardDefinition;
use crate::protocols::{
    pump_hid_reader, DeviceError, DeviceEvent, KeyboardProtocol, RawHidTransport, Reopener,
    WriteSupport,
};
use std::error::Error;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use zmk_studio_api::ClientError;

const ZMK_USAGE_PAGE: u16 = 0xff60;

use super::common::{build_from_zmk_data, zmk_action_filter, ZmkLayout};

pub struct ZmkProtocol {
    hid_transport: Option<Box<dyn RawHidTransport>>,
    layout: Arc<ZmkLayout>,
    transport: ZmkTransport,
    session: Option<ZmkStudioSession>,
}

struct ZmkReopener {
    layout: Arc<ZmkLayout>,
    transport: ZmkTransport,
}

impl Reopener for ZmkReopener {
    fn reopen(&self) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
        Ok(Box::new(ZmkProtocol::open_hid(
            Arc::clone(&self.layout),
            self.transport.clone(),
        )?))
    }
}

impl ZmkProtocol {
    pub fn connect_live(vid: u16, pid: u16, transport: &ZmkTransport) -> Result<Self, DeviceError> {
        let zmk_data = zmk_rpc::fetch_zmk_data(transport)?;
        let layout = build_from_zmk_data(vid, pid, zmk_data)
            .map_err(|e| DeviceError::Protocol(e.to_string()))?;
        Self::open_hid(Arc::new(layout), transport.clone())
    }

    fn open_hid(layout: Arc<ZmkLayout>, transport: ZmkTransport) -> Result<Self, DeviceError> {
        let (vid, pid) = (layout.definition.vid, layout.definition.pid);
        let hid_transport = crate::platform::hid::wait_and_open_hid_transport(
            vid,
            pid,
            ZMK_USAGE_PAGE,
            Duration::from_secs(8),
        )?;

        Ok(Self::from_parts(layout, transport, Some(hid_transport)))
    }

    /// Constructs a ZmkProtocol instance from its components.
    pub(crate) fn from_parts(
        layout: Arc<ZmkLayout>,
        transport: ZmkTransport,
        hid_transport: Option<Box<dyn RawHidTransport>>,
    ) -> Self {
        Self {
            hid_transport,
            layout,
            transport,
            session: None,
        }
    }

    /// Executes an operation with an active ZMK studio session.
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

    fn update_cached_action(&self, layer_index: usize, row: usize, col: usize, action: KeySpec) {
        let mut snapshot = self.layout.snapshot.lock().unwrap();
        if let Some(cell) = snapshot
            .actions
            .get_mut(layer_index)
            .and_then(|layer| layer.get_mut(row))
            .and_then(|r| r.get_mut(col))
        {
            *cell = Some(action);
        }
    }
}

/// Returns true if the RPC error requires dropping the session.
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
        let mut hid_transport = self
            .hid_transport
            .take()
            .ok_or_else(|| DeviceError::Protocol("Already subscribed to ZMK events".to_string()))?;
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

        Ok(event_rx)
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

        // ZMK's matrix is 1×N: the column is the key position, and the write
        // RPC addresses layers by their stable id.
        self.with_session(|session| session.set_key(layer.id, col as i32, behavior))?;
        self.update_cached_action(layer_index, row, col, spec.clone());
        Ok(())
    }

    fn save_keymap(&mut self) -> Result<(), DeviceError> {
        self.with_session(|session| session.save())
    }

    fn acquire_edit_lock(&mut self) -> Result<(), DeviceError> {
        self.with_session(|_session| Ok(()))
    }

    fn release_edit_lock(&mut self) {
        self.session = None;
    }

    fn reopener(&self) -> Option<Arc<dyn Reopener>> {
        Some(Arc::new(ZmkReopener {
            layout: Arc::clone(&self.layout),
            transport: self.transport.clone(),
        }))
    }

    fn action_filter(&self) -> Option<crate::protocols::ActionFilter> {
        zmk_action_filter(
            self.layout.supported_behaviors.clone(),
            self.layout.behavior_metadata.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid_labels::Modifiers;
    use crate::key_spec::{CustomBinding, CustomKind, HidKey};
    use std::collections::HashSet;
    use std::sync::Mutex;
    use zmk_studio_api::BehaviorRole;

    impl ZmkLayout {
        fn mock(supported_behaviors: HashSet<BehaviorRole>) -> Arc<Self> {
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

        let layout = ZmkLayout::mock(supported_behaviors);
        let proto =
            ZmkProtocol::from_parts(layout, ZmkTransport::SerialPort("mock".to_string()), None);

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
        let layout = ZmkLayout::mock(HashSet::new());
        let mock_transport = crate::protocols::MockHidTransport::new();
        // Queue an input packet: 0xF1, row=2, col=3, pressed=true
        mock_transport.push_incoming(vec![0xF1, 2, 3, 1]);

        let mut proto = ZmkProtocol::from_parts(
            layout,
            ZmkTransport::SerialPort("mock".to_string()),
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
