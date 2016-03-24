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
#![plugin(clippy)]

extern crate chrono;
extern crate serde;
extern crate serde_json;
extern crate string_cache;
extern crate transformable_channels;

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

/// The back-end thread, in charge of the heavy lifting of managing adapters.
mod backend;

/// The manager provides an API for (un)registering adapters, services, channels, and
/// uses these to implements the taxonomy API.
pub mod manager;

/// The API for defining Adapters.
pub mod adapter;

/// Utility module for inserting values in maps and keeping the insertion reversible in case of
/// any error.
pub mod transact;

/// Implementation of a fake adapter, controlled entirely programmatically. Designed to be used
/// as a component of tests.
pub mod fake_adapter;