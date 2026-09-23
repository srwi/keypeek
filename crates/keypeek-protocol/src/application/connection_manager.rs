use super::connection::{ConnectionRequest, ConnectionTask};
use super::device_discovery::DiscoveredDevice;
use super::keyboard::Keyboard;
use super::ui_wake::UiWake;
use keypeek_core::keymap_editor::EditorProfile;
use keypeek_core::OverlayConfig;
use crate::protocols::{ConnectionSpec, DeviceError, Reopener};

use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;

const RECONNECT_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub enum ConnectionState {
    Disconnected,
    Scanning,
    Connecting,
    RequiresLayoutFile {
        device: DiscoveredDevice,
    },
    Locked {
        device: DiscoveredDevice,
    },
    Connected {
        keyboard: Arc<Keyboard>,
        profile: Arc<dyn EditorProfile>,
    },
    Reconnecting {
        attempt: usize,
        next_try: Instant,
    },
    Error(String),
}

pub type ConnectionStatus = ConnectionState;

impl PartialEq for ConnectionState {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Disconnected, Self::Disconnected) => true,
            (Self::Scanning, Self::Scanning) => true,
            (Self::Connecting, Self::Connecting) => true,
            (Self::RequiresLayoutFile { device: d1 }, Self::RequiresLayoutFile { device: d2 }) => {
                d1 == d2
            }
            (Self::Locked { device: d1 }, Self::Locked { device: d2 }) => d1 == d2,
            (
                Self::Connected {
                    keyboard: k1,
                    profile: p1,
                },
                Self::Connected {
                    keyboard: k2,
                    profile: p2,
                },
            ) => Arc::ptr_eq(k1, k2) && Arc::ptr_eq(p1, p2),
            (
                Self::Reconnecting {
                    attempt: a1,
                    next_try: t1,
                },
                Self::Reconnecting {
                    attempt: a2,
                    next_try: t2,
                },
            ) => a1 == a2 && t1 == t2,
            (Self::Error(e1), Self::Error(e2)) => e1 == e2,
            _ => false,
        }
    }
}

impl fmt::Debug for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Scanning => write!(f, "Scanning"),
            Self::Connecting => write!(f, "Connecting"),
            Self::RequiresLayoutFile { device } => f
                .debug_struct("RequiresLayoutFile")
                .field("device", device)
                .finish(),
            Self::Locked { device } => f.debug_struct("Locked").field("device", device).finish(),
            Self::Connected { keyboard, .. } => f
                .debug_struct("Connected")
                .field("active_layout", &keyboard.active_layout_name())
                .finish(),
            Self::Reconnecting { attempt, next_try } => f
                .debug_struct("Reconnecting")
                .field("attempt", attempt)
                .field("next_try", next_try)
                .finish(),
            Self::Error(err) => f.debug_tuple("Error").field(err).finish(),
        }
    }
}

