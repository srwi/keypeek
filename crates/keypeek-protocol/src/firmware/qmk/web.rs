use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::platform::web_hid::WebHidTransport;
use crate::protocols::DeviceError;
use keypeek_core::{KeyboardDefinition, KeymapSnapshot, LayerInfo};

use super::codec as qmk_codec;
use super::kle_parser;
use qmk_via_api::QmkFeatures;

const KEYPEEK_SUBSCRIBE_MARKER: u8 = 0xC0;
const KEYPEEK_SUBSCRIBE_ACTIVE: u8 = 0xA1;
const KEYPEEK_SUBSCRIBE_INACTIVE: u8 = 0xA0;

const VIAL_PREFIX: u8 = 0xFE;
const VIAL_CMD_KEYBOARD_ID: u8 = 0x00;
const VIAL_CMD_SIZE: u8 = 0x01;
const VIAL_CMD_DEF: u8 = 0x02;

/// Minimum buffer length required for Vial keyboard ID response (4-byte version + 8-byte UID).
const VIAL_KEYBOARD_ID_RESP_LEN: usize = 12;
/// Maximum recognized Vial protocol version (Vial specification is currently at v9).
const VIAL_MAX_PROTOCOL_VERSION: u32 = 50;
/// QMK VIA unhandled command response marker (`id_unhandled`).
const VIA_UNHANDLED_MARKER: u8 = 0xFF;

const VIA_CMD_GET_PROTOCOL_VERSION: u8 = 0x01;
const VIA_CMD_GET_LAYER_COUNT: u8 = 0x11;
const VIA_CMD_GET_BUFFER: u8 = 0x12;

/// Checks whether the connected keyboard supports the Vial protocol.
///
/// Follows the Vial specification handshake: queries `VIAL_CMD_KEYBOARD_ID` (`0xFE, 0x00`).
/// Legitimate Vial firmware returns a 4-byte protocol version and an 8-byte hardware UID.
/// Standard QMK VIA keyboards reject unhandled commands with `0xFF` or echo the command byte `0xFE`.
async fn probe_vial(transport: &WebHidTransport) -> bool {
    let Ok(resp) = transport
        .command_timeout(VIAL_PREFIX, &[VIAL_CMD_KEYBOARD_ID], 250)
        .await
    else {
        return false;
    };

    if resp.len() < VIAL_KEYBOARD_ID_RESP_LEN {
        return false;
    }

    // Echoed or rejected unhandled commands from QMK VIA start with 0xFE or 0xFF
    if resp[0] == VIAL_PREFIX || resp[0] == VIA_UNHANDLED_MARKER {
        return false;
    }

    let version = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
    if version == 0 || version > VIAL_MAX_PROTOCOL_VERSION {
        return false;
    }

    // Real Vial devices return an 8-byte unique ID (not all zeroes)
    if resp[4..12] == [0u8; 8] {
        return false;
    }

    true
}

/// Fetches and parses the compressed Vial keyboard layout definition directly from keyboard storage.
async fn fetch_vial_definition(
    transport: &WebHidTransport,
    vid: u16,
    pid: u16,
) -> Result<KeyboardDefinition, DeviceError> {
    let size_resp = transport.command(VIAL_PREFIX, &[VIAL_CMD_SIZE]).await?;
    if size_resp.len() < 4 {
        return Err(DeviceError::Protocol(
            "Invalid Vial definition size response".into(),
        ));
    }
    let size =
        u32::from_le_bytes([size_resp[0], size_resp[1], size_resp[2], size_resp[3]]) as usize;
    if size == 0 {
        return Err(DeviceError::Protocol("Vial definition size is 0".into()));
    }

    let mut compressed = Vec::with_capacity(size);
    let mut block: u32 = 0;
    while compressed.len() < size {
        let mut payload = vec![VIAL_CMD_DEF];
        payload.extend_from_slice(&block.to_le_bytes());
        let resp = transport.command(VIAL_PREFIX, &payload).await?;
        let remaining = size - compressed.len();
        let chunk_size = remaining.min(32);
        if resp.len() < chunk_size {
            return Err(DeviceError::Protocol(
                "Short block response reading Vial definition".into(),
            ));
        }
        compressed.extend_from_slice(&resp[..chunk_size]);
        block += 1;
    }

    let mut decompressed = Vec::new();
    let mut cursor = std::io::Cursor::new(&compressed);
    lzma_rs::xz_decompress(&mut cursor, &mut decompressed)
        .map_err(|e| DeviceError::Protocol(format!("Failed to decompress Vial definition: {e}")))?;

    let json_str = String::from_utf8(decompressed)
        .map_err(|e| DeviceError::Protocol(format!("Vial definition is not valid UTF-8: {e}")))?;

    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| DeviceError::Protocol(format!("Failed to parse Vial definition JSON: {e}")))?;

    kle_parser::parse_vial_definition(&json, vid, pid)
        .map_err(|e| DeviceError::Protocol(format!("Failed to parse KLE definition: {e}")))
}

/// Probes the VIA protocol version.
async fn probe_via_version(transport: &WebHidTransport) -> Result<u16, DeviceError> {
    let resp = transport.command(VIA_CMD_GET_PROTOCOL_VERSION, &[]).await?;
    if resp.len() >= 3 {
        let version = ((resp[1] as u16) << 8) | (resp[2] as u16);
        Ok(version)
    } else {
        Err(DeviceError::Protocol(
            "Invalid VIA protocol version response".into(),
        ))
    }
}

/// Queries the number of layers from the keyboard over WebHID.
async fn get_layer_count(transport: &WebHidTransport) -> usize {
    match transport.command(VIA_CMD_GET_LAYER_COUNT, &[]).await {
        Ok(resp) if resp.len() >= 2 => resp[1].max(1) as usize,
        _ => 4,
    }
}

