use crate::application::Keyboard;
use crate::domain::visibility::OverlayConfig;
use crate::protocols::{ConnectionSpec, DeviceError, KeyboardProtocol, Reopener};
use crate::ui_wake::UiWake;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;

pub struct ConnectionRequest {
    pub spec: ConnectionSpec,
    pub overlay_config: OverlayConfig,
    pub layout_name: Option<String>,
    pub reopen: Option<Arc<dyn Reopener>>,
}

impl ConnectionRequest {
    fn open_protocol(&self) -> Result<Box<dyn KeyboardProtocol>, DeviceError> {
        match &self.reopen {
            Some(reopener) => reopener.reopen(),
            None => crate::firmware::connect_protocol(&self.spec),
        }
    }

    fn pick_layout_name(&self, layout_names: &[String]) -> Result<String, DeviceError> {
        if layout_names.is_empty() {
            return Err(DeviceError::Protocol(
                "Device did not provide any layouts".to_string(),
            ));
        }

        if let Some(name) = &self.layout_name {
            if layout_names.contains(name) {
                return Ok(name.clone());
            }
        }

        Ok(layout_names[0].clone())
    }
}

pub struct ConnectedState {
    pub keyboard: Keyboard,
    pub reopen: Option<Arc<dyn Reopener>>,
    pub editor_profile: Arc<dyn crate::keymap_editor::EditorProfile>,
}

pub struct ConnectionTask {
    rx: mpsc::Receiver<Result<ConnectedState, DeviceError>>,
}

impl ConnectionTask {
    pub fn start(request: ConnectionRequest, ui_wake: UiWake) -> Self {
        let (tx, rx) = mpsc::channel();
        let execute = move || {
            let result = build_connected_state(request, ui_wake.clone());
            let _ = tx.send(result);
            ui_wake.request_repaint();
        };

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(execute);

        #[cfg(target_arch = "wasm32")]
        execute();

        Self { rx }
    }

    pub fn try_finish(&self) -> Option<Result<ConnectedState, DeviceError>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(DeviceError::Transport(
                "Background connection task failed".to_string(),
            ))),
        }
    }
}

pub fn build_connected_state(
    request: ConnectionRequest,
    ui_wake: UiWake,
) -> Result<ConnectedState, DeviceError> {
    let bundle = crate::firmware::bundle_for_spec(&request.spec);
    let editor_profile = bundle.create_profile();
    let presenter = bundle.create_presenter();

    let protocol = request.open_protocol()?;

    let reopen = protocol.reopener();
    let layout_names = protocol.get_layout_definition().get_layout_names();
    let selected_layout_name = request.pick_layout_name(&layout_names)?;

    let keyboard = Keyboard::new(
        protocol,
        selected_layout_name,
        request.overlay_config,
        ui_wake,
        presenter,
    )
    .map_err(|e| DeviceError::Protocol(format!("Failed to create keyboard: {e}")))?;

    Ok(ConnectedState {
        keyboard,
        reopen,
        editor_profile,
    })
}
