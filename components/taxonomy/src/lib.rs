//! This crate defines the high-level API for accessing Connected Devices.
#![feature(custom_derive, plugin, stmt_expr_attributes)]
#![plugin(serde_macros)]
#![plugin(clippy)]
// To prevent clippy being noisy with derive(...)
#![allow(used_underscore_binding)]

#[macro_use]
extern crate lazy_static;

extern crate chrono;
extern crate libc;
#[macro_use]
extern crate log;

#[macro_use]
extern crate mopa;

extern crate rusqlite;
extern crate serde;
extern crate serde_json;
extern crate string_cache;
extern crate sublock;
extern crate transformable_channels;

pub mod adapters;
pub mod api;
pub mod io;
pub mod misc;

/// Standardized components.
pub mod library;