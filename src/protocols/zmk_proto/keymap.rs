// Pre-generated from proto/zmk/keymap.proto
// Copyright (c) 2024 The ZMK Contributors — SPDX-License-Identifier: MIT

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    #[prost(
        oneof = "request::RequestType",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12"
    )]
    pub request_type: ::core::option::Option<request::RequestType>,
}

pub mod request {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum RequestType {
        #[prost(bool, tag = "1")]
        GetKeymap(bool),
        #[prost(message, tag = "2")]
        SetLayerBinding(super::SetLayerBindingRequest),
        #[prost(bool, tag = "3")]
        CheckUnsavedChanges(bool),
        #[prost(bool, tag = "4")]
        SaveChanges(bool),
        #[prost(bool, tag = "5")]
        DiscardChanges(bool),
        #[prost(bool, tag = "6")]
        GetPhysicalLayouts(bool),
        #[prost(uint32, tag = "7")]
        SetActivePhysicalLayout(u32),
        #[prost(message, tag = "8")]
        MoveLayer(super::MoveLayerRequest),
        #[prost(message, tag = "9")]
        AddLayer(super::AddLayerRequest),
        #[prost(message, tag = "10")]
        RemoveLayer(super::RemoveLayerRequest),
        #[prost(message, tag = "11")]
        RestoreLayer(super::RestoreLayerRequest),
        #[prost(message, tag = "12")]
        SetLayerProps(super::SetLayerPropsRequest),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(
        oneof = "response::ResponseType",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12"
    )]
    pub response_type: ::core::option::Option<response::ResponseType>,
}

