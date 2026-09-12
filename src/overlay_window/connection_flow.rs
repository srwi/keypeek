use super::state::AppConnectionState;
use super::OverlayApp;
use crate::connection::{ConnectedState, ConnectionRequest, ConnectionTask};
use crate::protocols::{ConnectionSpec, Reopener};
use std::sync::Arc;
use std::time::{Duration, Instant};

const RECONNECT_INTERVAL: Duration = Duration::from_secs(3);

fn layout_preference(name: &str) -> Option<String> {
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

impl OverlayApp {
    pub(super) fn select_device(&mut self, index: usize) {
        if self.connect.available_devices.get(index).is_some() {
            self.connect.selected_device_index = Some(index);
            self.session.layout_names.clear();
            self.session.active_layout_name.clear();
            self.session.draft_layout_name.clear();
            self.ui.settings_error = None;
        }
    }

    fn build_connection_spec(&self) -> Result<ConnectionSpec, String> {
        let selected_device = self
            .connect
            .selected_device_index
            .and_then(|i| self.connect.available_devices.get(i))
            .ok_or_else(|| "No device selected".to_string())?;

        let mut spec = selected_device.spec.clone();
        if selected_device.requires_layout_file {
            let path = self.connect.layout_file_path.trim();
            if path.is_empty() {
                return Err("Please provide a JSON config file path".to_string());
            }
            if let ConnectionSpec::Via { json_path } = &mut spec {
                *json_path = path.to_string();
            }
        }
        Ok(spec)
    }

    pub(super) fn apply_connected_state(&mut self, connected: ConnectedState) {
        self.session.layout_names = connected.layout_names;
        self.session.active_layout_name = connected.selected_layout_name.clone();
        self.session.draft_layout_name = connected.selected_layout_name;
        self.session.connected_definition = Some(connected.definition);
        self.session.reopen = connected.reopen;
        self.session.connection = AppConnectionState::Connected {
            keyboard: Arc::new(connected.keyboard),
            profile: connected.editor_profile,
        };
        self.session.ever_connected = true;
        self.ui.settings_error = None;
        self.ui.settings_warning = None;

        self.persist_settings();
    }

    pub(crate) fn persist_settings(&self) {
        if let Err(e) = self.settings.active.save() {
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

        let Some(selected_device) = self
            .connect
            .selected_device_index
            .and_then(|i| self.connect.available_devices.get(i))
        else {
            self.ui.settings_error = Some("No device selected".to_string());
            return;
        };

        if selected_device.requires_layout_file && self.connect.layout_file_path.trim().is_empty() {
            self.ui.file_dialog.pick_file();
            return;
        }

        self.begin_connect_with_current_draft();
    }

    fn begin_connect_with_current_draft(&mut self) {
        if self.connect.pending_connect.is_some() {
            return;
        }

        let spec = match self.build_connection_spec() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.ui.settings_error = Some(e);
                return;
            }
        };

        self.session.last_spec = Some(spec.clone());
        self.session.reopen = None;

        let layout_name = layout_preference(&self.session.draft_layout_name);
        self.spawn_connection(spec, layout_name, None);
        self.ui.settings_error = None;
    }

    fn spawn_connection(
        &mut self,
        spec: ConnectionSpec,
        layout_name: Option<String>,
        reopen: Option<Arc<dyn Reopener>>,
    ) {
        let request = ConnectionRequest {
            spec,
            overlay_config: self.overlay_config(),
            layout_name,
            reopen,
        };
        self.connect.pending_connect = Some(ConnectionTask::start(request, self.ui_wake.clone()));
    }

    fn schedule_reconnect(&mut self) {
        self.session.connection = AppConnectionState::Reconnecting {
            next_attempt_at: Instant::now() + RECONNECT_INTERVAL,
        };
    }

    /// Detects a dropped connection and drives background reconnect attempts, reusing
    /// the last successful spec. Called every frame.
    pub(super) fn maintain_connection(&mut self, ctx: &egui::Context) {
        if let AppConnectionState::Connected { keyboard, .. } = &self.session.connection {
            if !keyboard.is_alive() {
                self.close_editor();
                self.session.connection = AppConnectionState::Reconnecting {
                    next_attempt_at: Instant::now(),
                };
            }
        }

        let AppConnectionState::Reconnecting { next_attempt_at } = &self.session.connection else {
            return;
        };
        let next_attempt_at = *next_attempt_at;

        // An attempt is already in flight; poll_connect_result will resolve it.
        if self.connect.pending_connect.is_some() {
            return;
        }

        let now = Instant::now();
        if now < next_attempt_at {
            ctx.request_repaint_after(next_attempt_at - now);
            return;
        }

        let Some(spec) = self.session.last_spec.clone() else {
            self.session.connection = AppConnectionState::Disconnected;
            return;
        };

        let layout_name = layout_preference(&self.session.active_layout_name);
        let reopen = self.session.reopen.clone();
        self.spawn_connection(spec, layout_name, reopen);
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
                // A failed reconnect retries silently; only surface errors for user-initiated connects.
                if matches!(
                    self.session.connection,
                    AppConnectionState::Reconnecting { .. }
                ) {
                    eprintln!("Reconnect attempt failed: {e}");
                    self.schedule_reconnect();
                } else {
                    self.ui.settings_error = Some(e);
                }
            }
            None => {}
        }
    }
}
