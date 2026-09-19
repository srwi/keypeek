use super::codec as zmk_codec;
use super::rpc::{self as zmk_rpc, ZmkData, ZmkStudioSession, ZmkTransport};
use crate::key_spec::{KeySpec, KeymapSnapshot, LayerInfo};
use crate::layout::geometry::flattened_top_left_after_center_rotation;
use crate::layout::{Key, KeyboardDefinition, KeyboardLayout};
use crate::protocols::{
    pump_hid_reader, DeviceError, DeviceEvent, KeyboardProtocol, Reopener, WriteSupport,
};
use hidapi::{HidApi, HidDevice};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use web_time::Instant;
use zmk_studio_api::{BehaviorBindingParametersSet, BehaviorRole, ClientError, ResolvedLayer};

const ZMK_USAGE_PAGE: u16 = 0xff60;

struct ZmkLayout {
    definition: KeyboardDefinition,
    snapshot: Mutex<KeymapSnapshot>,
    supported_behaviors: HashSet<BehaviorRole>,
    behavior_metadata: HashMap<BehaviorRole, Vec<BehaviorBindingParametersSet>>,
}

pub struct ZmkProtocol {
    hid_device: Option<HidDevice>,
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
        wait_for_hid_reappearance(vid, pid, ZMK_USAGE_PAGE, Duration::from_secs(8))
            .map_err(DeviceError::Transport)?;
        let hid_device = open_zmk_hid(vid, pid).map_err(|e| {
            DeviceError::Transport(format!(
                "Failed to connect HID ({vid:04x}:{pid:04x}) after reappearance: {e}"
            ))
        })?;

