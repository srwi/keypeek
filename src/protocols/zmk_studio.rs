//! ZMK Studio protocol client.
//!
//! Communicates with ZMK firmware over USB CDC-ACM serial using the ZMK Studio
//! protocol: protobuf messages wrapped in SOF/ESC/EOF byte-stuffing framing.
//!
//! This module provides:
//! - Message framing (encode/decode)
//! - RPC request/response over serial
//! - Physical layout + keymap retrieval
//! - Behavior binding → LayoutKey conversion

use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use crate::protocols::zmk_proto::{behaviors, core, keymap, meta, studio};
use prost::Message;
use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Write};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Framing constants (matches zmk-main/app/src/studio/msg_framing.h)
// ---------------------------------------------------------------------------
const SOF: u8 = 0xAB;
const ESC: u8 = 0xAC;
const EOF: u8 = 0xAD;

// ---------------------------------------------------------------------------
// Framing helpers
// ---------------------------------------------------------------------------

/// Encode a raw protobuf payload into a framed message: SOF <escaped bytes> EOF.
fn frame_encode(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 8);
    out.push(SOF);
    for &b in payload {
        if b == SOF || b == ESC || b == EOF {
            out.push(ESC);
        }
        out.push(b);
    }
    out.push(EOF);
    out
}

/// Deframe state machine — accumulates bytes until a complete frame is found.
struct Deframer {
    buf: Vec<u8>,
    in_frame: bool,
    escaped: bool,
}

impl Deframer {
    fn new() -> Self {
        Self {
            buf: Vec::with_capacity(512),
            in_frame: false,
            escaped: false,
        }
    }

