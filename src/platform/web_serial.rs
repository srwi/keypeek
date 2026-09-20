//! Web Serial transport adapter for ZMK Studio keyboards in browser environments.

#[cfg(target_arch = "wasm32")]
use crate::protocols::DeviceError;
#[cfg(target_arch = "wasm32")]
use crate::ui_wake::UiWake;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};

use crate::protocols::DeviceEvent;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use zmk_studio_api::framing::FrameDecoder;
use zmk_studio_api::proto::zmk::studio;

/// Async timer utility that resolves after `ms` milliseconds using browser setTimeout.
#[cfg(target_arch = "wasm32")]
pub async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Returns whether the current browser environment supports the Web Serial API.
#[cfg(target_arch = "wasm32")]
pub fn is_web_serial_supported() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::has(&w.navigator(), &JsValue::from_str("serial")).ok())
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn is_web_serial_supported() -> bool {
    false
}

/// Extracts USB Vendor ID and Product ID from a `SerialPort` if available.
#[cfg(target_arch = "wasm32")]
pub fn get_serial_port_info(port: &web_sys::SerialPort) -> (Option<u16>, Option<u16>) {
    let Ok(get_info_val) = js_sys::Reflect::get(port, &JsValue::from_str("getInfo")) else {
        return (None, None);
    };
    if let Ok(func) = get_info_val.dyn_into::<js_sys::Function>() {
        if let Ok(info) = func.call0(port) {
            let vid = js_sys::Reflect::get(&info, &JsValue::from_str("usbVendorId"))
                .ok()
                .and_then(|v| v.as_f64().map(|n| n as u16));
            let pid = js_sys::Reflect::get(&info, &JsValue::from_str("usbProductId"))
                .ok()
                .and_then(|v| v.as_f64().map(|n| n as u16));
            return (vid, pid);
        }
    }
    (None, None)
}

/// Prompts the user to select and pair a Web Serial port.
#[cfg(target_arch = "wasm32")]
pub async fn request_web_serial_port() -> Result<Option<web_sys::SerialPort>, DeviceError> {
    let window =
        web_sys::window().ok_or_else(|| DeviceError::Transport("No window found".into()))?;
    if !is_web_serial_supported() {
        return Err(DeviceError::Unsupported(
            "Web Serial is not supported by your browser. Please use Chrome, Edge, or Opera."
                .into(),
        ));
    }

    let serial = window.navigator().serial();
    let promise = serial.request_port();
    let port_val = match wasm_bindgen_futures::JsFuture::from(promise).await {
        Ok(val) => val,
        Err(err) => {
            let err_str = format!("{err:?}");
            if err_str.contains("AbortError")
                || err_str.contains("cancel")
                || err_str.contains("user did not select")
            {
                return Ok(None);
            }
            return Err(DeviceError::Transport(format!(
                "Web Serial request error: {err_str}"
            )));
        }
    };

    let port: web_sys::SerialPort = port_val
        .dyn_into()
        .map_err(|e| DeviceError::Transport(format!("Expected SerialPort: {e:?}")))?;

    // Open port with 115200 baud rate (standard CDC-ACM virtual serial port)
    let options = web_sys::SerialOptions::new(115_200);
    let open_promise = port.open(&options);
    if let Err(e) = wasm_bindgen_futures::JsFuture::from(open_promise).await {
        let err_str = format!("{e:?}");
        if !err_str.contains("already open") {
            return Err(DeviceError::Transport(format!(
                "Failed to open Web Serial port: {err_str}. The port may be open in another tab or application."
            )));
        }
    }

    Ok(Some(port))
}

#[cfg(target_arch = "wasm32")]
type ResponseMap = HashMap<u32, futures_channel::oneshot::Sender<studio::RequestResponse>>;

/// Web Serial transport for ZMK Studio RPC communication in WebAssembly.
#[cfg(target_arch = "wasm32")]
pub struct WebSerialTransport {
    port: web_sys::SerialPort,
    alive: Arc<AtomicBool>,
    next_request_id: AtomicU32,
    pending_requests: Arc<Mutex<ResponseMap>>,
    _ui_wake: UiWake,
}

/// Demuxed frame from the serial stream: either a ZMK Studio RPC response or a companion telemetry event.
#[derive(Debug, PartialEq)]
pub enum DemuxedSerialFrame {
    Response(studio::Response),
    Event(DeviceEvent),
}

