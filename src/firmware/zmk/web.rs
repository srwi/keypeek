use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::domain::layout::KeyboardDefinition;
use crate::key_spec::{KeySpec, KeymapSnapshot, LayerInfo};
use crate::platform::web_serial::WebSerialTransport;
use crate::protocols::{ActionFilter, DeviceError, DeviceEvent, KeyboardProtocol, WriteSupport};

use super::codec as zmk_codec;
use super::common::{build_from_zmk_data, resolve_binding, zmk_action_filter, ZmkData};

use zmk_studio_api::proto::zmk::behaviors::{
    BehaviorBindingParametersSet, GetBehaviorDetailsResponse,
};
use zmk_studio_api::proto::zmk::core::LockState;
use zmk_studio_api::proto::zmk::keymap::{BehaviorBinding, Keymap, PhysicalLayouts};
use zmk_studio_api::proto::zmk::studio;
use zmk_studio_api::{role_from_display_name, Behavior, BehaviorRole, ResolvedLayer};

/// Queries the ZMK Studio lock state over Web Serial.
async fn query_lock_state(transport: &WebSerialTransport) -> Result<LockState, DeviceError> {
    let req = studio::request::Subsystem::Core(zmk_studio_api::proto::zmk::core::Request {
        request_type: Some(
            zmk_studio_api::proto::zmk::core::request::RequestType::GetLockState(true),
        ),
    });
    let resp = transport.call(req).await?;
    match resp.subsystem {
        Some(studio::request_response::Subsystem::Core(core_resp)) => match core_resp.response_type
        {
            Some(zmk_studio_api::proto::zmk::core::response::ResponseType::GetLockState(raw)) => {
                LockState::try_from(raw)
                    .map_err(|_| DeviceError::Protocol(format!("Invalid lock state value: {raw}")))
            }
            _ => Err(DeviceError::Protocol(
                "Missing GetLockState response type".into(),
            )),
        },
        _ => Err(DeviceError::Protocol(
            "Unexpected subsystem in GetLockState response".into(),
        )),
    }
}

/// Queries physical layout definitions over Web Serial.
async fn query_physical_layouts(
    transport: &WebSerialTransport,
) -> Result<PhysicalLayouts, DeviceError> {
    let req = studio::request::Subsystem::Keymap(zmk_studio_api::proto::zmk::keymap::Request {
        request_type: Some(
            zmk_studio_api::proto::zmk::keymap::request::RequestType::GetPhysicalLayouts(true),
        ),
    });
    let resp = transport.call(req).await?;
    match resp.subsystem {
        Some(studio::request_response::Subsystem::Keymap(keymap_resp)) => {
            match keymap_resp.response_type {
                Some(
                    zmk_studio_api::proto::zmk::keymap::response::ResponseType::GetPhysicalLayouts(
                        layouts,
                    ),
                ) => Ok(layouts),
                _ => Err(DeviceError::Protocol(
                    "Missing GetPhysicalLayouts response type".into(),
                )),
            }
        }
        _ => Err(DeviceError::Protocol(
            "Unexpected subsystem in GetPhysicalLayouts response".into(),
        )),
    }
}

/// Queries keymap layers and bindings over Web Serial.
async fn query_keymap(transport: &WebSerialTransport) -> Result<Keymap, DeviceError> {
    let req = studio::request::Subsystem::Keymap(zmk_studio_api::proto::zmk::keymap::Request {
        request_type: Some(
            zmk_studio_api::proto::zmk::keymap::request::RequestType::GetKeymap(true),
        ),
    });
    let resp = transport.call(req).await?;
    match resp.subsystem {
        Some(studio::request_response::Subsystem::Keymap(keymap_resp)) => {
            match keymap_resp.response_type {
                Some(zmk_studio_api::proto::zmk::keymap::response::ResponseType::GetKeymap(
                    keymap,
                )) => Ok(keymap),
                _ => Err(DeviceError::Protocol(
                    "Missing GetKeymap response type".into(),
                )),
            }
        }
        _ => Err(DeviceError::Protocol(
            "Unexpected subsystem in GetKeymap response".into(),
        )),
    }
}

