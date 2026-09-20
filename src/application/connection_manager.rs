use super::connection::{ConnectionRequest, ConnectionTask};
use super::device_discovery::DiscoveredDevice;
use super::keyboard::Keyboard;
use super::ui_wake::UiWake;
use crate::domain::visibility::OverlayConfig;
use crate::presentation::keymap_editor::EditorProfile;
use crate::protocols::{ConnectionSpec, Reopener};

use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;

const RECONNECT_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected {
        keyboard: Arc<Keyboard>,
        profile: Arc<dyn EditorProfile>,
    },
    Reconnecting {
        next_attempt_at: Instant,
    },
}

impl ConnectionStatus {
    pub fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected)
    }

    pub fn is_connecting(&self) -> bool {
        matches!(self, Self::Connecting)
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }

    pub fn is_reconnecting(&self) -> bool {
        matches!(self, Self::Reconnecting { .. })
    }

    pub fn is_locked(&self) -> bool {
        !self.is_disconnected()
    }
}

pub enum ConnectOutcome {
    Started,
    RequiresLayoutFile,
    AlreadyConnected,
    Failed(String),
}

pub enum ConnectionEvent {
    Connected,
    Disconnected,
    ConnectionFailed(String),
}

pub struct DeviceConnectionManager {
    available_devices: Vec<DiscoveredDevice>,
    selected_device_index: Option<usize>,
    layout_file_path: String,
    status: ConnectionStatus,
    pending_connect: Option<ConnectionTask>,
    last_spec: Option<ConnectionSpec>,
    reopen: Option<Arc<dyn Reopener>>,
    preferred_layout_name: Option<String>,
    ever_connected: bool,
    ui_wake: UiWake,
}

impl DeviceConnectionManager {
    pub fn new(available_devices: Vec<DiscoveredDevice>, ui_wake: UiWake) -> Self {
        Self {
            available_devices,
            selected_device_index: None,
            layout_file_path: String::new(),
            status: ConnectionStatus::Disconnected,
            pending_connect: None,
            last_spec: None,
            reopen: None,
            preferred_layout_name: None,
            ever_connected: false,
            ui_wake,
        }
    }

    pub fn available_devices(&self) -> &[DiscoveredDevice] {
        &self.available_devices
    }

    pub fn selected_device_index(&self) -> Option<usize> {
        self.selected_device_index
    }

    pub fn selected_device(&self) -> Option<&DiscoveredDevice> {
        self.selected_device_index
            .and_then(|i| self.available_devices.get(i))
    }

    pub fn select_device(&mut self, index: usize) {
        if self.available_devices.get(index).is_some() {
            self.selected_device_index = Some(index);
            self.preferred_layout_name = None;
        }
    }

    pub fn ui_wake(&self) -> &UiWake {
        &self.ui_wake
    }

    pub fn add_device(&mut self, device: DiscoveredDevice) -> usize {
        if let Some(pos) = self
            .available_devices
            .iter()
            .position(|d| d.vid == device.vid && d.pid == device.pid)
        {
            self.available_devices[pos] = device;
            pos
        } else {
            self.available_devices.push(device);
            self.available_devices.len() - 1
        }
    }

    pub fn set_layout_file_path(&mut self, path: String) {
        self.layout_file_path = path;
    }

    #[cfg(test)]
    pub fn is_disconnected(&self) -> bool {
        self.status.is_disconnected()
    }

    pub fn is_connecting(&self) -> bool {
        self.status.is_connecting()
    }

    pub fn is_connected(&self) -> bool {
        self.status.is_connected()
    }

    pub fn is_reconnecting(&self) -> bool {
        self.status.is_reconnecting()
    }

    pub fn is_locked(&self) -> bool {
        self.status.is_locked()
    }

    pub fn connected_keyboard(&self) -> Option<Arc<Keyboard>> {
        match &self.status {
            ConnectionStatus::Connected { keyboard, .. } => Some(Arc::clone(keyboard)),
            _ => None,
        }
    }

