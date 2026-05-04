use super::layout_geometry::flattened_top_left_after_center_rotation;
use super::zmk_rpc::{self, ZmkData, ZmkTransport};
use super::{Key, KeyboardDefinition, KeyboardLayout, KeyboardProtocol};
use crate::layout_key::LayoutKey;
use hidapi::{HidApi, HidDevice};
use std::error::Error;
use std::time::{Duration, Instant};

type LayerKeys3d = Vec<Vec<Vec<Option<LayoutKey>>>>;
const ZMK_USAGE_PAGE: u16 = 0xff60;

pub struct ZmkProtocol {
    hid_device: HidDevice,
    definition: KeyboardDefinition,
    layout_keys: LayerKeys3d,
    layer_count: usize,
}

impl ZmkProtocol {
    pub fn connect_live(
        vid: u16,
        pid: u16,
        transport: &ZmkTransport,
    ) -> Result<Self, Box<dyn Error>> {
        let zmk_data = zmk_rpc::fetch_zmk_data(transport)?;
        wait_for_hid_reappearance(vid, pid, ZMK_USAGE_PAGE, Duration::from_secs(8))
            .map_err(std::io::Error::other)?;
        let hid_device = open_zmk_hid(vid, pid).map_err(|e| {
            std::io::Error::other(format!(
                "Failed to connect HID ({vid:04x}:{pid:04x}) after reappearance: {e}"
            ))
        })?;
        let (definition, layout_keys, layer_count) = build_from_zmk_data(vid, pid, zmk_data)?;

        Ok(Self {
            hid_device,
            definition,
            layout_keys,
            layer_count,
        })
    }
}

fn open_zmk_hid(vid: u16, pid: u16) -> Result<HidDevice, String> {
    let api = HidApi::new().map_err(|e| format!("hidapi init failed: {e}"))?;
    // Unlike QMK/Vial, ZMK only needs a plain raw-HID reader here. The VIA helper sends an
    // eager GetProtocolVersion command on open, but ZMK raw-HID devices can emit async layer/key
    // notifications without implementing the VIA command/response handshake.
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
    while Instant::now() < deadline {
        let api = HidApi::new().map_err(|e| format!("hidapi init failed: {e}"))?;
        let present = api
            .device_list()
            .any(|d| d.vendor_id() == vid && d.product_id() == pid && d.usage_page() == usage_page);
        if present {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
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
        &self.definition
    }

    fn get_layer_count(&self) -> Result<usize, Box<dyn Error>> {
        Ok(self.layer_count)
    }

    fn read_all_keys(&self, _layers: usize, _rows: usize, _cols: usize) -> LayerKeys3d {
        self.layout_keys.clone()
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut buffer = vec![0; 32];
        self.hid_device
            .read(&mut buffer)
            .map_err(|e| format!("HID read error: {e}").into())
            .map(|_| buffer)
    }
}

fn build_from_zmk_data(
    vid: u16,
    pid: u16,
    data: ZmkData,
) -> Result<(KeyboardDefinition, LayerKeys3d, usize), Box<dyn Error>> {
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

            // Keypeek does not support rotated keys, so we recalculate the position adjusted for rotation around the pivot point.
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

    let layer_count = data.layer_count;
    let active_key_count = num_keys;
    let mut layout_keys_3d = Vec::with_capacity(layer_count);

    for layer_keys in &data.layout_keys {
        let mut row = vec![None; num_keys];

        for (pos, key) in layer_keys.iter().enumerate() {
            if pos >= active_key_count {
                break;
            }
            if pos < num_keys {
                row[pos] = key.clone();
            }
        }

        layout_keys_3d.push(vec![row]);
    }

    Ok((definition, layout_keys_3d, layer_count))
}