/// Queries all behavior IDs available on the device.
async fn query_all_behaviors(transport: &WebSerialTransport) -> Result<Vec<u32>, DeviceError> {
    let req =
        studio::request::Subsystem::Behaviors(zmk_studio_api::proto::zmk::behaviors::Request {
            request_type: Some(
                zmk_studio_api::proto::zmk::behaviors::request::RequestType::ListAllBehaviors(true),
            ),
        });
    let resp = transport.call(req).await?;
    match resp.subsystem {
        Some(studio::request_response::Subsystem::Behaviors(beh_resp)) => {
            match beh_resp.response_type {
                Some(
                    zmk_studio_api::proto::zmk::behaviors::response::ResponseType::ListAllBehaviors(
                        list,
                    ),
                ) => Ok(list.behaviors),
                _ => Err(DeviceError::Protocol(
                    "Missing ListAllBehaviors response type".into(),
                )),
            }
        }
        _ => Err(DeviceError::Protocol(
            "Unexpected subsystem in ListAllBehaviors response".into(),
        )),
    }
}

/// Queries behavior details (name and parameter metadata) for a given behavior ID.
async fn query_behavior_details(
    transport: &WebSerialTransport,
    behavior_id: u32,
) -> Result<GetBehaviorDetailsResponse, DeviceError> {
    let req =
        studio::request::Subsystem::Behaviors(zmk_studio_api::proto::zmk::behaviors::Request {
            request_type: Some(
                zmk_studio_api::proto::zmk::behaviors::request::RequestType::GetBehaviorDetails(
                    zmk_studio_api::proto::zmk::behaviors::GetBehaviorDetailsRequest {
                        behavior_id,
                    },
                ),
            ),
        });
    let resp = transport.call(req).await?;
    match resp.subsystem {
        Some(studio::request_response::Subsystem::Behaviors(beh_resp)) => match beh_resp
            .response_type
        {
            Some(
                zmk_studio_api::proto::zmk::behaviors::response::ResponseType::GetBehaviorDetails(
                    details,
                ),
            ) => Ok(details),
            _ => Err(DeviceError::Protocol(format!(
                "Missing GetBehaviorDetails response for behavior {behavior_id}"
            ))),
        },
        _ => Err(DeviceError::Protocol(
            "Unexpected subsystem in GetBehaviorDetails response".into(),
        )),
    }
}

/// Connected ZMK Studio keyboard communicating asynchronously over Web Serial and WebHID.
pub struct WebZmkProtocol {
    transport: Arc<WebSerialTransport>,
    definition: KeyboardDefinition,
    snapshot: Mutex<KeymapSnapshot>,
    supported_behaviors: HashSet<BehaviorRole>,
    behavior_metadata: HashMap<BehaviorRole, Vec<BehaviorBindingParametersSet>>,
    behavior_id_by_role: HashMap<BehaviorRole, u32>,
    event_rx: Mutex<Option<mpsc::Receiver<DeviceEvent>>>,
    _hid_transport: Option<crate::platform::web_hid::WebHidTransport>,
}

impl KeyboardProtocol for WebZmkProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn read_keymap(&self) -> Result<KeymapSnapshot, DeviceError> {
        Ok(self.snapshot.lock().unwrap().clone())
    }

    fn subscribe_events(&mut self) -> Result<mpsc::Receiver<DeviceEvent>, DeviceError> {
        self.event_rx
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| DeviceError::Protocol("Already subscribed to ZMK events".to_string()))
    }

    fn write_support(&self) -> WriteSupport {
        WriteSupport::Staged
    }

    fn set_key(
        &mut self,
        layer: &LayerInfo,
        layer_index: usize,
        row: usize,
        col: usize,
        spec: &KeySpec,
    ) -> Result<(), DeviceError> {
        let behavior = zmk_codec::keyspec_to_zmk(spec)?;
        if row != 0 {
            return Err(DeviceError::Unsupported(format!(
                "Invalid ZMK key position {row}:{col}"
            )));
        }

        let behavior_id = match behavior.role() {
            Some(role) => self
                .behavior_id_by_role
                .get(&role)
                .copied()
                .ok_or_else(|| {
                    DeviceError::Unsupported(format!("Missing role {role:?} on device"))
                })?,
            None => match behavior {
                Behavior::Custom { behavior_id, .. } => behavior_id,
                Behavior::Unknown { behavior_id, .. } => behavior_id as u32,
                _ => {
                    return Err(DeviceError::Unsupported(
                        "Cannot resolve behavior ID for key binding".into(),
                    ))
                }
            },
        };

        let (param1, param2) = behavior.raw_params();
        let binding = BehaviorBinding {
            behavior_id: behavior_id as i32,
            param1,
            param2,
        };

        let transport = Arc::clone(&self.transport);
        let layer_id = layer.id;
        let key_position = col as i32;

        wasm_bindgen_futures::spawn_local(async move {
            let req =
                studio::request::Subsystem::Keymap(zmk_studio_api::proto::zmk::keymap::Request {
                    request_type: Some(
                        zmk_studio_api::proto::zmk::keymap::request::RequestType::SetLayerBinding(
                            zmk_studio_api::proto::zmk::keymap::SetLayerBindingRequest {
                                layer_id,
                                key_position,
                                binding: Some(binding),
                            },
                        ),
                    ),
                });
            if let Err(e) = transport.call(req).await {
                log::error!("Failed to write ZMK key binding: {e:?}");
            }
        });

        // Update in-memory snapshot immediately
        let mut snapshot = self.snapshot.lock().unwrap();
        if let Some(layer_actions) = snapshot.actions.get_mut(layer_index) {
            if let Some(row_actions) = layer_actions.get_mut(row) {
                if let Some(cell) = row_actions.get_mut(col) {
                    *cell = Some(spec.clone());
                }
            }
        }

        Ok(())
    }

    fn save_keymap(&mut self) -> Result<(), DeviceError> {
        let transport = Arc::clone(&self.transport);
        wasm_bindgen_futures::spawn_local(async move {
            let req =
                studio::request::Subsystem::Keymap(zmk_studio_api::proto::zmk::keymap::Request {
                    request_type: Some(
                        zmk_studio_api::proto::zmk::keymap::request::RequestType::SaveChanges(true),
                    ),
                });
            if let Err(e) = transport.call(req).await {
                log::error!("Failed to save ZMK changes: {e:?}");
            }
        });
        Ok(())
    }

    fn acquire_edit_lock(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn release_edit_lock(&mut self) {}

    fn action_filter(&self) -> Option<ActionFilter> {
        zmk_action_filter(
            self.supported_behaviors.clone(),
            self.behavior_metadata.clone(),
        )
    }

    fn supports_live_layout_switching(&self) -> bool {
        true
    }
}

