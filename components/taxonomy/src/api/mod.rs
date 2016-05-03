//! The high-level API.
//!
//! This API provides uniform management of devices for third-party developers.

/// Error-handling.
pub mod error;

/// The JSON API. Used to define the REST API and clients in other languages.
pub mod json;

/// The native API. Used by Rust clients.
pub mod native;

/// Selecting services or features.
pub mod selector;

/// Description of services and features.
pub mod services;