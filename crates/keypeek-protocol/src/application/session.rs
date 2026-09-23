use keypeek_core::{KeyPresenter, KeySpec, KeyboardDomain};
use crate::protocols::{ActionFilter, DeviceEvent, KeyboardProtocol, WriteSupport};
use crate::ui_wake::UiWake;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use web_time::Instant;

/// A keymap command for the protocol, executed on the reader thread so writes
/// and reads never race the same HID handle.
pub enum KeymapCommand {
    /// Acquires an exclusive write lock ahead of key writes (ZMK).
    AcquireEditLock {
        respond: mpsc::Sender<Result<(), String>>,
    },
    SetKey {
        layer_index: usize,
        row: usize,
        col: usize,
        action: KeySpec,
        respond: mpsc::Sender<Result<(), String>>,
    },
    Save {
        respond: mpsc::Sender<Result<(), String>>,
    },
    /// Fire-and-forget; releases any active write lock.
    ReleaseEditLock,
}

/// Manages background worker threads, command queue, and protocol communication.
pub struct KeyboardSession {
    #[allow(dead_code)]
    command_tx: mpsc::Sender<KeymapCommand>,
    alive: Arc<AtomicBool>,
    write_support: WriteSupport,
    supports_live_layout_switching: bool,
    action_filter: Option<ActionFilter>,
    #[cfg(target_arch = "wasm32")]
    event_rx: Mutex<mpsc::Receiver<DeviceEvent>>,
    #[cfg(target_arch = "wasm32")]
    domain: Arc<KeyboardDomain>,
    #[cfg(target_arch = "wasm32")]
    protocol: Arc<Mutex<Box<dyn KeyboardProtocol>>>,
    #[cfg(target_arch = "wasm32")]
    layer_names: Vec<String>,
    #[cfg(target_arch = "wasm32")]
    presenter: Arc<dyn KeyPresenter>,
    #[cfg(target_arch = "wasm32")]
    ui_wake: UiWake,
}

