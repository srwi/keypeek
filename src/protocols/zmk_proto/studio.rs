// Pre-generated from proto/zmk/studio.proto
// Copyright (c) 2024 The ZMK Contributors — SPDX-License-Identifier: MIT

use super::{behaviors, core, keymap, meta};

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    #[prost(uint32, tag = "1")]
    pub request_id: u32,
    #[prost(oneof = "request::Subsystem", tags = "3, 4, 5")]
    pub subsystem: ::core::option::Option<request::Subsystem>,
}

pub mod request {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Subsystem {
        #[prost(message, tag = "3")]
        Core(super::core::Request),
        #[prost(message, tag = "4")]
        Behaviors(super::behaviors::Request),
        #[prost(message, tag = "5")]
        Keymap(super::keymap::Request),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(oneof = "response::Type", tags = "1, 2")]
    pub r#type: ::core::option::Option<response::Type>,
}

pub mod response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Type {
        #[prost(message, tag = "1")]
        RequestResponse(super::RequestResponse),
        #[prost(message, tag = "2")]
        Notification(super::Notification),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RequestResponse {
    #[prost(uint32, tag = "1")]
    pub request_id: u32,
    #[prost(oneof = "request_response::Subsystem", tags = "2, 3, 4, 5")]
    pub subsystem: ::core::option::Option<request_response::Subsystem>,
}

pub mod request_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Subsystem {
        #[prost(message, tag = "2")]
        Meta(super::meta::Response),
        #[prost(message, tag = "3")]
        Core(super::core::Response),
        #[prost(message, tag = "4")]
        Behaviors(super::behaviors::Response),
        #[prost(message, tag = "5")]
        Keymap(super::keymap::Response),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Notification {
    #[prost(oneof = "notification::Subsystem", tags = "2, 5")]
    pub subsystem: ::core::option::Option<notification::Subsystem>,
}

pub mod notification {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Subsystem {
        #[prost(message, tag = "2")]
        Core(super::core::Notification),
        #[prost(message, tag = "5")]
        Keymap(super::keymap::Notification),
    }
}