/// Connects to a ZMK Studio keyboard over Web Serial, performs handshake and keymap resolution.
pub async fn connect_web_zmk(
    transport: Arc<WebSerialTransport>,
    vid: u16,
    pid: u16,
    event_rx: mpsc::Receiver<DeviceEvent>,
    hid_transport: Option<crate::platform::web_hid::WebHidTransport>,
) -> Result<WebZmkProtocol, DeviceError> {
    let lock_state = query_lock_state(&transport).await?;
    if lock_state == LockState::ZmkStudioCoreLockStateLocked {
        return Err(DeviceError::DeviceLocked);
    }

    let physical_layouts = query_physical_layouts(&transport).await?;
    let keymap = query_keymap(&transport).await?;
    let behavior_ids = query_all_behaviors(&transport).await?;

    let mut supported_behaviors = HashSet::new();
    let mut behavior_id_by_role = HashMap::new();
    let mut behavior_role_by_id = HashMap::new();
    let mut behavior_metadata = HashMap::new();
    let mut custom_behavior_details = HashMap::new();

    for id in behavior_ids {
        if let Ok(details) = query_behavior_details(&transport, id).await {
            match role_from_display_name(&details.display_name) {
                Some(role) => {
                    supported_behaviors.insert(role);
                    behavior_id_by_role.insert(role, id);
                    behavior_role_by_id.insert(id, role);
                    behavior_metadata.insert(role, details.metadata);
                }
                None => {
                    custom_behavior_details.insert(id, details);
                }
            }
        }
    }

    let resolved_layers: Vec<ResolvedLayer> = keymap
        .layers
        .iter()
        .map(|layer| ResolvedLayer {
            id: layer.id,
            name: layer.name.clone(),
            bindings: layer
                .bindings
                .iter()
                .map(|b| {
                    let b_id = b.behavior_id as u32;
                    let role = behavior_role_by_id.get(&b_id).copied();
                    let custom = custom_behavior_details.get(&b_id);
                    resolve_binding(b, role, custom)
                })
                .collect(),
        })
        .collect();

    let zmk_data = ZmkData {
        physical_layouts,
        resolved_layers,
        supported_behaviors: supported_behaviors.clone(),
        behavior_metadata: behavior_metadata.clone(),
    };

    let layout = build_from_zmk_data(vid, pid, zmk_data)
        .map_err(|e| DeviceError::Protocol(format!("Failed to build ZMK layout: {e}")))?;

    Ok(WebZmkProtocol {
        transport,
        definition: layout.definition,
        snapshot: layout.snapshot,
        supported_behaviors,
        behavior_metadata,
        behavior_id_by_role,
        event_rx: Mutex::new(Some(event_rx)),
        _hid_transport: hid_transport,
    })
}
