use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::Arc;

use crate::platform::web_serial::WebSerialTransport;
use crate::protocols::{DeviceError, DeviceEvent};

use super::common::{build_from_zmk_data, resolve_binding, ZmkData};

use zmk_studio_api::proto::zmk::behaviors::GetBehaviorDetailsResponse;
use zmk_studio_api::proto::zmk::core::LockState;
use zmk_studio_api::proto::zmk::keymap::{Keymap, PhysicalLayouts};
use zmk_studio_api::proto::zmk::studio;
use zmk_studio_api::{role_from_display_name, ResolvedLayer};

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

pub use super::driver::ZmkProtocol;

/// Connects to a ZMK Studio keyboard over Web Serial, performs handshake and keymap resolution.
pub async fn connect_web_zmk(
    transport: Arc<WebSerialTransport>,
    vid: u16,
    pid: u16,
    event_rx: mpsc::Receiver<DeviceEvent>,
    _hid_transport: Option<crate::platform::web_hid::WebHidTransport>,
) -> Result<ZmkProtocol, DeviceError> {
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

    let backend = Box::new(super::driver::WebZmkBackend::new(
        transport,
        behavior_id_by_role,
    ));

    Ok(ZmkProtocol::new_web(Arc::new(layout), backend, event_rx))
}
