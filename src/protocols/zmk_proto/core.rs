// Pre-generated from proto/zmk/core.proto
// Copyright (c) 2024 The ZMK Contributors — SPDX-License-Identifier: MIT

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum LockState {
    Locked = 0,
    Unlocked = 1,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    #[prost(oneof = "request::RequestType", tags = "1, 2, 3, 4")]
    pub request_type: ::core::option::Option<request::RequestType>,
}

pub mod request {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum RequestType {
        #[prost(bool, tag = "1")]
        GetDeviceInfo(bool),
        #[prost(bool, tag = "2")]
        GetLockState(bool),
        #[prost(bool, tag = "3")]
        Lock(bool),
        #[prost(bool, tag = "4")]
        ResetSettings(bool),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(oneof = "response::ResponseType", tags = "1, 2, 4")]
    pub response_type: ::core::option::Option<response::ResponseType>,
}

pub mod response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ResponseType {
        #[prost(message, tag = "1")]
        GetDeviceInfo(super::GetDeviceInfoResponse),
        #[prost(enumeration = "super::LockState", tag = "2")]
        GetLockState(i32),
        #[prost(bool, tag = "4")]
        ResetSettings(bool),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDeviceInfoResponse {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub serial_number: ::prost::alloc::vec::Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Notification {
    #[prost(oneof = "notification::NotificationType", tags = "1")]
    pub notification_type: ::core::option::Option<notification::NotificationType>,
}

pub mod notification {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum NotificationType {
        #[prost(enumeration = "super::LockState", tag = "1")]
        LockStateChanged(i32),
    }
}
