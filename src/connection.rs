use crate::keyboard::Keyboard;
use crate::protocols::zmk;
use crate::protocols::zmk_rpc;
use crate::protocols::{
    connect_protocol, parse_zmk_config, KeyboardDefinition, KeyboardProtocol, ZmkTransportConfig,
};
use crate::settings::ProtocolType;
use std::sync::mpsc::{self, TryRecvError};

pub struct ConnectionRequest {
    pub protocol_type: ProtocolType,
    pub protocol_config: String,
    pub timeout: u64,
    pub layout_name: Option<String>,
}

pub struct ConnectedState {
    pub definition: KeyboardDefinition,
    pub layout_names: Vec<String>,
    pub selected_layout_name: String,
    pub keyboard: Keyboard,
}

pub struct ConnectionTask {
    rx: mpsc::Receiver<Result<ConnectedState, String>>,
}

impl ConnectionTask {
    pub fn start(request: ConnectionRequest) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = build_connected_state(request);
            let _ = tx.send(result);
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

fn connect_zmk_live(protocol_config: &str) -> Result<Box<dyn KeyboardProtocol>, String> {
    let (vid, pid, transport) =
        parse_zmk_config(protocol_config).map_err(|e| format!("Invalid ZMK config: {e}"))?;

    let zmk_transport = match transport {
        ZmkTransportConfig::Serial(port_name) => zmk_rpc::ZmkTransport::SerialPort(port_name),
        ZmkTransportConfig::Ble(device_id) => zmk_rpc::ZmkTransport::BleDevice(device_id),
    };

    let zmk_data = zmk_rpc::fetch_zmk_data(&zmk_transport).map_err(|e| {
        if e.to_string() == "DEVICE_LOCKED" {
            "Device is locked. Please press the ZMK Studio unlock key combination on your keyboard, then click Connect again.".to_string()
        } else {
            format!("ZMK error: {e}")
        }
    })?;

    let protocol =
        zmk::ZmkProtocol::connect_live(vid, pid, &zmk_data).map_err(|e| format!("ZMK error: {e}"))?;
    Ok(Box::new(protocol))
}

fn select_layout_name(layout_names: &[String], requested: &Option<String>) -> Result<String, String> {
    if layout_names.is_empty() {
        return Err("Device did not provide any layouts".to_string());
    }

    if let Some(name) = requested {
        if layout_names.contains(name) {
            return Ok(name.clone());
        }
    }

    Ok(layout_names[0].clone())
}

pub fn build_connected_state(request: ConnectionRequest) -> Result<ConnectedState, String> {
    let protocol = match request.protocol_type {
        ProtocolType::Zmk => connect_zmk_live(&request.protocol_config)?,
        _ => connect_protocol(request.protocol_type, &request.protocol_config)
            .map_err(|e| format!("Failed to connect to device: {e}"))?,
    };

    let layout_names = protocol.get_layout_definition().get_layout_names();
    let selected_layout_name = select_layout_name(&layout_names, &request.layout_name)?;
    let definition = protocol.get_layout_definition().clone();

    let keyboard = Keyboard::new(protocol, selected_layout_name.clone(), request.timeout)
        .map_err(|e| format!("Failed to create keyboard: {e}"))?;

    Ok(ConnectedState {
        definition,
        layout_names,
        selected_layout_name,
        keyboard,
    })
}
