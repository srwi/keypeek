use crate::keyboard::{Keyboard, OverlayConfig};
use crate::protocols::{
    connect_protocol, ConnectionSpec, DeviceLocked, KeyboardDefinition, KeyboardProtocol, Reopener,
};
use crate::ui_wake::UiWake;
use std::error::Error;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;

const ZMK_LOCKED_ERROR: &str = "Device is locked. Please press the ZMK Studio unlock key combination on your keyboard, then click Connect again.";

pub struct ConnectionRequest {
    pub spec: ConnectionSpec,
    pub overlay_config: OverlayConfig,
    pub layout_name: Option<String>,
    pub reopen: Option<Arc<dyn Reopener>>,
}

impl ConnectionRequest {
    fn open_protocol(&self) -> Result<Box<dyn KeyboardProtocol>, String> {
        let result = match &self.reopen {
            Some(reopener) => reopener.reopen(),
            None => connect_protocol(&self.spec),
        };
        result.map_err(|e| format_connect_error(&self.spec, e.as_ref()))
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

fn format_connect_error(spec: &ConnectionSpec, error: &(dyn Error + 'static)) -> String {
    if matches!(spec, ConnectionSpec::Zmk { .. }) {
        if error.downcast_ref::<DeviceLocked>().is_some() {
            return ZMK_LOCKED_ERROR.to_string();
        }
        return format!("ZMK error: {error}");
    }

    format!("Failed to connect to device: {error}")
}

pub struct ConnectedState {
    pub definition: KeyboardDefinition,
    pub layout_names: Vec<String>,
    pub selected_layout_name: String,
    pub keyboard: Keyboard,
    pub reopen: Option<Arc<dyn Reopener>>,
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
    let protocol = request.open_protocol()?;

    let reopen = protocol.reopener();
    let layout_names = protocol.get_layout_definition().get_layout_names();
    let selected_layout_name = request.pick_layout_name(&layout_names)?;
    let definition = protocol.get_layout_definition().clone();

    let keyboard = Keyboard::new(
        protocol,
        selected_layout_name.clone(),
        request.overlay_config,
        ui_wake,
    )
    .map_err(|e| format!("Failed to create keyboard: {e}"))?;

    Ok(ConnectedState {
        definition,
        layout_names,
        selected_layout_name,
        keyboard,
        reopen,
    })
}
