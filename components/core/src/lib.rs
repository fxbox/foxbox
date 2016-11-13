// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![feature(plugin)]

#![plugin(clippy)]
#![deny(clippy)]

#![cfg_attr(test, feature(const_fn))] // Dependency of stainless
#![cfg_attr(test, plugin(stainless))] // Test runner

extern crate core;
extern crate foxbox_users;
extern crate hyper;
extern crate libc;

#[macro_use]
extern crate log;
extern crate serde_json;

extern crate tls;

#[cfg(test)]
extern crate uuid;
#[cfg(test)]
extern crate tempdir;

extern crate ws;
extern crate xml;

// Needs to come first to let other modules use exported macros.
#[macro_use]
pub mod utils;

pub mod config_store;
pub mod managed_process;
pub mod profile_service;
pub mod traits;
pub mod upnp;
