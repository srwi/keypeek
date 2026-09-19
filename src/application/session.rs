use crate::domain::KeyboardDomain;
use crate::key_presenter::KeyPresenter;
use crate::key_spec::KeySpec;
use crate::protocols::{ActionFilter, DeviceEvent, KeyboardProtocol, WriteSupport};
use crate::ui_wake::UiWake;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
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
    command_tx: mpsc::Sender<KeymapCommand>,
    alive: Arc<AtomicBool>,
    write_support: WriteSupport,
    supports_live_layout_switching: bool,
    action_filter: Option<ActionFilter>,
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

        let (command_tx, command_rx) = mpsc::channel::<KeymapCommand>();
        let alive = Arc::new(AtomicBool::new(true));
        let protocol = Arc::new(Mutex::new(protocol));

        // 1. Live event consumer loop (drives hardware events into domain model)
        let domain_for_events = Arc::clone(&domain);
        let alive_for_events = Arc::clone(&alive);
        let ui_wake_events = ui_wake.clone();
        thread::spawn(move || {
            while let Ok(event) = event_rx.recv() {
                let needs_repaint = match event {
                    DeviceEvent::LayersChanged {
                        active_layers,
                        default_layers,
                    } => domain_for_events.on_layers_changed(
                        active_layers,
                        default_layers,
                        Instant::now(),
                    ),
                    DeviceEvent::KeyPressed { row, col, pressed } => {
                        domain_for_events.on_key_pressed(row, col, pressed, Instant::now())
                    }
                    DeviceEvent::Disconnected(_) => {
                        alive_for_events.store(false, Ordering::Relaxed);
                        ui_wake_events.request_repaint();
                        break;
                    }
                };

                if needs_repaint {
                    ui_wake_events.request_repaint();
                }
            }
            alive_for_events.store(false, Ordering::Relaxed);
            ui_wake_events.request_repaint();
        });

        // 2. Command execution loop (runs writes on dedicated worker)
        let domain_for_cmds = Arc::clone(&domain);
        let ui_wake_cmd = ui_wake;
        let presenter_cmd = presenter;
        let protocol_for_cmds = Arc::clone(&protocol);
        thread::spawn(move || {
            while let Ok(command) = command_rx.recv() {
                let mut protocol_guard = protocol_for_cmds.lock().unwrap();
                run_keymap_command(
                    protocol_guard.as_mut(),
                    command,
                    &layer_names,
                    &domain_for_cmds,
                    &ui_wake_cmd,
                    presenter_cmd.as_ref(),
                );
            }
        });

        Ok(Self {
            command_tx,
            alive,
            write_support,
            supports_live_layout_switching,
            action_filter,
        })
    }

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
        let _ = self.command_tx.send(KeymapCommand::ReleaseEditLock);
    }

    fn send_keymap_command(
        &self,
        build: impl FnOnce(mpsc::Sender<Result<(), String>>) -> KeymapCommand,
    ) -> mpsc::Receiver<Result<(), String>> {
        let (respond, receiver) = mpsc::channel();
        let command = build(respond);

        if let Err(send_error) = self.command_tx.send(command) {
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
}

/// Executes one command on the protocol. Runs on the dedicated command worker thread.
fn run_keymap_command(
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