/// Reads the complete keymap matrix for all layers over WebHID.
async fn read_keymap_snapshot(
    transport: &WebHidTransport,
    definition: &KeyboardDefinition,
    layer_count: usize,
) -> Result<KeymapSnapshot, DeviceError> {
    let (rows, cols) = (definition.rows, definition.cols);
    let total_keys = rows * cols;
    let total_bytes = total_keys * 2;
    const CHUNK_SIZE: usize = 28;

    let mut actions = vec![vec![vec![None; cols]; rows]; layer_count];

    for (layer, layer_actions) in actions.iter_mut().enumerate() {
        let mut raw_bytes = Vec::with_capacity(total_bytes);
        let layer_offset = layer * total_bytes;

        for chunk_start in (0..total_bytes).step_by(CHUNK_SIZE) {
            let chunk_len = (total_bytes - chunk_start).min(CHUNK_SIZE) as u8;
            let offset = (layer_offset + chunk_start) as u16;
            let hi = (offset >> 8) as u8;
            let lo = (offset & 0xFF) as u8;

            let resp = transport
                .command(VIA_CMD_GET_BUFFER, &[hi, lo, chunk_len])
                .await?;

            if resp.len() >= 4 + chunk_len as usize {
                raw_bytes.extend_from_slice(&resp[4..4 + chunk_len as usize]);
            }
        }

        let mut keycodes = Vec::with_capacity(total_keys);
        for pair in raw_bytes.chunks_exact(2) {
            keycodes.push(u16::from_be_bytes([pair[0], pair[1]]));
        }

        for (i, &keycode) in keycodes.iter().enumerate() {
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

fn keypeek_subscribe_report(active: bool) -> [u8; crate::platform::web_hid::RAW_REPORT_SIZE] {
    let mut report = [0u8; crate::platform::web_hid::RAW_REPORT_SIZE];
    report[0] = KEYPEEK_SUBSCRIBE_MARKER;
    report[1] = if active {
        KEYPEEK_SUBSCRIBE_ACTIVE
    } else {
        KEYPEEK_SUBSCRIBE_INACTIVE
    };
    report
}

pub use super::common::QmkProtocol;

pub use super::json_parser::parse_layout_json_str;

/// Retrieves cached layout JSON for a given VID/PID from browser localStorage.
pub fn get_cached_layout(vid: u16, pid: u16) -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage
        .get_item(&format!("via_layout_{vid:04x}_{pid:04x}"))
        .ok()?
}

/// Persists layout JSON for a given VID/PID to browser localStorage.
pub fn save_cached_layout(vid: u16, pid: u16, content: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(&format!("via_layout_{vid:04x}_{pid:04x}"), content);
        }
    }
}

/// Loads a VIA layout definition from provided JSON or localStorage.
async fn load_via_definition(
    transport: &WebHidTransport,
    vid: u16,
    pid: u16,
    layout_json: Option<&str>,
) -> Result<Option<KeyboardDefinition>, DeviceError> {
    let version = probe_via_version(transport).await?;
    if version < 9 {
        return Err(DeviceError::Unsupported(format!(
            "Unsupported VIA protocol version: {version}. Minimum required version is 9."
        )));
    }

    if let Some(json_content) = layout_json {
        let parsed = parse_layout_json_str(json_content, vid, pid)?;
        save_cached_layout(vid, pid, json_content);
        Ok(Some(parsed))
    } else if let Some(cached) = get_cached_layout(vid, pid) {
        let parsed = parse_layout_json_str(&cached, vid, pid)?;
        Ok(Some(parsed))
    } else {
        Ok(None)
    }
}

/// Result of connecting to a WebHID QMK/Vial/VIA device.
pub enum WebQmkOutcome {
    Connected(QmkProtocol),
    RequiresLayoutFile,
}

/// Probes protocol capabilities, loads the keyboard layout definition and keymap snapshot,
/// and returns a live `WebQmkOutcome`.
pub async fn connect_web_qmk(
    transport: Arc<WebHidTransport>,
    vid: u16,
    pid: u16,
    layout_json: Option<&str>,
) -> Result<WebQmkOutcome, DeviceError> {
    let is_vial = probe_vial(&transport).await;

    let definition = if is_vial {
        match fetch_vial_definition(&transport, vid, pid).await {
            Ok(def) => Some(def),
            Err(e) => {
                log::warn!("Failed to fetch Vial definition, falling back to VIA: {e}");
                load_via_definition(&transport, vid, pid, layout_json).await?
            }
        }
    } else {
        load_via_definition(&transport, vid, pid, layout_json).await?
    };

    let Some(definition) = definition else {
        return Ok(WebQmkOutcome::RequiresLayoutFile);
    };

    let layer_count = get_layer_count(&transport).await;
    let snapshot = read_keymap_snapshot(&transport, &definition, layer_count).await?;
    let features = QmkFeatures::default();
    let alive = Arc::new(AtomicBool::new(true));

    let alive_clone = Arc::clone(&alive);
    let transport_clone = Arc::clone(&transport);
    wasm_bindgen_futures::spawn_local(async move {
        let report = keypeek_subscribe_report(true);
        while alive_clone.load(Ordering::Relaxed) {
            transport_clone.fire_and_forget_report(&report);
            crate::platform::web_hid::sleep_ms(1000).await;
        }
    });

    let event_rx = transport.take_event_receiver()?;
    let protocol = QmkProtocol::new_web(transport, definition, snapshot, features, event_rx, alive);

    Ok(WebQmkOutcome::Connected(protocol))
}