impl KeyboardSession {
    pub fn start(
        mut protocol: Box<dyn KeyboardProtocol>,
        domain: Arc<KeyboardDomain>,
        layer_names: Vec<String>,
        presenter: Arc<dyn KeyPresenter>,
        ui_wake: UiWake,
    ) -> Result<Self, String> {
        let event_rx = protocol
            .subscribe_events()
            .map_err(|e| format!("Failed to subscribe to keyboard events: {e}"))?;

        let write_support = protocol.write_support();
        let supports_live_layout_switching = protocol.supports_live_layout_switching();
        let action_filter = protocol.action_filter();

        #[cfg(not(target_arch = "wasm32"))]
        let (command_tx, command_rx) = mpsc::channel::<KeymapCommand>();
        #[cfg(target_arch = "wasm32")]
        let (command_tx, _command_rx) = mpsc::channel::<KeymapCommand>();
        let alive = Arc::new(AtomicBool::new(true));
        let protocol = Arc::new(Mutex::new(protocol));

        #[cfg(not(target_arch = "wasm32"))]
        {
            // 1. Live event consumer loop (drives hardware events into domain model)
            let domain_for_events = Arc::clone(&domain);
            let alive_for_events = Arc::clone(&alive);
            let ui_wake_events = ui_wake.clone();
            thread::spawn(move || {
                while let Ok(event) = event_rx.recv() {
                    let is_disconnected = matches!(event, DeviceEvent::Disconnected(_));
                    if handle_device_event(
                        &domain_for_events,
                        &alive_for_events,
                        event,
                        Instant::now(),
                    ) {
                        ui_wake_events.request_repaint();
                    }
                    if is_disconnected {
                        break;
                    }
                }
                alive_for_events.store(false, Ordering::Relaxed);
                ui_wake_events.request_repaint();
            });

            // 2. Command execution loop (runs writes on dedicated worker)
            let domain_for_cmds = Arc::clone(&domain);
            let ui_wake_cmd = ui_wake.clone();
            let presenter_cmd = Arc::clone(&presenter);
            let protocol_for_cmds = Arc::clone(&protocol);
            let layer_names_for_cmds = layer_names.clone();
            thread::spawn(move || {
                while let Ok(command) = command_rx.recv() {
                    let mut protocol_guard = protocol_for_cmds.lock().unwrap();
                    run_keymap_command(
                        protocol_guard.as_mut(),
                        command,
                        &layer_names_for_cmds,
                        &domain_for_cmds,
                        &ui_wake_cmd,
                        presenter_cmd.as_ref(),
                    );
                }
            });
        }

        Ok(Self {
            command_tx,
            alive,
            write_support,
            supports_live_layout_switching,
            action_filter,
            #[cfg(target_arch = "wasm32")]
            event_rx: Mutex::new(event_rx),
            #[cfg(target_arch = "wasm32")]
            domain,
            #[cfg(target_arch = "wasm32")]
            protocol,
            #[cfg(target_arch = "wasm32")]
            layer_names,
            #[cfg(target_arch = "wasm32")]
            presenter,
            #[cfg(target_arch = "wasm32")]
            ui_wake,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn poll(&self) {
        if let Ok(rx) = self.event_rx.lock() {
            while let Ok(event) = rx.try_recv() {
                let is_disconnected = matches!(event, DeviceEvent::Disconnected(_));
                if handle_device_event(&self.domain, &self.alive, event, Instant::now()) {
                    self.ui_wake.request_repaint();
                }
                if is_disconnected {
                    self.alive.store(false, Ordering::Relaxed);
                    self.ui_wake.request_repaint();
                    break;
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll(&self) {}

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn write_support(&self) -> WriteSupport {
        self.write_support
    }

    pub fn supports_live_layout_switching(&self) -> bool {
        self.supports_live_layout_switching
    }

    pub fn is_action_supported(&self, action: &KeySpec) -> bool {
        self.action_filter
            .as_ref()
            .is_none_or(|filter| filter(action))
    }

    pub fn set_key(
        &self,
        layer_index: usize,
        row: usize,
        col: usize,
        action: KeySpec,
    ) -> mpsc::Receiver<Result<(), String>> {
        self.send_keymap_command(|respond| KeymapCommand::SetKey {
            layer_index,
            row,
            col,
            action,
            respond,
        })
    }

    pub fn save_keymap(&self) -> mpsc::Receiver<Result<(), String>> {
        self.send_keymap_command(|respond| KeymapCommand::Save { respond })
    }

    pub fn acquire_edit_lock(&self) -> mpsc::Receiver<Result<(), String>> {
        self.send_keymap_command(|respond| KeymapCommand::AcquireEditLock { respond })
    }

    pub fn release_edit_lock(&self) {
        let _ = self.send_command(KeymapCommand::ReleaseEditLock);
    }

    fn send_command(&self, command: KeymapCommand) -> Result<(), mpsc::SendError<KeymapCommand>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.command_tx.send(command)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut protocol_guard = self.protocol.lock().unwrap();
            run_keymap_command(
                protocol_guard.as_mut(),
                command,
                &self.layer_names,
                &self.domain,
                &self.ui_wake,
                self.presenter.as_ref(),
            );
            Ok(())
        }
    }

    fn send_keymap_command(
        &self,
        build: impl FnOnce(mpsc::Sender<Result<(), String>>) -> KeymapCommand,
    ) -> mpsc::Receiver<Result<(), String>> {
        let (respond, receiver) = mpsc::channel();
        let command = build(respond);

        if let Err(send_error) = self.send_command(command) {
            match send_error.0 {
                KeymapCommand::SetKey { respond, .. }
                | KeymapCommand::Save { respond }
                | KeymapCommand::AcquireEditLock { respond } => {
                    let _ = respond.send(Err("Connection lost".to_string()));
                }
                KeymapCommand::ReleaseEditLock => {}
            }
        }
        receiver
    }

    #[cfg(test)]
    pub fn new_test(
        command_tx: mpsc::Sender<KeymapCommand>,
        alive: Arc<AtomicBool>,
        write_support: WriteSupport,
    ) -> Self {
        Self {
            command_tx,
            alive,
            write_support,
            supports_live_layout_switching: false,
            action_filter: None,
        }
    }
}

/// Handles a single incoming device event, updating domain state and session liveness.
///
/// Returns `true` if the event causes state changes requiring a UI repaint.
pub fn handle_device_event(
    domain: &KeyboardDomain,
    alive: &AtomicBool,
    event: DeviceEvent,
    now: Instant,
) -> bool {
    match event {
        DeviceEvent::LayersChanged {
            active_layers,
            default_layers,
        } => domain.on_layers_changed(active_layers, default_layers, now),
        DeviceEvent::KeyPressed { row, col, pressed } => {
            domain.on_key_pressed(row, col, pressed, now)
        }
        DeviceEvent::Disconnected(_) => {
            alive.store(false, Ordering::Relaxed);
            true
        }
    }
}

/// Executes one command on the protocol and synchronizes keymap updates to domain state.
pub fn run_keymap_command(
    protocol: &mut dyn KeyboardProtocol,
    command: KeymapCommand,
    layer_names: &[String],
    domain: &KeyboardDomain,
    ui_wake: &UiWake,
    presenter: &dyn KeyPresenter,
) {
    match command {
        KeymapCommand::AcquireEditLock { respond } => {
            let result = protocol.acquire_edit_lock().map_err(|e| e.to_string());
            let _ = respond.send(result);
        }
        KeymapCommand::SetKey {
            layer_index,
            row,
            col,
            action,
            respond,
        } => {
            let layer_info = domain.layer_info(layer_index);
            let result = layer_info
                .ok_or_else(|| format!("Unknown layer index {layer_index}"))
                .and_then(|layer| {
                    protocol
                        .set_key(&layer, layer_index, row, col, &action)
                        .map_err(|e| e.to_string())
                });

            if result.is_ok() {
                let label = presenter.present_key(&action, layer_names);
                domain.update_binding(layer_index, row, col, action, label);
                ui_wake.request_repaint();
            }
            let _ = respond.send(result);
        }
        KeymapCommand::Save { respond } => {
            let result = protocol.save_keymap().map_err(|e| e.to_string());
            let _ = respond.send(result);
        }
        KeymapCommand::ReleaseEditLock => protocol.release_edit_lock(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keypeek_core::OverlayConfig;

    #[test]
    fn test_handle_device_event_layers_changed() {
        let domain = crate::test_utils::create_test_domain();
        let alive = AtomicBool::new(true);

        let repaint = handle_device_event(
            &domain,
            &alive,
            DeviceEvent::LayersChanged {
                active_layers: 2,
                default_layers: 1,
            },
            Instant::now(),
        );

        assert!(repaint);
        assert_eq!(domain.layer_state(), 2);
        assert!(alive.load(Ordering::Relaxed));
    }

    #[test]
    fn test_handle_device_event_key_pressed() {
        let domain = crate::test_utils::create_test_domain();
        domain.set_config(OverlayConfig {
            timeout_ms: 2000,
            activation_delay_ms: 0,
            visible_layers: u32::MAX,
        });
        let alive = AtomicBool::new(true);

        // Activate non-base layer so overlay is visible
        handle_device_event(
            &domain,
            &alive,
            DeviceEvent::LayersChanged {
                active_layers: 2,
                default_layers: 1,
            },
            Instant::now(),
        );

        let repaint_press = handle_device_event(
            &domain,
            &alive,
            DeviceEvent::KeyPressed {
                row: 0,
                col: 0,
                pressed: true,
            },
            Instant::now(),
        );
        assert!(repaint_press);
        assert!(domain.is_key_pressed(0, 0));

        let repaint_release = handle_device_event(
            &domain,
            &alive,
            DeviceEvent::KeyPressed {
                row: 0,
                col: 0,
                pressed: false,
            },
            Instant::now(),
        );
        assert!(repaint_release);
        assert!(!domain.is_key_pressed(0, 0));
    }

    #[test]
    fn test_handle_device_event_disconnected() {
        let domain = crate::test_utils::create_test_domain();
        let alive = AtomicBool::new(true);

        let repaint = handle_device_event(
            &domain,
            &alive,
            DeviceEvent::Disconnected("cable pulled".to_string()),
            Instant::now(),
        );

        assert!(repaint);
        assert!(!alive.load(Ordering::Relaxed));
    }

    #[test]
    fn test_run_keymap_command_acquire_lock_and_save() {
        let domain = crate::test_utils::create_test_domain();
        let mut test_protocol = crate::test_utils::TestProtocol::default();
        let ui_wake = UiWake::default();
        let presenter = crate::key_presenter::StandardKeyPresenter;

        let (tx, rx) = mpsc::channel();
        run_keymap_command(
            &mut test_protocol,
            KeymapCommand::AcquireEditLock { respond: tx },
            &[],
            &domain,
            &ui_wake,
            &presenter,
        );
        assert_eq!(rx.recv().unwrap(), Ok(()));

        let (tx_save, rx_save) = mpsc::channel();
        run_keymap_command(
            &mut test_protocol,
            KeymapCommand::Save { respond: tx_save },
            &[],
            &domain,
            &ui_wake,
            &presenter,
        );
        assert_eq!(rx_save.recv().unwrap(), Ok(()));
    }

    #[test]
    fn test_session_command_channel_disconnect() {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        drop(cmd_rx); // Disconnect receiver
        let session = KeyboardSession::new_test(
            cmd_tx,
            Arc::new(AtomicBool::new(true)),
            WriteSupport::Immediate,
        );

        let rx = session.acquire_edit_lock();
        assert_eq!(rx.recv().unwrap(), Err("Connection lost".to_string()));

        let rx_save = session.save_keymap();
        assert_eq!(rx_save.recv().unwrap(), Err("Connection lost".to_string()));
    }
}
