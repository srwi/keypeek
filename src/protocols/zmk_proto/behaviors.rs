// Pre-generated from proto/zmk/behaviors.proto
// Copyright (c) 2024 The ZMK Contributors — SPDX-License-Identifier: MIT

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    #[prost(oneof = "request::RequestType", tags = "1, 2")]
    pub request_type: ::core::option::Option<request::RequestType>,
}

pub mod request {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum RequestType {
        #[prost(bool, tag = "1")]
        ListAllBehaviors(bool),
        #[prost(message, tag = "2")]
        GetBehaviorDetails(super::GetBehaviorDetailsRequest),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBehaviorDetailsRequest {
    #[prost(uint32, tag = "1")]
    pub behavior_id: u32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(oneof = "response::ResponseType", tags = "1, 2")]
    pub response_type: ::core::option::Option<response::ResponseType>,
}

pub mod response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ResponseType {
        #[prost(message, tag = "1")]
        ListAllBehaviors(super::ListAllBehaviorsResponse),
        #[prost(message, tag = "2")]
        GetBehaviorDetails(super::GetBehaviorDetailsResponse),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListAllBehaviorsResponse {
    #[prost(uint32, repeated, tag = "1")]
    pub behaviors: ::prost::alloc::vec::Vec<u32>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBehaviorDetailsResponse {
    #[prost(uint32, tag = "1")]
    pub id: u32,
    #[prost(string, tag = "2")]
    pub display_name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub metadata: ::prost::alloc::vec::Vec<BehaviorBindingParametersSet>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BehaviorBindingParametersSet {
    #[prost(message, repeated, tag = "1")]
    pub param1: ::prost::alloc::vec::Vec<BehaviorParameterValueDescription>,
    #[prost(message, repeated, tag = "2")]
    pub param2: ::prost::alloc::vec::Vec<BehaviorParameterValueDescription>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BehaviorParameterNil {}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BehaviorParameterLayerId {}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BehaviorParameterHidUsage {
    #[prost(uint32, tag = "1")]
    pub keyboard_max: u32,
    #[prost(uint32, tag = "2")]
    pub consumer_max: u32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BehaviorParameterValueDescriptionRange {
    #[prost(int32, tag = "1")]
    pub min: i32,
    #[prost(int32, tag = "2")]
    pub max: i32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BehaviorParameterValueDescription {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(
        oneof = "behavior_parameter_value_description::ValueType",
        tags = "2, 3, 4, 5, 6"
    )]
    pub value_type: ::core::option::Option<behavior_parameter_value_description::ValueType>,
}

pub mod behavior_parameter_value_description {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ValueType {
        #[prost(message, tag = "2")]
        Nil(super::BehaviorParameterNil),
        #[prost(uint32, tag = "3")]
        Constant(u32),
        #[prost(message, tag = "4")]
        Range(super::BehaviorParameterValueDescriptionRange),
        #[prost(message, tag = "5")]
        HidUsage(super::BehaviorParameterHidUsage),
        #[prost(message, tag = "6")]
        LayerId(super::BehaviorParameterLayerId),
    }
}