        Ok(Self {
            hid_device: Some(hid_device),
            layout,
            transport,
            session: None,
        })
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

fn open_zmk_hid(vid: u16, pid: u16) -> Result<HidDevice, String> {
    let api = HidApi::new().map_err(|e| format!("hidapi init failed: {e}"))?;
    let path = api
        .device_list()
        .find(|device| {
            device.vendor_id() == vid
                && device.product_id() == pid
                && device.usage_page() == ZMK_USAGE_PAGE
        })
        .map(|device| device.path().to_owned())
        .ok_or_else(|| {
            format!(
                "could not find HID interface for {:04x}:{:04x} usage 0x{:04x}",
                vid, pid, ZMK_USAGE_PAGE
            )
        })?;

    api.open_path(&path).map_err(|e| e.to_string())
}

fn wait_for_hid_reappearance(
    vid: u16,
    pid: u16,
    usage_page: u16,
    timeout: Duration,
) -> Result<(), String> {
    // On Linux BLE, the HID node can temporarily disappear while HoG/GATT activity settles; wait
    // for the matching HID interface to reappear before reconnecting via hidapi.
    let deadline = Instant::now() + timeout;
    let mut device_present_without_usage = false;
    while Instant::now() < deadline {
        let api = HidApi::new().map_err(|e| format!("hidapi init failed: {e}"))?;
        let mut matched = false;
        for d in api.device_list() {
            if d.vendor_id() == vid && d.product_id() == pid {
                if d.usage_page() == usage_page {
                    matched = true;
                    break;
                }
                device_present_without_usage = true;
            }
        }
        if matched {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    if device_present_without_usage {
        return Err("Please re-pair the keyboard to refresh the HID descriptor.".to_string());
    }

    Err(format!(
        "HID interface did not reappear in {} ms for {:04x}:{:04x} usage 0x{:04x}",
        timeout.as_millis(),
        vid,
        pid,
        usage_page
    ))
}

impl KeyboardProtocol for ZmkProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.layout.definition
    }

    fn read_keymap(&self) -> Result<KeymapSnapshot, DeviceError> {
        Ok(self.layout.snapshot.lock().unwrap().clone())
    }

    fn subscribe_events(&mut self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError> {
        let hid_device = self
            .hid_device
            .take()
            .ok_or_else(|| DeviceError::Protocol("Already subscribed to ZMK events".to_string()))?;
        let (event_tx, event_rx) = mpsc::channel();

        std::thread::spawn(move || {
            let mut buffer = [0u8; 32];
            pump_hid_reader(
                || match hid_device.read_timeout(&mut buffer, 200) {
                    Ok(read) if read > 0 => Ok(Some(buffer[..read].to_vec())),
                    Ok(_) => Ok(None),
                    Err(e) => Err(e.to_string()),
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
        let supported = self.layout.supported_behaviors.clone();
        let metadata = self.layout.behavior_metadata.clone();
        Some(Arc::new(move |spec| {
            match zmk_codec::keyspec_to_zmk(spec) {
                Ok(behavior) => match behavior.role() {
                    Some(role) => {
                        if !supported.is_empty() && !supported.contains(&role) {
                            return false;
                        }
                        metadata
                            .get(&role)
                            .is_none_or(|sets| behavior.matches_metadata(sets))
                    }
                    None => false,
                },
                Err(_) => false,
            }
        }))
    }
}

fn build_from_zmk_data(vid: u16, pid: u16, data: ZmkData) -> Result<ZmkLayout, Box<dyn Error>> {
    const ACTIVE_LAYOUT_NAME: &str = "active physical layout";

    let active_idx = data.physical_layouts.active_layout_index as usize;
    let proto_layouts = &data.physical_layouts.layouts;

    if proto_layouts.is_empty() {
        return Err("Device has no physical layouts".into());
    }

    let active_layout = proto_layouts
        .get(active_idx)
        .ok_or_else(|| format!("Invalid active layout index: {active_idx}"))?;
    let active_keys: Vec<Key> = active_layout
        .keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let w = k.width as f32 / 100.0;
            let h = k.height as f32 / 100.0;

            let x = k.x as f32 / 100.0;
            let y = k.y as f32 / 100.0;

            // Position is where the key's center lands after rotating around the pivot;
            // the rotation itself is applied at render time via `r`.
            let angle_deg = k.r as f32 / 100.0;
            let pivot_x = if k.rx == 0 { k.x } else { k.rx } as f32 / 100.0;
            let pivot_y = if k.ry == 0 { k.y } else { k.ry } as f32 / 100.0;
            let (x, y) =
                flattened_top_left_after_center_rotation(x, y, w, h, angle_deg, pivot_x, pivot_y);

            Key {
                row: 0,
                col: i,
                x,
                y,
                w,
                h,
                r: angle_deg,
            }
        })
        .collect();
    let num_keys = active_keys.len();

    let definition = KeyboardDefinition {
        vid,
        pid,
        rows: 1,
        cols: num_keys,
        layouts: vec![KeyboardLayout {
            name: ACTIVE_LAYOUT_NAME.to_string(),
            keys: active_keys,
        }],
    };

    let snapshot = snapshot_from_resolved(&data.resolved_layers, num_keys);

    Ok(ZmkLayout {
        definition,
        snapshot: Mutex::new(snapshot),
        supported_behaviors: data.supported_behaviors,
        behavior_metadata: data.behavior_metadata,
    })
}

/// Builds a keymap snapshot from the device's resolved layers. Bindings beyond
/// `num_keys` are dropped and short layers are padded with `None`, matching the
/// matrix dimensions.
fn snapshot_from_resolved(resolved: &[ResolvedLayer], num_keys: usize) -> KeymapSnapshot {
    let layers = resolved
        .iter()
        .map(|layer| LayerInfo {
            id: layer.id,
            name: (!layer.name.is_empty()).then(|| layer.name.clone()),
        })
        .collect();

    let actions = resolved
        .iter()
        .map(|layer| {
            let mut row: Vec<Option<KeySpec>> = layer
                .bindings
                .iter()
                .map(|b| Some(zmk_codec::zmk_to_keyspec(b)))
                .collect();
            row.resize(num_keys, None);
            vec![row]
        })
        .collect();

    KeymapSnapshot { layers, actions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid_labels::Modifiers;
    use crate::key_spec::{CustomBinding, CustomKind, HidKey};
    use std::collections::HashSet;
    use zmk_studio_api::BehaviorRole;

    #[test]
    fn test_zmk_action_filter_rejects_unknown_custom_behaviors() {
        let mut supported_behaviors = HashSet::new();
        supported_behaviors.insert(BehaviorRole::KeyPress);
        supported_behaviors.insert(BehaviorRole::KeyToggle);

        let layout = Arc::new(ZmkLayout {
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
        });

        let proto = ZmkProtocol {
            hid_device: None,
            layout,
            transport: ZmkTransport::SerialPort("mock".to_string()),
            session: None,
        };

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
}