    /// Feed bytes and return the first complete frame payload, if any.
    fn feed(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        for &b in data {
            if self.escaped {
                self.escaped = false;
                if self.in_frame {
                    self.buf.push(b);
                }
                continue;
            }
            match b {
                SOF => {
                    self.buf.clear();
                    self.in_frame = true;
                    self.escaped = false;
                }
                ESC => {
                    self.escaped = true;
                }
                EOF => {
                    if self.in_frame && !self.buf.is_empty() {
                        self.in_frame = false;
                        return Some(std::mem::take(&mut self.buf));
                    }
                    self.in_frame = false;
                    self.buf.clear();
                }
                _ => {
                    if self.in_frame {
                        self.buf.push(b);
                    }
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// RPC Client
// ---------------------------------------------------------------------------

/// A ZMK Studio RPC client over a serial port.
pub struct StudioRpcClient {
    port: Box<dyn serialport::SerialPort>,
    next_request_id: u32,
}

impl StudioRpcClient {
    /// Open a serial port and create a new RPC client.
    pub fn open(port_name: &str) -> Result<Self, Box<dyn Error>> {
        let port = serialport::new(port_name, 115_200)
            .timeout(Duration::from_secs(5))
            .open()
            .map_err(|e| format!("Failed to open serial port '{port_name}': {e}"))?;
        Ok(Self {
            port,
            next_request_id: 1,
        })
    }

    /// Send a top-level Studio Request and receive the matching RequestResponse.
    fn rpc_call(
        &mut self,
        subsystem: studio::request::Subsystem,
    ) -> Result<studio::RequestResponse, Box<dyn Error>> {
        let req_id = self.next_request_id;
        self.next_request_id += 1;

        let request = studio::Request {
            request_id: req_id,
            subsystem: Some(subsystem),
        };

        // Encode and frame
        let payload = request.encode_to_vec();
        let frame = frame_encode(&payload);
        self.port.write_all(&frame)?;
        self.port.flush()?;

        // Read response — loop until we get a RequestResponse with our request_id
        let mut deframer = Deframer::new();
        let mut read_buf = [0u8; 256];

        loop {
            let n = self.port.read(&mut read_buf)?;
            if let Some(frame_data) = deframer.feed(&read_buf[..n]) {
                let response = studio::Response::decode(frame_data.as_slice())
                    .map_err(|e| format!("Failed to decode Studio response: {e}"))?;

                match response.r#type {
                    Some(studio::response::Type::RequestResponse(rr)) => {
                        if rr.request_id == req_id {
                            // Check for meta error
                            if let Some(studio::request_response::Subsystem::Meta(meta_resp)) =
                                &rr.subsystem
                            {
                                if let Some(meta::response::ResponseType::SimpleError(code)) =
                                    &meta_resp.response_type
                                {
                                    let err = meta::ErrorConditions::try_from(*code)
                                        .unwrap_or(meta::ErrorConditions::Generic);
                                    return Err(format!("ZMK Studio error: {err:?}").into());
                                }
                            }
                            return Ok(rr);
                        }
                        // Wrong request_id — ignore, keep reading
                    }
                    Some(studio::response::Type::Notification(_)) => {
                        // Ignore notifications, keep reading
                    }
                    None => {}
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Core subsystem
    // -----------------------------------------------------------------------

    /// Get the lock state of the device. This is an unsecured RPC.
    pub fn get_lock_state(&mut self) -> Result<core::LockState, Box<dyn Error>> {
        let rr = self.rpc_call(studio::request::Subsystem::Core(core::Request {
            request_type: Some(core::request::RequestType::GetLockState(true)),
        }))?;
        match rr.subsystem {
            Some(studio::request_response::Subsystem::Core(resp)) => match resp.response_type {
                Some(core::response::ResponseType::GetLockState(state)) => {
                    core::LockState::try_from(state)
                        .map_err(|_| format!("Unknown lock state: {state}").into())
                }
                _ => Err("Unexpected core response type".into()),
            },
            _ => Err("Unexpected subsystem in response".into()),
        }
    }

    /// Get device info (name, serial). Unsecured.
    #[allow(dead_code)]
    pub fn get_device_info(&mut self) -> Result<core::GetDeviceInfoResponse, Box<dyn Error>> {
        let rr = self.rpc_call(studio::request::Subsystem::Core(core::Request {
            request_type: Some(core::request::RequestType::GetDeviceInfo(true)),
        }))?;
        match rr.subsystem {
            Some(studio::request_response::Subsystem::Core(resp)) => match resp.response_type {
                Some(core::response::ResponseType::GetDeviceInfo(info)) => Ok(info),
                _ => Err("Unexpected core response type".into()),
            },
            _ => Err("Unexpected subsystem in response".into()),
        }
    }

    // -----------------------------------------------------------------------
    // Keymap subsystem (all secured — device must be unlocked)
    // -----------------------------------------------------------------------

    /// Get all physical layouts from the device.
    pub fn get_physical_layouts(&mut self) -> Result<keymap::PhysicalLayouts, Box<dyn Error>> {
        let rr = self.rpc_call(studio::request::Subsystem::Keymap(keymap::Request {
            request_type: Some(keymap::request::RequestType::GetPhysicalLayouts(true)),
        }))?;
        match rr.subsystem {
            Some(studio::request_response::Subsystem::Keymap(resp)) => match resp.response_type {
                Some(keymap::response::ResponseType::GetPhysicalLayouts(layouts)) => Ok(layouts),
                _ => Err("Unexpected keymap response type".into()),
            },
            _ => Err("Unexpected subsystem in response".into()),
        }
    }

    /// Get the full keymap (all layers with bindings).
    pub fn get_keymap(&mut self) -> Result<keymap::Keymap, Box<dyn Error>> {
        let rr = self.rpc_call(studio::request::Subsystem::Keymap(keymap::Request {
            request_type: Some(keymap::request::RequestType::GetKeymap(true)),
        }))?;
        match rr.subsystem {
            Some(studio::request_response::Subsystem::Keymap(resp)) => match resp.response_type {
                Some(keymap::response::ResponseType::GetKeymap(km)) => Ok(km),
                _ => Err("Unexpected keymap response type".into()),
            },
            _ => Err("Unexpected subsystem in response".into()),
        }
    }

    // -----------------------------------------------------------------------
    // Behaviors subsystem (secured)
    // -----------------------------------------------------------------------

    /// List all behavior IDs.
    pub fn list_all_behaviors(&mut self) -> Result<Vec<u32>, Box<dyn Error>> {
        let rr = self.rpc_call(studio::request::Subsystem::Behaviors(behaviors::Request {
            request_type: Some(behaviors::request::RequestType::ListAllBehaviors(true)),
        }))?;
        match rr.subsystem {
            Some(studio::request_response::Subsystem::Behaviors(resp)) => {
                match resp.response_type {
                    Some(behaviors::response::ResponseType::ListAllBehaviors(list)) => {
                        Ok(list.behaviors)
                    }
                    _ => Err("Unexpected behaviors response type".into()),
                }
            }
            _ => Err("Unexpected subsystem in response".into()),
        }
    }

    /// Get details for a specific behavior.
    pub fn get_behavior_details(
        &mut self,
        behavior_id: u32,
    ) -> Result<behaviors::GetBehaviorDetailsResponse, Box<dyn Error>> {
        let rr = self.rpc_call(studio::request::Subsystem::Behaviors(behaviors::Request {
            request_type: Some(behaviors::request::RequestType::GetBehaviorDetails(
                behaviors::GetBehaviorDetailsRequest { behavior_id },
            )),
        }))?;
        match rr.subsystem {
            Some(studio::request_response::Subsystem::Behaviors(resp)) => {
                match resp.response_type {
                    Some(behaviors::response::ResponseType::GetBehaviorDetails(details)) => {
                        Ok(details)
                    }
                    _ => Err("Unexpected behaviors response type".into()),
                }
            }
            _ => Err("Unexpected subsystem in response".into()),
        }
    }

    /// Fetch all behavior details into a map of behavior_id → details.
    pub fn fetch_all_behavior_details(
        &mut self,
    ) -> Result<HashMap<u32, behaviors::GetBehaviorDetailsResponse>, Box<dyn Error>> {
        let ids = self.list_all_behaviors()?;
        let mut map = HashMap::with_capacity(ids.len());
        for id in ids {
            let details = self.get_behavior_details(id)?;
            map.insert(id, details);
        }
        Ok(map)
    }
}

// ---------------------------------------------------------------------------
// Serial port auto-detection
// ---------------------------------------------------------------------------

/// Information about a detected ZMK Studio serial port.
pub struct ZmkSerialDevice {
    pub port_name: String,
    pub vid: u16,
    pub pid: u16,
    pub product: Option<String>,
}

/// Scan for USB serial ports and return those that respond to ZMK Studio
/// `get_device_info` (or, as a lighter heuristic, all CDC-ACM ports).
/// For now we return all USB serial ports so the user can pick. The caller
/// can optionally filter by VID/PID.
pub fn scan_serial_ports() -> Vec<ZmkSerialDevice> {
    let Ok(ports) = serialport::available_ports() else {
        return Vec::new();
    };

    ports
        .into_iter()
        .filter_map(|p| {
            if let serialport::SerialPortType::UsbPort(usb) = &p.port_type {
                Some(ZmkSerialDevice {
                    port_name: p.port_name,
                    vid: usb.vid,
                    pid: usb.pid,
                    product: usb.product.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Behavior binding → LayoutKey conversion
// ---------------------------------------------------------------------------

/// Cached behavior metadata used for converting bindings to display keys.
#[derive(Clone)]
pub struct BehaviorMap {
    /// behavior_id → (display_name, param1_type, param2_type)
    entries: HashMap<u32, BehaviorInfo>,
}

#[derive(Clone, Debug)]
struct BehaviorInfo {
    display_name: String,
    param1_type: ParamType,
    param2_type: ParamType,
}

#[derive(Clone, Debug, PartialEq)]
enum ParamType {
    None,
    HidUsage,
    LayerId,
    Constant,
    Range,
    Unknown,
}

impl BehaviorMap {
    /// Build from the raw behavior details fetched from the device.
    pub fn from_details(details: &HashMap<u32, behaviors::GetBehaviorDetailsResponse>) -> Self {
        let mut entries = HashMap::new();
        for (id, detail) in details {
            let (p1_type, p2_type) = if let Some(set) = detail.metadata.first() {
                (
                    classify_param(&set.param1),
                    classify_param(&set.param2),
                )
            } else {
                (ParamType::None, ParamType::None)
            };

            entries.insert(
                *id,
                BehaviorInfo {
                    display_name: detail.display_name.clone(),
                    param1_type: p1_type,
                    param2_type: p2_type,
                },
            );
        }
        Self { entries }
    }

    /// Convert a ZMK behavior binding to a LayoutKey.
    ///
    /// Returns `None` for transparent bindings (behavior_id == 0 typically
    /// means `&trans` if the firmware encodes it that way, but we also check
    /// display_name).
    pub fn binding_to_layout_key(&self, binding: &keymap::BehaviorBinding) -> Option<LayoutKey> {
        let behavior_id = binding.behavior_id;

        // behavior_id 0 with all-zero params is typically transparent
        if behavior_id == 0 && binding.param1 == 0 && binding.param2 == 0 {
            // Check if it's actually "Transparent" or "None" behavior
            if let Some(info) = self.entries.get(&0) {
                let name_lower = info.display_name.to_lowercase();
                if name_lower.contains("trans") {
                    return None; // transparent → None
                }
                if name_lower.contains("none") {
                    return Some(LayoutKey {
                        tap: Label::new(""),
                        ..Default::default()
                    });
                }
            }
            // If no behavior 0 info, treat as transparent
            return None;
        }

        let Some(info) = self.entries.get(&(behavior_id as u32)) else {
            // Unknown behavior — show as hex
            return Some(LayoutKey {
                tap: Label::new(format!("?0x{:X}", behavior_id)),
                ..Default::default()
            });
        };

        let name_lower = info.display_name.to_lowercase();

        // Handle transparent behavior (can appear with any behavior_id)
        if name_lower.contains("trans") {
            return None;
        }

        // Handle "None" / disabled key
        if name_lower == "none" || name_lower == "disabled" {
            return Some(LayoutKey {
                tap: Label::new(""),
                ..Default::default()
            });
        }

        // --- Single-param behaviors ---

        // Pure HID keypress (e.g., &kp A)
        if info.param2_type == ParamType::None && info.param1_type == ParamType::HidUsage {
            return Some(hid_usage_to_layout_key(binding.param1));
        }

        // Layer activate (e.g., &mo 1, &tog 1)
        if info.param2_type == ParamType::None && info.param1_type == ParamType::LayerId {
            let layer_num = binding.param1 as u8;
            let abbrev = layer_abbreviation(&info.display_name);
            return Some(LayoutKey {
                tap: Label::with_short(
                    format!("{} {}", abbrev, layer_num),
                    format!("{}{}", abbrev, layer_num),
                ),
                kind: KeycodeKind::Special,
                layer_ref: Some(layer_num),
                ..Default::default()
            });
        }

        // --- Two-param behaviors (hold-tap family) ---

        // param1 = layer, param2 = HID usage → layer-tap (e.g., &lt 1 A)
        if info.param1_type == ParamType::LayerId && info.param2_type == ParamType::HidUsage {
            let layer_num = binding.param1 as u8;
            let tap_key = hid_usage_to_layout_key(binding.param2);
            return Some(LayoutKey {
                tap: tap_key.tap,
                hold: Some(Label::with_short(
                    format!("L{}", layer_num),
                    format!("L{}", layer_num),
                )),
                symbol: tap_key.symbol,
                kind: KeycodeKind::Special,
                layer_ref: Some(layer_num),
            });
        }

        // param1 = HID usage (modifier), param2 = HID usage → mod-tap (e.g., &mt LSFT A)
        if info.param1_type == ParamType::HidUsage && info.param2_type == ParamType::HidUsage {
            let hold_key = hid_usage_to_layout_key(binding.param1);
            let tap_key = hid_usage_to_layout_key(binding.param2);
            return Some(LayoutKey {
                tap: tap_key.tap,
                hold: Some(hold_key.tap),
                symbol: tap_key.symbol,
                kind: KeycodeKind::Modifier,
                layer_ref: None,
            });
        }

        // --- Fallback: show display name + params ---
        let abbrev = layer_abbreviation(&info.display_name);
        let label = if binding.param2 != 0 {
            format!("{} {} {}", abbrev, binding.param1, binding.param2)
        } else if binding.param1 != 0 {
            format!("{} {}", abbrev, binding.param1)
        } else {
            abbrev.to_string()
        };

        Some(LayoutKey {
            tap: Label::new(label),
            ..Default::default()
        })
    }
}

/// Classify the dominant parameter type from a list of value descriptions.
fn classify_param(descs: &[behaviors::BehaviorParameterValueDescription]) -> ParamType {
    if descs.is_empty() {
        return ParamType::None;
    }
    // Take the first description's type as representative
    match &descs[0].value_type {
        Some(behaviors::behavior_parameter_value_description::ValueType::HidUsage(_)) => {
            ParamType::HidUsage
        }
        Some(behaviors::behavior_parameter_value_description::ValueType::LayerId(_)) => {
            ParamType::LayerId
        }
        Some(behaviors::behavior_parameter_value_description::ValueType::Constant(_)) => {
            ParamType::Constant
        }
        Some(behaviors::behavior_parameter_value_description::ValueType::Range(_)) => {
            ParamType::Range
        }
        Some(behaviors::behavior_parameter_value_description::ValueType::Nil(_)) => ParamType::None,
        None => ParamType::Unknown,
    }
}

/// Convert an HID usage code to a LayoutKey.
///
/// ZMK's HID usage codes for keyboard keys (0x00-0xFF) match QMK's basic
/// keycodes, so we can reuse the existing label table.
/// Consumer usage codes are offset above the keyboard range.
fn hid_usage_to_layout_key(usage: u32) -> LayoutKey {
    use crate::keycode_labels::get_basic_layout_key;

    // Keyboard usage page codes (0x00..0xFF) map directly to QMK keycodes
    if usage <= 0xFF {
        if let Some(key) = get_basic_layout_key(usage as u16) {
            return key;
        }
    }

    // Consumer usage codes — check common ones
    // ZMK encodes consumer usages with an implicit page flag; the raw proto
    // value is the HID consumer usage ID. Map known ones:
    let consumer_label: Option<(&str, &str, Option<&str>)> = match usage {
        // Common consumer control usages (HID Consumer Page 0x0C)
        0x00B5 => Some(("Next", "▶▶", None)),
        0x00B6 => Some(("Prev", "◀◀", None)),
        0x00B7 => Some(("Stop", "⏹", None)),
        0x00CD => Some(("Play/Pause", "⏯", None)),
        0x00E2 => Some(("Mute", "Mute", None)),
        0x00E9 => Some(("Vol+", "Vol+", None)),
        0x00EA => Some(("Vol-", "Vol-", None)),
        0x0183 => Some(("Media", "Media", None)),
        0x018A => Some(("Mail", "Mail", None)),
        0x0192 => Some(("Calc", "Calc", None)),
        0x0194 => Some(("Files", "Files", None)),
        0x0221 => Some(("Search", "Search", None)),
        0x0223 => Some(("Home", "WWW", None)),
        0x0224 => Some(("Back", "Back", None)),
        0x0225 => Some(("Forward", "Fwd", None)),
        0x0226 => Some(("Stop", "Stop", None)),
        0x0227 => Some(("Refresh", "Ref", None)),
        0x022A => Some(("Bookmarks", "Bkmk", None)),
        0x029D => Some(("Bright+", "Bri+", None)),
        0x029E => Some(("Bright-", "Bri-", None)),
        _ => None,
    };

    if let Some((full, short, _symbol)) = consumer_label {
        return LayoutKey {
            tap: Label::with_short(full, short),
            ..Default::default()
        };
    }

    // Fallback: show hex
    LayoutKey {
        tap: Label::new(format!("0x{:04X}", usage)),
        ..Default::default()
    }
}

/// Create a short abbreviation from a behavior display name.
fn layer_abbreviation(display_name: &str) -> &str {
    let lower = display_name.to_lowercase();
    // Common ZMK behavior name patterns
    if lower.contains("momentary") || lower == "mo" {
        return "MO";
    }
    if lower.contains("toggle") || lower == "tog" {
        return "TG";
    }
    if lower.contains("to layer") || lower == "to" {
        return "TO";
    }
    if lower.contains("layer tap") || lower == "lt" {
        return "LT";
    }
    if lower.contains("sticky") || lower.contains("one shot") {
        return "SL";
    }
    if lower.contains("conditional") {
        return "CL";
    }
    // Fallback: use first 3 chars uppercase
    if display_name.len() >= 3 {
        return &display_name[..3];
    }
    display_name
}

// ---------------------------------------------------------------------------
// High-level: fetch everything from a ZMK Studio device
// ---------------------------------------------------------------------------

/// All data fetched from a ZMK Studio device.
pub struct StudioData {
    pub physical_layouts: keymap::PhysicalLayouts,
    pub keymap: keymap::Keymap,
    pub behavior_map: BehaviorMap,
}

/// Connect to a ZMK Studio device, ensure it is unlocked, and fetch all
/// layout/keymap/behavior data. Returns an error with a special message
/// if the device is locked so the caller can show an unlock prompt.
pub fn fetch_studio_data(port_name: &str) -> Result<StudioData, Box<dyn Error>> {
    let mut client = StudioRpcClient::open(port_name)?;

    // Check lock state (unsecured RPC)
    let lock_state = client.get_lock_state()?;
    if lock_state == core::LockState::Locked {
        return Err("DEVICE_LOCKED".into());
    }

    // Fetch all data (secured RPCs — device must be unlocked)
    let physical_layouts = client.get_physical_layouts()?;
    let keymap = client.get_keymap()?;
    let behavior_details = client.fetch_all_behavior_details()?;
    let behavior_map = BehaviorMap::from_details(&behavior_details);

    Ok(StudioData {
        physical_layouts,
        keymap,
        behavior_map,
    })
}

/// Poll lock state until unlocked (with timeout).
#[allow(dead_code)]
pub fn wait_for_unlock(
    port_name: &str,
    timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    let mut client = StudioRpcClient::open(port_name)?;
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err("Timed out waiting for device unlock".into());
        }

        match client.get_lock_state() {
            Ok(core::LockState::Unlocked) => return Ok(()),
            Ok(core::LockState::Locked) => {
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(e) => return Err(e),
        }
    }
}
