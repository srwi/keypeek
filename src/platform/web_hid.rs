//! WebHID transport adapter and companion packet demuxer for browser environments.

use crate::protocols::{decode_raw_hid_packet, DeviceError, DeviceEvent};
use crate::ui_wake::UiWake;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Maximum size of a standard raw HID / VIA report.
pub const RAW_REPORT_SIZE: usize = 32;

/// Shared queue and async waker for incoming VIA RPC response packets.
#[derive(Default, Debug)]
pub struct ResponseSlot {
    pending: VecDeque<Vec<u8>>,
    waker: Option<Waker>,
}

impl ResponseSlot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues a response packet and wakes any task waiting for it.
    pub fn push_response(&mut self, packet: Vec<u8>) {
        self.pending.push_back(packet);
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    /// Takes the next queued response packet if available.
    pub fn pop_response(&mut self) -> Option<Vec<u8>> {
        self.pending.pop_front()
    }
}

/// A future that resolves when a VIA RPC response packet is received.
pub struct ResponseFuture {
    slot: Arc<Mutex<ResponseSlot>>,
}

impl Future for ResponseFuture {
    type Output = Result<Vec<u8>, DeviceError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.slot.lock().unwrap();
        if let Some(packet) = guard.pop_response() {
            Poll::Ready(Ok(packet))
        } else {
            guard.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// Demuxes an incoming raw HID packet.
///
/// If the packet is a KeyPeek companion firmware notification (`0xFF` for layer changes
/// or `0xF1` for key presses), it is forwarded to `event_tx` and triggers an egui repaint.
/// Otherwise, it is treated as a VIA RPC response and enqueued in `response_slot`.
pub fn demux_input_report(
    bytes: &[u8],
    event_tx: &mpsc::Sender<DeviceEvent>,
    response_slot: &Mutex<ResponseSlot>,
    ui_wake: &UiWake,
) {
    if let Some(device_event) = decode_raw_hid_packet(bytes) {
        let _ = event_tx.send(device_event);
        ui_wake.request_repaint();
    } else {
        let mut slot = response_slot.lock().unwrap();
        slot.push_response(bytes.to_vec());
    }
}

/// WebHID transport for QMK/VIA keyboards running in browser WebAssembly builds.
#[cfg(target_arch = "wasm32")]
pub struct WebHidTransport {
    device: web_sys::HidDevice,
    event_rx: Mutex<Option<mpsc::Receiver<DeviceEvent>>>,
    response_slot: Arc<Mutex<ResponseSlot>>,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for WebHidTransport {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WebHidTransport {}

#[cfg(target_arch = "wasm32")]
impl WebHidTransport {
    /// Creates a new `WebHidTransport` wrapping an opened `web_sys::HidDevice`.
    ///
    /// Attaches an `inputreport` listener that demuxes companion telemetry from VIA RPC responses.
    pub fn new(device: web_sys::HidDevice, ui_wake: UiWake) -> Result<Self, DeviceError> {
        let (event_tx, event_rx) = mpsc::channel();
        let response_slot = Arc::new(Mutex::new(ResponseSlot::new()));

        let slot_for_closure = Arc::clone(&response_slot);
        let wake_for_closure = ui_wake.clone();
        let closure = Closure::<dyn FnMut(web_sys::HidInputReportEvent)>::new(
            move |event: web_sys::HidInputReportEvent| {
                let data = event.data();
                let len = data.byte_length();
                let mut buffer = vec![0u8; len];
                for (i, b) in buffer.iter_mut().enumerate() {
                    *b = data.get_uint8(i);
                }
                demux_input_report(&buffer, &event_tx, &slot_for_closure, &wake_for_closure);
            },
        );

        device
            .add_event_listener_with_callback("inputreport", closure.as_ref().unchecked_ref())
            .map_err(|e| DeviceError::Transport(format!("Failed to attach inputreport listener: {e:?}")))?;

        closure.forget();

        Ok(Self {
            device,
            event_rx: Mutex::new(Some(event_rx)),
            response_slot,
        })
    }

    /// Access the underlying `web_sys::HidDevice`.
    pub fn device(&self) -> &web_sys::HidDevice {
        &self.device
    }

    /// Claims the receiver for demuxed `DeviceEvent`s (layers changed, key pressed).
    pub fn take_event_receiver(&self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError> {
        self.event_rx
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| DeviceError::Transport("Event receiver already claimed".into()))
    }

    /// Sends an output report without awaiting resolution (for non-blocking writes).
    pub fn fire_and_forget_report(&self, data: &[u8]) {
        let mut padded = vec![0u8; RAW_REPORT_SIZE];
        let copy_len = data.len().min(RAW_REPORT_SIZE);
        padded[..copy_len].copy_from_slice(&data[..copy_len]);

        let uint8_array = js_sys::Uint8Array::new_with_length(RAW_REPORT_SIZE as u32);
        uint8_array.copy_from(&padded);
        if let Ok(promise) = self.device.send_report_with_u8_array(0, &uint8_array) {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            });
        }
    }

    /// Asynchronously sends a raw output report (report ID 0) to the device.
    pub async fn send_report(&self, data: &[u8]) -> Result<(), DeviceError> {
        let mut padded = vec![0u8; RAW_REPORT_SIZE];
        let copy_len = data.len().min(RAW_REPORT_SIZE);
        padded[..copy_len].copy_from_slice(&data[..copy_len]);

        let uint8_array = js_sys::Uint8Array::new_with_length(RAW_REPORT_SIZE as u32);
        uint8_array.copy_from(&padded);
        let promise = self
            .device
            .send_report_with_u8_array(0, &uint8_array)
            .map_err(|e| DeviceError::Transport(format!("WebHID send_report call failed: {e:?}")))?;
        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| DeviceError::Transport(format!("WebHID send_report failed: {e:?}")))?;
        Ok(())
    }

    /// Asynchronously awaits the next VIA RPC response packet.
    pub fn read_response(&self) -> ResponseFuture {
        ResponseFuture {
            slot: Arc::clone(&self.response_slot),
        }
    }

    /// Sends a VIA command and awaits the corresponding response with a timeout in milliseconds.
    pub async fn command_timeout(
        &self,
        command_id: u8,
        payload: &[u8],
        timeout_ms: i32,
    ) -> Result<Vec<u8>, DeviceError> {
        // Clear any stale responses before sending a new command
        self.response_slot.lock().unwrap().pending.clear();

        let mut report = vec![0u8; RAW_REPORT_SIZE];
        report[0] = command_id;
        let copy_len = payload.len().min(RAW_REPORT_SIZE - 1);
        report[1..1 + copy_len].copy_from_slice(&payload[..copy_len]);

        self.send_report(&report).await?;

        use futures_util::future::{select, Either};
        match select(
            Box::pin(self.read_response()),
            Box::pin(sleep_ms(timeout_ms)),
        )
        .await
        {
            Either::Left((resp, _)) => resp,
            Either::Right(_) => Err(DeviceError::Transport(format!(
                "Timed out waiting for response to command 0x{command_id:02X}"
            ))),
        }
    }

    /// Sends a VIA command and awaits the corresponding response (with a 1000ms timeout).
    pub async fn command(&self, command_id: u8, payload: &[u8]) -> Result<Vec<u8>, DeviceError> {
        self.command_timeout(command_id, payload, 1000).await
    }
}

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

#[cfg(target_arch = "wasm32")]
impl qmk_via_api::ViaTransport for WebHidTransport {
    fn write_report(&mut self, report: &[u8]) -> Result<(), qmk_via_api::Error> {
        let uint8_array = js_sys::Uint8Array::new_with_length(report.len() as u32);
        uint8_array.copy_from(report);
        let promise = self
            .device
            .send_report_with_u8_array(0, &uint8_array)
            .map_err(|e| qmk_via_api::Error::Hid(format!("{e:?}")))?;
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = wasm_bindgen_futures::JsFuture::from(promise).await {
                log::error!("WebHID write_report error: {e:?}");
            }
        });
        Ok(())
    }

    fn read_report(&mut self, _timeout_ms: Option<i32>) -> Result<Vec<u8>, qmk_via_api::Error> {
        let mut slot = self.response_slot.lock().unwrap();
        slot.pop_response().ok_or_else(|| {
            qmk_via_api::Error::Hid("No pending WebHID response".into())
        })
    }
}

