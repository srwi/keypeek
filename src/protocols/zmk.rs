use super::layout_geometry::flattened_top_left_after_center_rotation;
use super::zmk_rpc::{self, ZmkData, ZmkStudioSession, ZmkTransport};
use super::{Key, KeyboardDefinition, KeyboardLayout, KeyboardProtocol, Reopener, WriteSupport};
use crate::key_action::{KeyAction, KeymapSnapshot, LayerInfo};
use hidapi::{HidApi, HidDevice};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use zmk_studio_api::ResolvedLayer;

const ZMK_USAGE_PAGE: u16 = 0xff60;

struct ZmkLayout {
    definition: KeyboardDefinition,
    /// Shared with the reopener so a reconnected protocol stays truthful
    /// after writes; mutated in place by the write methods.
    snapshot: Mutex<KeymapSnapshot>,
}

pub struct ZmkProtocol {
    hid_device: HidDevice,
    layout: Arc<ZmkLayout>,
    transport: ZmkTransport,
    session: Option<ZmkStudioSession>,
}

struct ZmkReopener {
    layout: Arc<ZmkLayout>,
    transport: ZmkTransport,
}

impl Reopener for ZmkReopener {
    fn reopen(&self) -> Result<Box<dyn KeyboardProtocol>, Box<dyn Error>> {
        Ok(Box::new(ZmkProtocol::open_hid(
            Arc::clone(&self.layout),
            self.transport.clone(),
        )?))
    }
}

impl ZmkProtocol {
    pub fn connect_live(
        vid: u16,
        pid: u16,
        transport: &ZmkTransport,
    ) -> Result<Self, Box<dyn Error>> {
        let zmk_data = zmk_rpc::fetch_zmk_data(transport)?;
        let layout = build_from_zmk_data(vid, pid, zmk_data)?;
        Self::open_hid(Arc::new(layout), transport.clone())
    }

    fn open_hid(layout: Arc<ZmkLayout>, transport: ZmkTransport) -> Result<Self, Box<dyn Error>> {
        let (vid, pid) = (layout.definition.vid, layout.definition.pid);
        wait_for_hid_reappearance(vid, pid, ZMK_USAGE_PAGE, Duration::from_secs(8))
            .map_err(std::io::Error::other)?;
        let hid_device = open_zmk_hid(vid, pid).map_err(|e| {
            std::io::Error::other(format!(
                "Failed to connect HID ({vid:04x}:{pid:04x}) after reappearance: {e}"
            ))
        })?;

        Ok(Self {
            hid_device,
            layout,
            transport,
            session: None,
        })
    }

    /// Runs `write` against the edit session, opening it lazily on first use
    /// (BLE opens are slow). Any failed operation drops the session so the
    /// next write starts from a fresh connection — a device that locked or
    /// drifted mid-session is handled by reconnecting.
    fn with_session<T>(
        &mut self,
        write: impl FnOnce(&mut ZmkStudioSession) -> Result<T, Box<dyn Error>>,
    ) -> Result<T, Box<dyn Error>> {
        if self.session.is_none() {
            self.session = Some(ZmkStudioSession::open(&self.transport)?);
        }
        let result = write(self.session.as_mut().unwrap());
        if result.is_err() {
            self.session = None;
        }
        result
    }

    fn update_cached_action(&self, layer_index: usize, row: usize, col: usize, action: KeyAction) {
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

    fn read_keymap(&self) -> Result<KeymapSnapshot, Box<dyn Error>> {
        Ok(self.layout.snapshot.lock().unwrap().clone())
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut buffer = [0u8; 32];
        let read = self
            .hid_device
            .read_timeout(&mut buffer, 200)
            .map_err(|e| format!("HID read error: {e}"))?;
        Ok(buffer[..read].to_vec())
    }

    fn write_support(&self) -> WriteSupport {
        WriteSupport::Session
    }

    fn set_key(
        &mut self,
        layer: &LayerInfo,
        layer_index: usize,
        row: usize,
        col: usize,
        action: &KeyAction,
    ) -> Result<(), Box<dyn Error>> {
        let behavior = match action {
            KeyAction::Zmk(behavior) => behavior.clone(),
            KeyAction::Qmk(_) => return Err("Cannot apply a QMK keycode to a ZMK keyboard".into()),
        };
        if row != 0 {
            return Err(format!("Invalid ZMK key position {row}:{col}").into());
        }

        // ZMK's matrix is 1×N: the column is the key position, and the write
        // RPC addresses layers by their stable id.
        self.with_session(|session| session.set_key(layer.id, col as i32, behavior))?;
        self.update_cached_action(layer_index, row, col, action.clone());
        Ok(())
    }

    fn save_keymap(&mut self) -> Result<(), Box<dyn Error>> {
        self.with_session(|session| session.save())
    }

    fn discard_keymap(&mut self) -> Result<KeymapSnapshot, Box<dyn Error>> {
        let resolved = self.with_session(|session| session.discard())?;
        let snapshot = snapshot_from_resolved(&resolved, self.layout.definition.cols);
        *self.layout.snapshot.lock().unwrap() = snapshot.clone();
        Ok(snapshot)
    }

    fn end_edit_session(&mut self) {
        self.session = None;
    }

    fn reopener(&self) -> Option<Arc<dyn Reopener>> {
        Some(Arc::new(ZmkReopener {
            layout: Arc::clone(&self.layout),
            transport: self.transport.clone(),
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
            let mut row: Vec<Option<KeyAction>> = layer
                .bindings
                .iter()
                .map(|b| Some(KeyAction::Zmk(b.clone())))
                .collect();
            row.resize(num_keys, None);
            vec![row]
        })
        .collect();

    KeymapSnapshot { layers, actions }
}