    pub fn connected_pair(&self) -> Option<(Arc<Keyboard>, Arc<dyn EditorProfile>)> {
        match &self.status {
            ConnectionStatus::Connected { keyboard, profile } => {
                Some((Arc::clone(keyboard), Arc::clone(profile)))
            }
            _ => None,
        }
    }

    pub fn ever_connected(&self) -> bool {
        self.ever_connected
    }

    #[cfg(test)]
    pub fn preferred_layout_name(&self) -> Option<&str> {
        self.preferred_layout_name.as_deref()
    }

    pub fn set_preferred_layout_name(&mut self, name: Option<String>) {
        self.preferred_layout_name = name;
    }

    pub fn connect(&mut self, overlay_config: OverlayConfig) -> ConnectOutcome {
        if self.is_connected() {
            return ConnectOutcome::AlreadyConnected;
        }

        if self.pending_connect.is_some() {
            return ConnectOutcome::Started;
        }

        let Some(selected_device) = self.selected_device() else {
            return ConnectOutcome::Failed("No device selected".to_string());
        };

        let mut spec = selected_device.spec.clone();
        if selected_device.requires_layout_file {
            let path = self.layout_file_path.trim();
            if path.is_empty() {
                return ConnectOutcome::RequiresLayoutFile;
            }
            if let ConnectionSpec::Via { json_path } = &mut spec {
                *json_path = path.to_string();
            }
        }

        self.last_spec = Some(spec.clone());
        self.reopen = None;
        self.status = ConnectionStatus::Connecting;

        let request = ConnectionRequest {
            spec,
            overlay_config,
            layout_name: self.preferred_layout_name.clone(),
            reopen: None,
        };
        self.pending_connect = Some(ConnectionTask::start(request, self.ui_wake.clone()));
        ConnectOutcome::Started
    }

    pub fn update(
        &mut self,
        overlay_config: OverlayConfig,
        schedule_repaint: impl Fn(Duration),
    ) -> Option<ConnectionEvent> {
        let mut event = None;

        // 1. Detect dropped connection
        if let ConnectionStatus::Connected { keyboard, .. } = &self.status {
            if !keyboard.is_alive() {
                self.status = ConnectionStatus::Reconnecting {
                    next_attempt_at: Instant::now(),
                };
                event = Some(ConnectionEvent::Disconnected);
            }
        }

        // 2. Poll pending connection task
        if let Some(task) = &self.pending_connect {
            match task.try_finish() {
                Some(Ok(connected)) => {
                    self.pending_connect = None;
                    self.preferred_layout_name = Some(connected.keyboard.active_layout_name());
                    self.reopen = connected.reopen;
                    let keyboard = Arc::new(connected.keyboard);
                    self.status = ConnectionStatus::Connected {
                        keyboard: Arc::clone(&keyboard),
                        profile: connected.editor_profile,
                    };
                    self.ever_connected = true;
                    event = Some(ConnectionEvent::Connected);
                }
                Some(Err(e)) => {
                    self.pending_connect = None;
                    if matches!(self.status, ConnectionStatus::Reconnecting { .. }) {
                        eprintln!("Reconnect attempt failed: {e}");
                        self.status = ConnectionStatus::Reconnecting {
                            next_attempt_at: Instant::now() + RECONNECT_INTERVAL,
                        };
                    } else {
                        self.status = ConnectionStatus::Disconnected;
                        event = Some(ConnectionEvent::ConnectionFailed(e));
                    }
                }
                None => {}
            }
        }

        // 3. Drive reconnection if due
        if let ConnectionStatus::Reconnecting { next_attempt_at } = self.status {
            if self.pending_connect.is_none() {
                let now = Instant::now();
                if now < next_attempt_at {
                    schedule_repaint(next_attempt_at - now);
                } else if let Some(spec) = self.last_spec.clone() {
                    let request = ConnectionRequest {
                        spec,
                        overlay_config,
                        layout_name: self.preferred_layout_name.clone(),
                        reopen: self.reopen.clone(),
                    };
                    self.pending_connect =
                        Some(ConnectionTask::start(request, self.ui_wake.clone()));
                } else {
                    self.status = ConnectionStatus::Disconnected;
                }
            }
        }

        event
    }