/// Returns whether the current browser environment supports the WebHID API.
#[cfg(target_arch = "wasm32")]
pub fn is_web_hid_supported() -> bool {
    web_sys::window()
        .and_then(|w| {
            js_sys::Reflect::has(&w.navigator(), &wasm_bindgen::JsValue::from_str("hid")).ok()
        })
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn is_web_hid_supported() -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
use crate::device_discovery::DiscoveredDevice;
#[cfg(target_arch = "wasm32")]
use crate::protocols::ConnectionSpec;
#[cfg(target_arch = "wasm32")]
use crate::protocols::KeyboardProtocol;

/// Prompts the user to select and pair a WebHID device matching QMK/VIA usage page (0xFF60).
#[cfg(target_arch = "wasm32")]
pub async fn request_web_hid_device() -> Result<Option<web_sys::HidDevice>, DeviceError> {
    let window =
        web_sys::window().ok_or_else(|| DeviceError::Transport("No window found".into()))?;
    if !is_web_hid_supported() {
        return Err(DeviceError::Unsupported(
            "WebHID is not supported by your browser. Please use Chrome, Edge, or Opera.".into(),
        ));
    }

    let hid = window.navigator().hid();
    let filter = web_sys::HidDeviceFilter::new();
    filter.set_usage_page(0xFF60);
    let filters = [filter];
    let options = web_sys::HidDeviceRequestOptions::new(&filters);

    let promise = hid.request_device(&options);
    let devices_val = match wasm_bindgen_futures::JsFuture::from(promise).await {
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
                "WebHID request error: {err_str}"
            )));
        }
    };

    let devices: js_sys::Array = devices_val
        .dyn_into()
        .map_err(|e| DeviceError::Transport(format!("Expected Array from requestDevice: {e:?}")))?;

    if devices.length() == 0 {
        return Ok(None);
    }

    let device: web_sys::HidDevice = devices
        .get(0)
        .dyn_into()
        .map_err(|e| DeviceError::Transport(format!("Expected HidDevice: {e:?}")))?;

    if !device.opened() {
        let open_promise = device.open();
        wasm_bindgen_futures::JsFuture::from(open_promise)
            .await
            .map_err(|e| DeviceError::Transport(format!("Failed to open WebHID device: {e:?}")))?;
    }

    Ok(Some(device))
}