impl ConnectionState {
    pub fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected)
    }

    pub fn is_scanning(&self) -> bool {
        matches!(self, Self::Scanning)
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
        matches!(self, Self::Locked { .. })
    }

    pub fn is_requires_layout_file(&self) -> bool {
        matches!(self, Self::RequiresLayoutFile { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Returns true if a connection attempt or active session is currently engaged.
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Connecting | Self::Connected { .. } | Self::Reconnecting { .. }
        )
    }

    pub fn connected_keyboard(&self) -> Option<Arc<Keyboard>> {
        match self {
            Self::Connected { keyboard, .. } => Some(Arc::clone(keyboard)),
            _ => None,
        }
    }

    pub fn connected_pair(&self) -> Option<(Arc<Keyboard>, Arc<dyn EditorProfile>)> {
        match self {
            Self::Connected { keyboard, profile } => {
                Some((Arc::clone(keyboard), Arc::clone(profile)))
            }
            _ => None,
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error(msg) => Some(msg.as_str()),
            _ => None,
        }
    }

    pub fn connect_button_label(&self) -> String {
        match self {
            Self::Reconnecting { attempt, .. } => format!("Reconnecting ({attempt})..."),
            Self::Connecting => "Connecting...".to_string(),
            Self::Locked { .. } => "Locked".to_string(),
            _ => "Connect".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectOutcome {
    Started,
    RequiresLayoutFile,
    AlreadyConnected,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionEvent {
    Connected,
    Disconnected,
    ConnectionFailed(String),
    DeviceLocked(DiscoveredDevice),
    RequiresLayoutFile(DiscoveredDevice),
}

pub struct DeviceConnectionManager {
    available_devices: Vec<DiscoveredDevice>,
    selected_device_index: Option<usize>,
    layout_file_path: String,
    state: ConnectionState,
    pending_connect: Option<ConnectionTask>,
    last_spec: Option<ConnectionSpec>,
    reopen: Option<Arc<dyn Reopener>>,
    preferred_layout_name: Option<String>,
    ever_connected: bool,
    reconnect_attempt: usize,
    ui_wake: UiWake,
}

impl DeviceConnectionManager {
    pub fn new(available_devices: Vec<DiscoveredDevice>, ui_wake: UiWake) -> Self {
        Self {
            available_devices,
            selected_device_index: None,
            layout_file_path: String::new(),
            state: ConnectionState::Disconnected,
            pending_connect: None,
            last_spec: None,
            reopen: None,
            preferred_layout_name: None,
            ever_connected: false,
            reconnect_attempt: 0,
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

    pub fn set_devices(&mut self, devices: Vec<DiscoveredDevice>) {
        self.available_devices = devices;
        if self.state.is_scanning() {
            self.state = ConnectionState::Disconnected;
        }
    }

    pub fn set_layout_file_path(&mut self, path: String) {
        self.layout_file_path = path;
    }

    pub fn layout_file_path(&self) -> &str {
        &self.layout_file_path
    }

    pub fn state(&self) -> &ConnectionState {
        &self.state
    }

    pub fn status(&self) -> &ConnectionState {
        &self.state
    }

    pub fn is_disconnected(&self) -> bool {
        self.state.is_disconnected()
    }

    pub fn is_scanning(&self) -> bool {
        self.state.is_scanning()
    }

    pub fn is_connecting(&self) -> bool {
        self.state.is_connecting()
    }

    pub fn is_connected(&self) -> bool {
        self.state.is_connected()
    }

    pub fn is_reconnecting(&self) -> bool {
        self.state.is_reconnecting()
    }

    pub fn is_locked(&self) -> bool {
        self.state.is_locked()
    }

    pub fn can_select_device(&self) -> bool {
        matches!(
            self.state,
            ConnectionState::Disconnected
                | ConnectionState::Scanning
                | ConnectionState::Error(_)
                | ConnectionState::Locked { .. }
        )
    }

    pub fn reconnect_attempt(&self) -> usize {
        self.reconnect_attempt
    }

    pub fn connected_keyboard(&self) -> Option<Arc<Keyboard>> {
        self.state.connected_keyboard()
    }

    pub fn connected_pair(&self) -> Option<(Arc<Keyboard>, Arc<dyn EditorProfile>)> {
        self.state.connected_pair()
    }

    pub fn ever_connected(&self) -> bool {
        self.ever_connected
    }

    pub fn preferred_layout_name(&self) -> Option<&str> {
        self.preferred_layout_name.as_deref()
    }

    pub fn set_preferred_layout_name(&mut self, name: Option<String>) {
        self.preferred_layout_name = name;
    }

    pub fn set_scanning(&mut self) {
        self.state = ConnectionState::Scanning;
    }

    pub fn set_connecting(&mut self) {
        self.state = ConnectionState::Connecting;
    }

    pub fn set_requires_layout_file(&mut self, device: DiscoveredDevice) {
        self.state = ConnectionState::RequiresLayoutFile { device };
    }

    pub fn set_locked(&mut self, device: DiscoveredDevice) {
        self.state = ConnectionState::Locked { device };
    }

    pub fn set_error(&mut self, error: String) {
        self.state = ConnectionState::Error(error);
    }

    pub fn clear_error(&mut self) {
        if matches!(self.state, ConnectionState::Error(_)) {
            self.state = ConnectionState::Disconnected;
        }
    }

    pub fn connect(&mut self, overlay_config: OverlayConfig) -> ConnectOutcome {
        if self.is_connected() {
            return ConnectOutcome::AlreadyConnected;
        }

        if self.pending_connect.is_some() {
            return ConnectOutcome::Started;
        }

        let Some(selected_device) = self.selected_device() else {
            self.state = ConnectionState::Error("No device selected".to_string());
            return ConnectOutcome::Failed("No device selected".to_string());
        };

        let mut spec = selected_device.spec.clone();
        if selected_device.requires_layout_file {
            let path = self.layout_file_path.trim();
            if path.is_empty() {
                let dev = selected_device.clone();
                self.state = ConnectionState::RequiresLayoutFile { device: dev };
                return ConnectOutcome::RequiresLayoutFile;
            }
            if let ConnectionSpec::Via { json_path } = &mut spec {
                *json_path = path.to_string();
            }
        }

        self.last_spec = Some(spec.clone());
        self.reopen = None;
        self.reconnect_attempt = 0;
        self.state = ConnectionState::Connecting;

        let request = ConnectionRequest {
            spec,
            overlay_config,
            layout_name: self.preferred_layout_name.clone(),
            reopen: None,
        };
        self.pending_connect = Some(ConnectionTask::start(request, self.ui_wake.clone()));
        ConnectOutcome::Started
    }

    pub fn selected_or_fallback_device(&self) -> DiscoveredDevice {
        self.selected_device().cloned().unwrap_or_else(|| {
            let mut dev = DiscoveredDevice::anonymous();
            if let Some(spec) = self.last_spec.clone() {
                dev.spec = spec;
            }
            dev
        })
    }

    pub fn handle_device_error(&mut self, error: &DeviceError) -> ConnectionEvent {
        match error {
            DeviceError::DeviceLocked => {
                let device = self.selected_or_fallback_device();
                self.state = ConnectionState::Locked {
                    device: device.clone(),
                };
                ConnectionEvent::DeviceLocked(device)
            }
            other => {
                let err_str = other.to_string();
                self.state = ConnectionState::Error(err_str.clone());
                ConnectionEvent::ConnectionFailed(err_str)
            }
        }
    }

    pub fn update(
        &mut self,
        overlay_config: OverlayConfig,
        mut schedule_repaint: impl FnMut(Duration),
    ) -> Option<ConnectionEvent> {
        let mut event = None;

        // 1. Detect dropped connection
        if let ConnectionState::Connected { keyboard, .. } = &self.state {
            if !keyboard.is_alive() {
                self.reconnect_attempt = 1;
                self.state = ConnectionState::Reconnecting {
                    attempt: 1,
                    next_try: Instant::now(),
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
                    self.reconnect_attempt = 0;
                    let keyboard = Arc::new(connected.keyboard);
                    self.state = ConnectionState::Connected {
                        keyboard: Arc::clone(&keyboard),
                        profile: connected.editor_profile,
                    };
                    self.ever_connected = true;
                    event = Some(ConnectionEvent::Connected);
                }
                Some(Err(e)) => {
                    self.pending_connect = None;
                    if matches!(self.state, ConnectionState::Reconnecting { .. }) {
                        eprintln!("Reconnect attempt failed: {e}");
                        self.reconnect_attempt += 1;
                        let next_try = Instant::now() + RECONNECT_INTERVAL;
                        self.state = ConnectionState::Reconnecting {
                            attempt: self.reconnect_attempt,
                            next_try,
                        };
                    } else {
                        event = Some(self.handle_device_error(&e));
                    }
                }
                None => {}
            }
        }

        // 3. Drive reconnection if due
        if let ConnectionState::Reconnecting { next_try, .. } = self.state {
            if self.pending_connect.is_none() {
                let now = Instant::now();
                if now < next_try {
                    schedule_repaint(next_try - now);
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
                    self.state = ConnectionState::Disconnected;
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
        self.reconnect_attempt = 0;
        self.state = ConnectionState::Connected { keyboard, profile };
        self.ever_connected = true;
    }

    pub fn disconnect(&mut self) {
        self.state = ConnectionState::Disconnected;
        self.pending_connect = None;
        self.reopen = None;
        self.reconnect_attempt = 0;
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
        assert_eq!(*mgr.state(), ConnectionState::Disconnected);
        assert!(mgr.is_disconnected());
        assert!(!mgr.is_connected());
        assert!(!mgr.is_connecting());
        assert!(!mgr.is_reconnecting());
        assert!(!mgr.is_locked());
        assert!(!mgr.is_scanning());
        assert!(mgr.can_select_device());
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
        assert_eq!(mgr.preferred_layout_name(), None);
    }

    #[test]
    fn test_scanning_state_transitions() {
        let mut mgr = DeviceConnectionManager::new(Vec::new(), dummy_wake());
        assert!(mgr.is_disconnected());

        mgr.set_scanning();
        assert!(mgr.is_scanning());
        assert_eq!(*mgr.state(), ConnectionState::Scanning);
        assert!(mgr.can_select_device());

        mgr.set_devices(vec![sample_device(false)]);
        assert!(mgr.is_disconnected());
        assert_eq!(mgr.available_devices().len(), 1);
    }

    #[test]
    fn test_layout_file_prompt_transition() {
        let mut mgr = DeviceConnectionManager::new(vec![sample_device(true)], dummy_wake());
        let config = OverlayConfig {
            timeout_ms: 1000,
            activation_delay_ms: 0,
            visible_layers: 1,
        };

        // No device selected -> Error state
        assert!(matches!(
            mgr.connect(config),
            ConnectOutcome::Failed(msg) if msg.contains("No device selected")
        ));
        assert!(mgr.state().is_error());

        mgr.select_device(0);
        // Requires layout file, but path is empty -> RequiresLayoutFile state
        assert_eq!(mgr.connect(config), ConnectOutcome::RequiresLayoutFile);
        assert!(mgr.state().is_requires_layout_file());
        match mgr.state() {
            ConnectionState::RequiresLayoutFile { device } => {
                assert_eq!(device.base_name, "Test Keyboard");
            }
            _ => panic!("Expected RequiresLayoutFile state"),
        }

        // Setting layout file path enables successful start
        mgr.set_layout_file_path("path/to/layout.json".to_string());
        assert_eq!(mgr.connect(config), ConnectOutcome::Started);
        assert!(mgr.is_connecting());
        assert!(!mgr.can_select_device());
    }

    #[test]
    fn test_lock_state_transitions() {
        let dev = sample_device(false);
        let mut mgr = DeviceConnectionManager::new(vec![dev.clone()], dummy_wake());

        mgr.set_locked(dev.clone());
        assert!(mgr.is_locked());
        assert_eq!(*mgr.state(), ConnectionState::Locked { device: dev });
        // Locked devices still permit switching to another device
        assert!(mgr.can_select_device());

        mgr.disconnect();
        assert!(mgr.is_disconnected());
        assert!(!mgr.is_locked());
    }

    #[test]
    fn test_error_state_and_clear() {
        let mut mgr = DeviceConnectionManager::new(Vec::new(), dummy_wake());
        mgr.set_error("USB failure".to_string());
        assert!(mgr.state().is_error());
        assert_eq!(mgr.state().error_message(), Some("USB failure"));

        mgr.clear_error();
        assert!(mgr.is_disconnected());
        assert_eq!(mgr.state().error_message(), None);
    }

    #[test]
    fn test_reconnect_backoff_transitions() {
        let mut mgr = DeviceConnectionManager::new(vec![sample_device(false)], dummy_wake());
        let keyboard = Arc::new(crate::test_utils::create_test_keyboard());
        let bundle = crate::firmware::bundle_for_spec(&crate::test_utils::test_spec());
        let profile = bundle.create_profile();

        mgr.set_connected(Arc::clone(&keyboard), profile, None);
        assert!(mgr.is_connected());
        assert_eq!(mgr.reconnect_attempt(), 0);

        // When keyboard session stops / drops alive, update() detects and triggers Reconnecting
        // For test, we trigger reconnecting directly or through update
        let config = OverlayConfig {
            timeout_ms: 1000,
            activation_delay_ms: 0,
            visible_layers: 1,
        };

        // Keyboard mock session without active listener is alive, so manually set reconnecting:
        mgr.state = ConnectionState::Reconnecting {
            attempt: 1,
            next_try: Instant::now() + Duration::from_secs(10),
        };
        assert!(mgr.is_reconnecting());
        assert!(!mgr.can_select_device());

        let mut repainted = false;
        let event = mgr.update(config, |_| {
            repainted = true;
        });
        assert!(event.is_none());
        assert!(repainted);

        mgr.disconnect();
        assert!(mgr.is_disconnected());
        assert_eq!(mgr.reconnect_attempt(), 0);
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

        new_dev.base_name = "Updated Device".to_string();
        let idx2 = mgr.add_device(new_dev);
        assert_eq!(idx2, 1);
        assert_eq!(mgr.available_devices().len(), 2);
        assert_eq!(mgr.available_devices()[1].base_name, "Updated Device");
    }
}