    pub fn set_connected(
        &mut self,
        keyboard: Arc<Keyboard>,
        profile: Arc<dyn EditorProfile>,
        reopen: Option<Arc<dyn Reopener>>,
    ) {
        self.pending_connect = None;
        self.preferred_layout_name = Some(keyboard.active_layout_name());
        self.reopen = reopen;
        self.status = ConnectionStatus::Connected { keyboard, profile };
        self.ever_connected = true;
    }

    pub fn disconnect(&mut self) {
        self.status = ConnectionStatus::Disconnected;
        self.pending_connect = None;
        self.reopen = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_wake() -> UiWake {
        UiWake::new(Arc::new(|| {}))
    }

    fn sample_device(requires_file: bool) -> DiscoveredDevice {
        DiscoveredDevice {
            base_name: "Test Keyboard".to_string(),
            vid: 0x1234,
            pid: 0x5678,
            driver_id: "test",
            protocol_label: "TestProto",
            requires_layout_file: requires_file,
            spec: crate::test_utils::test_spec(),
        }
    }

    #[test]
    fn test_initial_state() {
        let mgr = DeviceConnectionManager::new(vec![sample_device(false)], dummy_wake());
        assert!(mgr.is_disconnected());
        assert!(!mgr.is_connected());
        assert!(!mgr.is_connecting());
        assert!(!mgr.is_reconnecting());
        assert!(!mgr.is_locked());
        assert!(!mgr.ever_connected());
        assert_eq!(mgr.selected_device_index(), None);
        assert_eq!(mgr.available_devices().len(), 1);
    }

    #[test]
    fn test_select_device() {
        let mut mgr = DeviceConnectionManager::new(
            vec![sample_device(false), sample_device(true)],
            dummy_wake(),
        );
        mgr.set_preferred_layout_name(Some("ISO".to_string()));
        assert_eq!(mgr.preferred_layout_name(), Some("ISO"));

        mgr.select_device(1);
        assert_eq!(mgr.selected_device_index(), Some(1));
        // Changing selected device resets preferred layout name
        assert_eq!(mgr.preferred_layout_name(), None);
    }

    #[test]
    fn test_connect_validation() {
        let mut mgr = DeviceConnectionManager::new(vec![sample_device(true)], dummy_wake());
        let config = OverlayConfig {
            timeout_ms: 1000,
            activation_delay_ms: 0,
            visible_layers: 1,
        };

        // No device selected
        assert!(matches!(
            mgr.connect(config),
            ConnectOutcome::Failed(msg) if msg.contains("No device selected")
        ));

        mgr.select_device(0);
        // Requires layout file, but path is empty
        assert!(matches!(
            mgr.connect(config),
            ConnectOutcome::RequiresLayoutFile
        ));

        mgr.set_layout_file_path("path/to/layout.json".to_string());
        assert!(matches!(mgr.connect(config), ConnectOutcome::Started));
        assert!(mgr.is_connecting());
        assert!(mgr.is_locked());
    }

    #[test]
    fn test_add_device() {
        let mut mgr = DeviceConnectionManager::new(vec![sample_device(false)], dummy_wake());
        assert_eq!(mgr.available_devices().len(), 1);

        let mut new_dev = sample_device(false);
        new_dev.vid = 0xABCD;
        new_dev.pid = 0x1234;
        new_dev.base_name = "New Device".to_string();

        let idx = mgr.add_device(new_dev.clone());
        assert_eq!(idx, 1);
        assert_eq!(mgr.available_devices().len(), 2);
        assert_eq!(mgr.available_devices()[1].base_name, "New Device");

        // Adding device with same vid/pid updates in place
        new_dev.base_name = "Updated Device".to_string();
        let idx2 = mgr.add_device(new_dev);
        assert_eq!(idx2, 1);
        assert_eq!(mgr.available_devices().len(), 2);
        assert_eq!(mgr.available_devices()[1].base_name, "Updated Device");
    }
}