#[cfg(target_arch = "wasm32")]
pub struct ConnectedWebDevice {
    pub device: DiscoveredDevice,
    pub keyboard: Arc<crate::application::Keyboard>,
    pub profile: Arc<dyn crate::keymap_editor::EditorProfile>,
}

#[cfg(target_arch = "wasm32")]
impl ConnectedWebDevice {
    pub fn from_protocol(
        device: DiscoveredDevice,
        protocol: impl KeyboardProtocol + 'static,
        overlay_config: crate::domain::visibility::OverlayConfig,
        ui_wake: UiWake,
    ) -> Result<Self, DeviceError> {
        let layout_names = protocol.get_layout_definition().get_layout_names();
        let selected_layout = layout_names
            .first()
            .cloned()
            .unwrap_or_else(|| "default".to_string());

        let keyboard = crate::application::Keyboard::new(
            Box::new(protocol),
            selected_layout,
            overlay_config,
            ui_wake,
            Arc::new(crate::firmware::qmk::QmkKeyPresenter),
        )
        .map_err(DeviceError::Protocol)?;

        Ok(Self {
            device,
            keyboard: Arc::new(keyboard),
            profile: Arc::new(crate::firmware::qmk::QmkEditorProfile),
        })
    }
}

#[cfg(target_arch = "wasm32")]
pub enum WebConnectOutcome {
    Connected(ConnectedWebDevice),
    RequiresLayoutFile {
        device: DiscoveredDevice,
        transport: Arc<WebHidTransport>,
    },
}

/// Requests a WebHID device from user gesture, opens it, connects the live QMK/Vial protocol,
/// and returns the [`WebConnectOutcome`].
#[cfg(target_arch = "wasm32")]
pub async fn request_and_connect_device(
    overlay_config: crate::domain::visibility::OverlayConfig,
    ui_wake: UiWake,
) -> Result<Option<WebConnectOutcome>, DeviceError> {
    let Some(raw_device) = request_web_hid_device().await? else {
        return Ok(None);
    };

    let vid = raw_device.vendor_id();
    let pid = raw_device.product_id();
    let name = raw_device.product_name();
    let base_name = if name.trim().is_empty() {
        format!("USB Device ({vid:04X}:{pid:04X})")
    } else {
        name
    };

    let transport = Arc::new(WebHidTransport::new(raw_device, ui_wake.clone())?);

    let discovered = DiscoveredDevice {
        base_name,
        vid,
        pid,
        driver_id: "qmk",
        protocol_label: "WebHID",
        requires_layout_file: false,
        spec: ConnectionSpec::Vial { vid, pid },
    };

    match crate::firmware::qmk::web::connect_web_qmk(
        Arc::clone(&transport),
        vid,
        pid,
        None,
    )
    .await?
    {
        crate::firmware::qmk::web::WebQmkOutcome::Connected(protocol) => {
            let connected =
                ConnectedWebDevice::from_protocol(discovered, protocol, overlay_config, ui_wake)?;
            Ok(Some(WebConnectOutcome::Connected(connected)))
        }
        crate::firmware::qmk::web::WebQmkOutcome::RequiresLayoutFile => {
            let mut dev = discovered;
            dev.requires_layout_file = true;
            dev.spec = ConnectionSpec::Via {
                json_path: String::new(),
            };
            Ok(Some(WebConnectOutcome::RequiresLayoutFile {
                device: dev,
                transport,
            }))
        }
    }
}

