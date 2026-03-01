use super::state::{AppConnectionState, ZmkTransportDraft};
use super::{OverlayApp, SETTINGS_FILE};
use crate::connection::{ConnectedState, ConnectionRequest, ConnectionTask};
use crate::device_discovery::DeviceKind;
use crate::protocols::{format_vid_pid, format_zmk_config, ZmkTransportConfig};
use crate::settings::ProtocolType;

impl OverlayApp {
    pub(super) fn select_device(&mut self, index: usize) {
        if let Some(device) = self.connect.available_devices.get(index) {
            self.connect.selected_device_index = Some(index);
            self.session.layout_names.clear();
            self.session.active_layout_name.clear();
            self.session.draft_layout_name.clear();

            let vid_pid = format_vid_pid(device.vid, device.pid);
            match device.kind {
                DeviceKind::Zmk => {
                    self.connect.protocol_type = ProtocolType::Zmk;
                    self.connect.json_path = vid_pid;
                    self.connect.zmk_transport = if let Some(device_id) = &device.ble_device_id {
                        ZmkTransportDraft::Ble {
                            device_id: Some(device_id.clone()),
                        }
                    } else if let Some(port_name) = &device.serial_port {
                        ZmkTransportDraft::Serial {
                            port_name: Some(port_name.clone()),
                        }
                    } else {
                        ZmkTransportDraft::Ble { device_id: None }
                    };
                }
                DeviceKind::Vial => {
                    self.connect.protocol_type = ProtocolType::Vial;
                    self.connect.json_path = vid_pid;
                }
                DeviceKind::Qmk => {
                    self.connect.protocol_type = ProtocolType::Via;
                    self.connect.json_path = String::new();
                }
            }
            self.ui.settings_error = None;
        }
    }

    fn build_protocol_config(&self) -> Result<String, String> {
        match self.connect.protocol_type {
            ProtocolType::Vial => Ok(self.connect.json_path.trim().to_string()),
            ProtocolType::Via => {
                let path = self.connect.json_path.trim();
                if path.is_empty() {
                    Err("Please provide a JSON config file path".to_string())
                } else {
                    Ok(path.to_string())
                }
            }
            ProtocolType::Zmk => {
                let (vid, pid) = crate::protocols::parse_vid_pid(self.connect.json_path.trim())
                    .map_err(|e| format!("Invalid ZMK VID:PID: {e}"))?;

                let transport = match &self.connect.zmk_transport {
                    ZmkTransportDraft::Serial { port_name } => {
                        let port = port_name
                            .as_ref()
                            .ok_or_else(|| "No serial port selected for ZMK".to_string())?;
                        ZmkTransportConfig::Serial(port.clone())
                    }
                    ZmkTransportDraft::Ble { device_id } => {
                        let id = device_id
                            .as_ref()
                            .ok_or_else(|| "No BLE device selected for ZMK".to_string())?;
                        ZmkTransportConfig::Ble(id.clone())
                    }
                };

                Ok(format_zmk_config(vid, pid, &transport))
            }
        }
    }

    pub(super) fn apply_connected_state(&mut self, connected: ConnectedState) {
        self.session.layout_names = connected.layout_names;
        self.session.active_layout_name = connected.selected_layout_name.clone();
        self.session.draft_layout_name = connected.selected_layout_name;
        self.session.connected_definition = Some(connected.definition);
        self.session.connection = AppConnectionState::Connected {
            keyboard: connected.keyboard,
        };
        self.session.ever_connected = true;
        self.ui.settings_error = None;
        self.ui.settings_warning = None;

        self.persist_settings();
    }

    pub(super) fn persist_settings(&self) {
        if let Err(e) = self.settings.active.save_to_file(SETTINGS_FILE) {
            eprintln!("Failed to save settings: {e}");
        }
    }

    pub(super) fn connect_from_ui(&mut self) {
        if matches!(
            self.session.connection,
            AppConnectionState::Connected { .. }
        ) {
            self.ui.settings_warning = Some(
                "Switching device/protocol/layout requires app restart in this version."
                    .to_string(),
            );
            return;
        }

        if self.connect.selected_device_index.is_none() {
            self.ui.settings_error = Some("No device selected".to_string());
            return;
        }

        if self.connect.protocol_type == ProtocolType::Via
            && self.connect.json_path.trim().is_empty()
        {
            self.ui.file_dialog.pick_file();
            return;
        }

        self.begin_connect_with_current_draft();
    }

    fn begin_connect_with_current_draft(&mut self) {
        if self.connect.pending_connect.is_some() {
            return;
        }

        let protocol_config = match self.build_protocol_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.ui.settings_error = Some(e);
                return;
            }
        };

        let request = ConnectionRequest {
            protocol_type: self.connect.protocol_type,
            protocol_config,
            timeout: self.settings.draft.timeout,
            layout_name: if self.session.draft_layout_name.is_empty() {
                None
            } else {
                Some(self.session.draft_layout_name.clone())
            },
        };

        self.connect.pending_connect = Some(ConnectionTask::start(request));
        self.ui.settings_error = None;
    }

    pub(super) fn poll_connect_result(&mut self) {
        let Some(task) = self.connect.pending_connect.as_ref() else {
            return;
        };

        match task.try_finish() {
            Some(Ok(connected)) => {
                self.connect.pending_connect = None;
                self.apply_connected_state(connected);
            }
            Some(Err(e)) => {
                self.connect.pending_connect = None;
                self.ui.settings_error = Some(e);
            }
            None => {}
        }
    }
}
