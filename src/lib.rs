// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// Needed to derive `Serialize` on ServiceProperties
#![feature(custom_derive, plugin)]

// Make linter fail for every warning
#![plugin(clippy)]

#![deny(clippy)]

// Needed for many #[derive(...)] macros
#![allow(used_underscore_binding)]

#![cfg_attr(test, feature(const_fn))] // Dependency of stainless
#![cfg_attr(test, plugin(stainless))] // Test runner

#![feature(associated_consts)]

extern crate chrono;
extern crate core;
#[macro_use]
extern crate foxbox_core;
#[macro_use]
extern crate foxbox_taxonomy;
#[cfg(feature = "thinkerbell")]
extern crate foxbox_thinkerbell;
extern crate foxbox_users;
#[macro_use]
extern crate hyper;
#[macro_use]
extern crate iron;
extern crate iron_cors;
#[cfg(test)]
extern crate iron_test;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
extern crate mio;
extern crate mount;
extern crate openssl;
extern crate openssl_sys;
extern crate pagekite;
extern crate rand;
extern crate router;
extern crate rusqlite;
extern crate rustc_serialize;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate staticfile;
extern crate tls;
extern crate time;
extern crate timer;
extern crate transformable_channels;
extern crate unicase;
extern crate url;
extern crate ws;

// adapters
#[cfg(feature = "zwave")]
extern crate openzwave_adapter as openzwave;

#[cfg(test)]
extern crate regex;
#[cfg(test)]
extern crate tempdir;
#[cfg(test)]
extern crate uuid;

#[cfg(test)]
mod stubs {
    #![allow(dead_code)]
    #![allow(unused_variables)]
    #![allow(boxed_local)]
    pub mod controller;
}

mod adapters;
pub mod controller;
mod http_server;
pub mod registration;
mod static_router;
mod taxonomy_router;
pub mod tunnel_controller;
mod ws_server;