/// Connects a VIA keyboard over WebHID using a user-provided layout JSON string.
#[cfg(target_arch = "wasm32")]
pub async fn connect_via_with_layout(
    device: DiscoveredDevice,
    transport: Arc<WebHidTransport>,
    layout_json: String,
    overlay_config: crate::domain::visibility::OverlayConfig,
    ui_wake: UiWake,
) -> Result<ConnectedWebDevice, DeviceError> {
    let vid = device.vid;
    let pid = device.pid;

    let outcome = crate::firmware::qmk::web::connect_web_qmk(
        transport,
        vid,
        pid,
        Some(&layout_json),
    )
    .await?;

    match outcome {
        crate::firmware::qmk::web::WebQmkOutcome::Connected(protocol) => {
            ConnectedWebDevice::from_protocol(device, protocol, overlay_config, ui_wake)
        }
        crate::firmware::qmk::web::WebQmkOutcome::RequiresLayoutFile => {
            Err(DeviceError::Protocol("Layout definition is required".into()))
        }
    }
}

/// Opens the browser's native file chooser and sends (filename, text_content) to tx.
#[cfg(target_arch = "wasm32")]
pub fn trigger_web_file_picker(
    tx: mpsc::Sender<Result<(String, String), String>>,
    ui_wake: UiWake,
) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };
    let input: web_sys::HtmlInputElement = match document.create_element("input") {
        Ok(el) => match el.dyn_into() {
            Ok(inp) => inp,
            Err(_) => return,
        },
        Err(_) => return,
    };

    input.set_type("file");
    let _ = input.set_attribute("accept", ".json,application/json");

    let input_clone = input.clone();
    let tx_clone = tx.clone();
    let wake = ui_wake.clone();
    let closure = Closure::<dyn FnMut()>::new(move || {
        if let Some(files) = input_clone.files() {
            if let Some(file) = files.get(0) {
                let filename = file.name();
                let promise = file.text();
                let tx_inner = tx_clone.clone();
                let wake_inner = wake.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match wasm_bindgen_futures::JsFuture::from(promise).await {
                        Ok(js_val) => {
                            if let Some(text) = js_val.as_string() {
                                let _ = tx_inner.send(Ok((filename, text)));
                            } else {
                                let _ = tx_inner.send(Err("Failed to read file as text".into()));
                            }
                        }
                        Err(e) => {
                            let _ = tx_inner.send(Err(format!("File read error: {e:?}")));
                        }
                    }
                    wake_inner.request_repaint();
                });
            }
        }
    });

    input.set_onchange(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    input.click();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demux_layer_state_packet() {
        let (event_tx, event_rx) = mpsc::channel();
        let slot = Mutex::new(ResponseSlot::new());
        let ui_wake = UiWake::new(Arc::new(|| {}));

        // 0xFF layer state packet: size=1, default=0, active=2
        let packet = [0xFF, 1, 0, 2];
        demux_input_report(&packet, &event_tx, &slot, &ui_wake);

        assert!(slot.lock().unwrap().pop_response().is_none());
        let event = event_rx.try_recv().expect("expected event");
        assert_eq!(
            event,
            DeviceEvent::LayersChanged {
                active_layers: 2,
                default_layers: 0,
            }
        );
    }

    #[test]
    fn test_demux_key_event_packet() {
        let (event_tx, event_rx) = mpsc::channel();
        let slot = Mutex::new(ResponseSlot::new());
        let ui_wake = UiWake::new(Arc::new(|| {}));

        // 0xF1 key event packet: row=1, col=3, pressed=1
        let packet = [0xF1, 1, 3, 1];
        demux_input_report(&packet, &event_tx, &slot, &ui_wake);

        assert!(slot.lock().unwrap().pop_response().is_none());
        let event = event_rx.try_recv().expect("expected event");
        assert_eq!(
            event,
            DeviceEvent::KeyPressed {
                row: 1,
                col: 3,
                pressed: true,
            }
        );
    }

    #[test]
    fn test_demux_via_rpc_response() {
        let (event_tx, event_rx) = mpsc::channel();
        let slot = Mutex::new(ResponseSlot::new());
        let ui_wake = UiWake::new(Arc::new(|| {}));

        // Standard VIA GetProtocolVersion response (starts with 0x01)
        let packet = vec![0x01, 0x00, 0x0C, 0x00];
        demux_input_report(&packet, &event_tx, &slot, &ui_wake);

        assert!(event_rx.try_recv().is_err());
        let response = slot.lock().unwrap().pop_response().expect("expected response");
        assert_eq!(response, packet);
    }
}