pub mod response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ResponseType {
        #[prost(message, tag = "1")]
        GetKeymap(super::Keymap),
        #[prost(enumeration = "super::SetLayerBindingResponse", tag = "2")]
        SetLayerBinding(i32),
        #[prost(bool, tag = "3")]
        CheckUnsavedChanges(bool),
        #[prost(message, tag = "4")]
        SaveChanges(super::SaveChangesResponse),
        #[prost(bool, tag = "5")]
        DiscardChanges(bool),
        #[prost(message, tag = "6")]
        GetPhysicalLayouts(super::PhysicalLayouts),
        #[prost(message, tag = "7")]
        SetActivePhysicalLayout(super::SetActivePhysicalLayoutResponse),
        #[prost(message, tag = "8")]
        MoveLayer(super::MoveLayerResponse),
        #[prost(message, tag = "9")]
        AddLayer(super::AddLayerResponse),
        #[prost(message, tag = "10")]
        RemoveLayer(super::RemoveLayerResponse),
        #[prost(message, tag = "11")]
        RestoreLayer(super::RestoreLayerResponse),
        #[prost(enumeration = "super::SetLayerPropsResponse", tag = "12")]
        SetLayerProps(i32),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Notification {
    #[prost(oneof = "notification::NotificationType", tags = "1")]
    pub notification_type: ::core::option::Option<notification::NotificationType>,
}

pub mod notification {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum NotificationType {
        #[prost(bool, tag = "1")]
        UnsavedChangesStatusChanged(bool),
    }
}

// --- Keymap messages ---

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Keymap {
    #[prost(message, repeated, tag = "1")]
    pub layers: ::prost::alloc::vec::Vec<Layer>,
    #[prost(uint32, tag = "2")]
    pub available_layers: u32,
    #[prost(uint32, tag = "3")]
    pub max_layer_name_length: u32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Layer {
    #[prost(uint32, tag = "1")]
    pub id: u32,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub bindings: ::prost::alloc::vec::Vec<BehaviorBinding>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BehaviorBinding {
    #[prost(sint32, tag = "1")]
    pub behavior_id: i32,
    #[prost(uint32, tag = "2")]
    pub param1: u32,
    #[prost(uint32, tag = "3")]
    pub param2: u32,
}

// --- Physical layout messages ---

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PhysicalLayouts {
    #[prost(uint32, tag = "1")]
    pub active_layout_index: u32,
    #[prost(message, repeated, tag = "2")]
    pub layouts: ::prost::alloc::vec::Vec<PhysicalLayout>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PhysicalLayout {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "2")]
    pub keys: ::prost::alloc::vec::Vec<KeyPhysicalAttrs>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KeyPhysicalAttrs {
    #[prost(sint32, tag = "1")]
    pub width: i32,
    #[prost(sint32, tag = "2")]
    pub height: i32,
    #[prost(sint32, tag = "3")]
    pub x: i32,
    #[prost(sint32, tag = "4")]
    pub y: i32,
    #[prost(sint32, tag = "5")]
    pub r: i32,
    #[prost(sint32, tag = "6")]
    pub rx: i32,
    #[prost(sint32, tag = "7")]
    pub ry: i32,
}

// --- Mutation request/response types (included for completeness) ---

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetLayerBindingRequest {
    #[prost(uint32, tag = "1")]
    pub layer_id: u32,
    #[prost(int32, tag = "2")]
    pub key_position: i32,
    #[prost(message, optional, tag = "3")]
    pub binding: ::core::option::Option<BehaviorBinding>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveLayerRequest {
    #[prost(uint32, tag = "1")]
    pub start_index: u32,
    #[prost(uint32, tag = "2")]
    pub dest_index: u32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AddLayerRequest {}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoveLayerRequest {
    #[prost(uint32, tag = "1")]
    pub layer_index: u32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RestoreLayerRequest {
    #[prost(uint32, tag = "1")]
    pub layer_id: u32,
    #[prost(uint32, tag = "2")]
    pub at_index: u32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetLayerPropsRequest {
    #[prost(uint32, tag = "1")]
    pub layer_id: u32,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SaveChangesResponse {
    #[prost(oneof = "save_changes_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<save_changes_response::Result>,
}

pub mod save_changes_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(bool, tag = "1")]
        Ok(bool),
        #[prost(enumeration = "super::SaveChangesErrorCode", tag = "2")]
        Err(i32),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SaveChangesErrorCode {
    Ok = 0,
    Generic = 1,
    NotSupported = 2,
    NoSpace = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SetLayerBindingResponse {
    Ok = 0,
    InvalidLocation = 1,
    InvalidBehavior = 2,
    InvalidParameters = 3,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetActivePhysicalLayoutResponse {
    #[prost(oneof = "set_active_physical_layout_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<set_active_physical_layout_response::Result>,
}

pub mod set_active_physical_layout_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Ok(super::Keymap),
        #[prost(enumeration = "super::SetActivePhysicalLayoutErrorCode", tag = "2")]
        Err(i32),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SetActivePhysicalLayoutErrorCode {
    Ok = 0,
    Generic = 1,
    InvalidLayoutIndex = 2,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveLayerResponse {
    #[prost(oneof = "move_layer_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<move_layer_response::Result>,
}

pub mod move_layer_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Ok(super::Keymap),
        #[prost(enumeration = "super::MoveLayerErrorCode", tag = "2")]
        Err(i32),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MoveLayerErrorCode {
    Ok = 0,
    Generic = 1,
    InvalidLayer = 2,
    InvalidDestination = 3,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AddLayerResponse {
    #[prost(oneof = "add_layer_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<add_layer_response::Result>,
}

pub mod add_layer_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Ok(super::AddLayerResponseDetails),
        #[prost(enumeration = "super::AddLayerErrorCode", tag = "2")]
        Err(i32),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AddLayerResponseDetails {
    #[prost(uint32, tag = "1")]
    pub index: u32,
    #[prost(message, optional, tag = "2")]
    pub layer: ::core::option::Option<Layer>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum AddLayerErrorCode {
    Ok = 0,
    Generic = 1,
    NoSpace = 2,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoveLayerResponse {
    #[prost(oneof = "remove_layer_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<remove_layer_response::Result>,
}

pub mod remove_layer_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Ok(super::RemoveLayerOk),
        #[prost(enumeration = "super::RemoveLayerErrorCode", tag = "2")]
        Err(i32),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoveLayerOk {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum RemoveLayerErrorCode {
    Ok = 0,
    Generic = 1,
    InvalidIndex = 2,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RestoreLayerResponse {
    #[prost(oneof = "restore_layer_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<restore_layer_response::Result>,
}

pub mod restore_layer_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Ok(super::Layer),
        #[prost(enumeration = "super::RestoreLayerErrorCode", tag = "2")]
        Err(i32),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum RestoreLayerErrorCode {
    Ok = 0,
    Generic = 1,
    InvalidId = 2,
    InvalidIndex = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SetLayerPropsResponse {
    Ok = 0,
    ErrGeneric = 1,
    ErrInvalidId = 2,
}
