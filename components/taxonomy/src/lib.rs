//! This crate defines the high-level API for accessing Connected Devices.
//!
//!
//! # Taxonomy
//!
//! A network of Connected Devices is composed of `Service`s. Each service
//! is essentially a collection of `Service<Input>`s, which provide
//! data from the devices for use by applications, and
//! `Service<Output>`s, which give applications the ability to send
//! instructions to devices.
//!
//! Each `Service` has a `ServiceKind`, which determines the only
//! feature provided by this service, as well as the type of messages
//! that can be sent to/received from a service. The core list of
//! `ServiceKind` is hardcoded, but open for extensions.
//!
//!
//!
//! # Example
//!
//! The FoxBox itelf is a `Service`, which may offer the following services:
//!
//! - `Service<Input>`: `ServiceKind::CurrentTime`, `ServiceKind::CurrentTimeOfDay`, ...
//! - `Service<Output>`: `ServiceKind::SMS`.
//!
//!
//! # Example
//!
//! A light is a `Service`, which may offer:
//!
//! - a `Service<Output>` with `ServiceKind::OnOff`, to turn the light on or off;
//! - a `Service<Input>` with `ServiceKind::OnOff`, to determine whether the light is on or off;
//! - a `Service<Output>` with `ServiceKind::Color`, to change the color of the light;
//! - ...
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate chrono;
extern crate serde;
extern crate serde_json;

/// Metadata on devices
pub mod services;

/// Public-facing API
pub mod api;

/// Selecting one or more devices. Exposed through the API.
pub mod selector;

/// Values that may be sent to/received from devices
pub mod values;

/// Various utilities
pub mod util;
