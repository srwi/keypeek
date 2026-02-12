// Pre-generated from proto/zmk/meta.proto
// Copyright (c) 2024 The ZMK Contributors — SPDX-License-Identifier: MIT

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ErrorConditions {
    Generic = 0,
    UnlockRequired = 1,
    RpcNotFound = 2,
    MsgDecodeFailed = 3,
    MsgEncodeFailed = 4,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(oneof = "response::ResponseType", tags = "1, 2")]
    pub response_type: ::core::option::Option<response::ResponseType>,
}

pub mod response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ResponseType {
        #[prost(bool, tag = "1")]
        NoResponse(bool),
        #[prost(enumeration = "super::ErrorConditions", tag = "2")]
        SimpleError(i32),
    }
}
