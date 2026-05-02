use crate::keyboard::Keyboard;
use crate::protocols::{connect_protocol, ConnectionSpec, KeyboardDefinition, KeyboardProtocol};
use crate::ui_wake::UiWake;
use std::sync::mpsc::{self, TryRecvError};

const ZMK_LOCKED_ERROR: &str = "Device is locked. Please press the ZMK Studio unlock key combination on your keyboard, then click Connect again.";

pub struct ConnectionRequest {
    pub spec: ConnectionSpec,
    pub timeout: i64,
    pub layout_name: Option<String>,
}

impl ConnectionRequest {
    fn connect_protocol(&self) -> Result<Box<dyn KeyboardProtocol>, String> {
        connect_protocol(&self.spec).map_err(|e| format_connect_error(&self.spec, &e.to_string()))
    }

    fn pick_layout_name(&self, layout_names: &[String]) -> Result<String, String> {
        if layout_names.is_empty() {
            return Err("Device did not provide any layouts".to_string());
        }

        if let Some(name) = &self.layout_name {
            if layout_names.contains(name) {
                return Ok(name.clone());
            }
        }

        Ok(layout_names[0].clone())
    }
}

fn format_connect_error(spec: &ConnectionSpec, error_text: &str) -> String {
    if matches!(spec, ConnectionSpec::Zmk { .. }) {
        if error_text == "DEVICE_LOCKED" {
            return ZMK_LOCKED_ERROR.to_string();
        }
        return format!("ZMK error: {error_text}");
    }

    format!("Failed to connect to device: {error_text}")
}

pub struct ConnectedState {
    pub spec: ConnectionSpec,
    pub definition: KeyboardDefinition,
    pub layout_names: Vec<String>,
    pub selected_layout_name: String,
    pub keyboard: Keyboard,
}

pub struct ConnectionTask {
    rx: mpsc::Receiver<Result<ConnectedState, String>>,
}

impl ConnectionTask {
    pub fn start(request: ConnectionRequest, ui_wake: UiWake) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = build_connected_state(request, ui_wake.clone());
            let _ = tx.send(result);
            ui_wake.request_repaint();
        });
        Self { rx }
    }

    pub fn try_finish(&self) -> Option<Result<ConnectedState, String>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Err("Background connection task failed".to_string()))
            }
        }
    }
}

pub fn build_connected_state(
    request: ConnectionRequest,
    ui_wake: UiWake,
) -> Result<ConnectedState, String> {
    let protocol = request.connect_protocol()?;

    let layout_names = protocol.get_layout_definition().get_layout_names();
    let selected_layout_name = request.pick_layout_name(&layout_names)?;
    let definition = protocol.get_layout_definition().clone();

    let keyboard = Keyboard::new(
        protocol,
        selected_layout_name.clone(),
        request.timeout,
        ui_wake,
    )
    .map_err(|e| format!("Failed to create keyboard: {e}"))?;

    Ok(ConnectedState {
        spec: request.spec,
        definition,
        layout_names,
        selected_layout_name,
        keyboard,
    })
}