/// Demuxes a SLIP-decoded frame from the serial stream:
/// - If it is a valid ZMK Studio protobuf response, returns `Some(DemuxedSerialFrame::Response)`.
/// - If it is a companion telemetry packet (0xFF / 0xF1), returns `Some(DemuxedSerialFrame::Event)`.
/// - Otherwise returns `None`.
pub fn demux_serial_frame(frame: &[u8]) -> Option<DemuxedSerialFrame> {
    if let Ok(response) = prost::Message::decode(frame) {
        Some(DemuxedSerialFrame::Response(response))
    } else {
        crate::protocols::decode_raw_hid_packet(frame).map(DemuxedSerialFrame::Event)
    }
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for WebSerialTransport {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WebSerialTransport {}

#[cfg(target_arch = "wasm32")]
impl WebSerialTransport {
    /// Creates a new `WebSerialTransport` wrapping an opened `web_sys::SerialPort`.
    ///
    /// Spawns a background read loop that demuxes responses using ZMK Studio framing
    /// and delivers decoded RPC responses and telemetry events into `event_tx`.
    pub fn new(
        port: web_sys::SerialPort,
        event_tx: std::sync::mpsc::Sender<DeviceEvent>,
        ui_wake: UiWake,
    ) -> Result<Self, DeviceError> {
        let alive = Arc::new(AtomicBool::new(true));
        let next_request_id = AtomicU32::new(1);
        let pending_requests: Arc<Mutex<ResponseMap>> = Arc::new(Mutex::new(HashMap::new()));

        let readable = port.readable();
        let reader: web_sys::ReadableStreamDefaultReader = readable.get_reader().unchecked_into();

        let reader_alive = Arc::clone(&alive);
        let reader_pending = Arc::clone(&pending_requests);
        let wake_clone = ui_wake.clone();
        let event_tx_clone = event_tx;

        wasm_bindgen_futures::spawn_local(async move {
            let mut decoder = FrameDecoder::new();
            while reader_alive.load(Ordering::Relaxed) {
                let promise = reader.read();
                match wasm_bindgen_futures::JsFuture::from(promise).await {
                    Ok(result) => {
                        let done = js_sys::Reflect::get(&result, &JsValue::from_str("done"))
                            .ok()
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if done {
                            break;
                        }
                        if let Ok(value) =
                            js_sys::Reflect::get(&result, &JsValue::from_str("value"))
                        {
                            if !value.is_undefined() && !value.is_null() {
                                let uint8_arr: js_sys::Uint8Array = value.unchecked_into();
                                let mut chunk = vec![0u8; uint8_arr.length() as usize];
                                uint8_arr.copy_to(&mut chunk);
                                let frames = decoder.push(&chunk);
                                for frame in frames {
                                    match demux_serial_frame(&frame) {
                                        Some(DemuxedSerialFrame::Response(response)) => {
                                            match response.r#type {
                                                Some(studio::response::Type::RequestResponse(
                                                    rr,
                                                )) => {
                                                    let mut pending =
                                                        reader_pending.lock().unwrap();
                                                    if let Some(tx) = pending.remove(&rr.request_id)
                                                    {
                                                        let _ = tx.send(rr);
                                                    }
                                                }
                                                Some(studio::response::Type::Notification(_n)) => {
                                                    wake_clone.request_repaint();
                                                }
                                                None => {}
                                            }
                                        }
                                        Some(DemuxedSerialFrame::Event(event)) => {
                                            let _ = event_tx_clone.send(event);
                                            wake_clone.request_repaint();
                                        }
                                        None => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Web Serial reader encountered error: {e:?}");
                        break;
                    }
                }
            }
            reader.release_lock();
        });

        Ok(Self {
            port,
            alive,
            next_request_id,
            pending_requests,
            _ui_wake: ui_wake,
        })
    }

    /// Access the underlying `web_sys::SerialPort`.
    pub fn port(&self) -> &web_sys::SerialPort {
        &self.port
    }

    /// Asynchronously sends raw framed bytes to the serial port.
    async fn send_bytes(&self, data: &[u8]) -> Result<(), DeviceError> {
        let writable = self.port.writable();
        let writer = writable
            .get_writer()
            .map_err(|e| DeviceError::Transport(format!("Failed to get serial writer: {e:?}")))?;

        let chunk = js_sys::Uint8Array::new_with_length(data.len() as u32);
        chunk.copy_from(data);
        let promise = writer.write_with_chunk(&chunk);
        let res = wasm_bindgen_futures::JsFuture::from(promise).await;
        writer.release_lock();

        res.map_err(|e| DeviceError::Transport(format!("Failed to write to serial port: {e:?}")))?;
        Ok(())
    }

    /// Sends a ZMK Studio RPC request and awaits the corresponding response with retry logic.
    pub async fn call(
        &self,
        subsystem: studio::request::Subsystem,
    ) -> Result<studio::RequestResponse, DeviceError> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        let request = studio::Request {
            request_id,
            subsystem: Some(subsystem),
        };
        let bytes = zmk_studio_api::protocol::encode_request(&request);

        const MAX_ATTEMPTS: usize = 3;
        const TIMEOUT_MS: i32 = 1500;

        for attempt in 1..=MAX_ATTEMPTS {
            let (tx, rx) = futures_channel::oneshot::channel();
            self.pending_requests.lock().unwrap().insert(request_id, tx);

            self.send_bytes(&bytes).await?;

            use futures_util::future::{select, Either};
            match select(rx, Box::pin(sleep_ms(TIMEOUT_MS))).await {
                Either::Left((Ok(response), _)) => {
                    return Ok(response);
                }
                Either::Left((Err(_), _)) => {
                    return Err(DeviceError::Transport("Web Serial channel closed".into()));
                }
                Either::Right(_) => {
                    log::warn!(
                        "Web Serial RPC request {request_id} timed out on attempt {attempt}/{MAX_ATTEMPTS}"
                    );
                    self.pending_requests.lock().unwrap().remove(&request_id);
                }
            }
        }

        Err(DeviceError::Transport(format!(
            "Timed out waiting for response to ZMK RPC request {request_id}"
        )))
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for WebSerialTransport {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}

/// Prompts the user to select a Web Serial port, connects to the ZMK Studio keyboard,
/// and returns the [`WebConnectOutcome`].
#[cfg(target_arch = "wasm32")]
pub async fn request_and_connect_zmk(
    overlay_config: crate::domain::visibility::OverlayConfig,
    ui_wake: UiWake,
) -> Result<Option<super::web_hid::WebConnectOutcome>, DeviceError> {
    let Some(port) = request_web_serial_port().await? else {
        return Ok(None);
    };

    let (vid_opt, pid_opt) = get_serial_port_info(&port);
    let vid = vid_opt.unwrap_or(0x1D50);
    let pid = pid_opt.unwrap_or(0x615E);

    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let transport = Arc::new(WebSerialTransport::new(
        port,
        event_tx.clone(),
        ui_wake.clone(),
    )?);

    // Check if WebHID telemetry is already authorized for this device
    let hid_transport = crate::platform::web_hid::WebHidTransport::open_paired(
        vid,
        pid,
        event_tx.clone(),
        ui_wake.clone(),
    )
    .await;

    let pending_telemetry = if hid_transport.is_none() {
        Some(super::web_hid::PendingZmkTelemetry { vid, pid, event_tx })
    } else {
        None
    };

    let protocol = crate::firmware::zmk::web::connect_web_zmk(
        Arc::clone(&transport),
        vid,
        pid,
        event_rx,
        hid_transport,
    )
    .await?;

    let discovered = crate::device_discovery::DiscoveredDevice {
        base_name: format!("ZMK Studio ({vid:04X}:{pid:04X})"),
        vid,
        pid,
        driver_id: "zmk",
        protocol_label: "Web Serial",
        requires_layout_file: false,
        spec: crate::protocols::ConnectionSpec::Zmk {
            vid,
            pid,
            transport: crate::protocols::ZmkTransportConfig::Serial("WebSerial".to_string()),
        },
    };

    let connected =
        super::web::ConnectedWebDevice::new(discovered, protocol, overlay_config, ui_wake)?;

    Ok(Some(super::web_hid::WebConnectOutcome::ZmkConnected {
        connected,
        pending_telemetry,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use zmk_studio_api::framing::{encode_frame, FrameDecoder, FRAMING_EOF, FRAMING_SOF};
    use zmk_studio_api::proto::zmk::studio::{
        request::Subsystem, Request, RequestResponse, Response,
    };
    use zmk_studio_api::protocol::{decode_responses, encode_request};

    #[test]
    fn test_web_serial_framing_and_decode() {
        let request = Request {
            request_id: 42,
            subsystem: Some(Subsystem::Core(zmk_studio_api::proto::zmk::core::Request {
                request_type: Some(
                    zmk_studio_api::proto::zmk::core::request::RequestType::GetLockState(true),
                ),
            })),
        };
        let encoded = encode_request(&request);
        assert_eq!(encoded.first(), Some(&FRAMING_SOF));
        assert_eq!(encoded.last(), Some(&FRAMING_EOF));

        let mut decoder = FrameDecoder::new();

        let response = Response {
            r#type: Some(zmk_studio_api::proto::zmk::studio::response::Type::RequestResponse(
                RequestResponse {
                    request_id: 42,
                    subsystem: Some(zmk_studio_api::proto::zmk::studio::request_response::Subsystem::Core(
                        zmk_studio_api::proto::zmk::core::Response {
                            response_type: Some(zmk_studio_api::proto::zmk::core::response::ResponseType::GetLockState(
                                zmk_studio_api::proto::zmk::core::LockState::ZmkStudioCoreLockStateUnlocked as i32,
                            )),
                        },
                    )),
                },
            )),
        };

        let response_wire = encode_frame(&response.encode_to_vec());

        // Test with corrupted byte prefix (stale UART buffer) and trailing garbage
        let mut corrupted_stream = vec![0x12, 0x34, 0x56];
        corrupted_stream.extend_from_slice(&response_wire);
        corrupted_stream.extend_from_slice(&[0x78, 0x9A]);

        let decoded_responses = decode_responses(&mut decoder, &corrupted_stream);
        assert_eq!(decoded_responses.len(), 1);
        match &decoded_responses[0].r#type {
            Some(zmk_studio_api::proto::zmk::studio::response::Type::RequestResponse(rr)) => {
                assert_eq!(rr.request_id, 42);
            }
            _ => panic!("Expected RequestResponse"),
        }
    }

    #[test]
    fn test_demux_serial_frame_rpc_response() {
        let response = Response {
            r#type: Some(
                zmk_studio_api::proto::zmk::studio::response::Type::RequestResponse(
                    RequestResponse {
                        request_id: 123,
                        subsystem: None,
                    },
                ),
            ),
        };
        let bytes = response.encode_to_vec();
        match demux_serial_frame(&bytes) {
            Some(DemuxedSerialFrame::Response(resp)) => match resp.r#type {
                Some(zmk_studio_api::proto::zmk::studio::response::Type::RequestResponse(rr)) => {
                    assert_eq!(rr.request_id, 123);
                }
                _ => panic!("Expected RequestResponse"),
            },
            other => panic!("Expected Some(DemuxedSerialFrame::Response), got {other:?}"),
        }
    }

    #[test]
    fn test_demux_serial_frame_companion_layer_packet() {
        // [0xFF, size=4, default=1, active=4]
        let mut packet = vec![0xFF, 4];
        packet.extend_from_slice(&1u32.to_le_bytes());
        packet.extend_from_slice(&4u32.to_le_bytes());

        match demux_serial_frame(&packet) {
            Some(DemuxedSerialFrame::Event(DeviceEvent::LayersChanged {
                active_layers,
                default_layers,
            })) => {
                assert_eq!(active_layers, 4);
                assert_eq!(default_layers, 1);
            }
            other => {
                panic!("Expected Some(DemuxedSerialFrame::Event(LayersChanged)), got {other:?}")
            }
        }
    }

    #[test]
    fn test_demux_serial_frame_companion_key_packet() {
        // [0xF1, row=2, col=5, pressed=1]
        let packet = vec![0xF1, 2, 5, 1];

        match demux_serial_frame(&packet) {
            Some(DemuxedSerialFrame::Event(DeviceEvent::KeyPressed { row, col, pressed })) => {
                assert_eq!(row, 2);
                assert_eq!(col, 5);
                assert!(pressed);
            }
            other => panic!("Expected Some(DemuxedSerialFrame::Event(KeyPressed)), got {other:?}"),
        }
    }
}
