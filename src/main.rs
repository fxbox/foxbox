/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Needed to derive `Serialize` on ServiceProperties
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
// For Docopt macro
#![plugin(docopt_macros)]

// Needed for IntoIter in controller.rs
#![feature(collections)]

// Make linter fail for every warning
#![plugin(clippy)]
#![deny(clippy)]

#![cfg_attr(test, feature(const_fn))] // Dependency of stainless
#![cfg_attr(test, plugin(stainless))] // Test runner

#![feature(reflect_marker)]

#![feature(associated_consts)]

#![feature(fnbox)] // Let us use FnBox<>, since Box<FnOnce> doesn't work.

extern crate chrono;
extern crate core;
extern crate docopt;
extern crate env_logger;
extern crate foxbox_users;
extern crate foxbox_taxonomy;
#[macro_use]
extern crate iron;
#[cfg(test)]
extern crate iron_test;
extern crate libc;
#[macro_use]
extern crate log;
extern crate mio;
extern crate mount;
extern crate nix;
extern crate router;
extern crate rustc_serialize;
extern crate serde;
extern crate staticfile;
extern crate time;
extern crate timer;
extern crate unicase;
extern crate uuid;
extern crate ws;
extern crate multicast_dns;
extern crate xml;

#[cfg(test)]
extern crate regex;

// Need to be declared first so to let the macros be visible from other modules.
#[macro_use]
mod utils;
mod transact;
mod adapt;
mod adapters;
mod config_store;
mod controller;
mod http_server;
mod managed_process;
mod profile_service;
mod registration;
mod service;
mod upnp;
mod service_router;
mod stable_uuid;
mod static_router;
mod tunnel_controller;
mod ws_server;

mod stubs {
    #![allow(dead_code)]
    #![allow(unused_variables)]
    #![allow(boxed_local)]
    pub mod service;
}

use controller::{ Controller, FoxBox };
use env_logger::LogBuilder;
use tunnel_controller:: { TunnelConfig, Tunnel };
use libc::SIGINT;
use log::{ LogRecord, LogLevelFilter };

use multicast_dns::host::HostManager;
use std::env;
use std::mem;
use std::sync::atomic::{ AtomicBool, Ordering, ATOMIC_BOOL_INIT };

docopt!(Args derive Debug, "
Usage: foxbox [-v] [-h] [-n <hostname>] [-p <port>] [-w <wsport>] [-r <url>] [-i <iface>] [-t <tunnel>] [-c <namespace;key;value>]...

Options:
    -v, --verbose            Toggle verbose output.
    -n, --name <hostname>    Set local hostname. Linux only. Requires to be a member of the netdev group.
    -p, --port <port>        Set port to listen on for http connections. [default: 3000]
    -w, --wsport <wsport>    Set port to listen on for websocket. [default: 4000]
    -r, --register <url>     Change the url of the registration endpoint. [default: http://localhost:4242/register]
    -i, --iface <iface>      Specify the local IP interface.
    -t, --tunnel <tunnel>    Set the tunnel endpoint's hostname. If omitted, the tunnel is disabled.
    -c, --config <namespace;key;value>  Set configuration override
    -h, --help               Print this help menu.
",
        flag_name: Option<String>,
        flag_port: u16,
        flag_wsport: u16,
        flag_register: String,
        flag_iface: Option<String>,
        flag_tunnel: Option<String>,
        flag_config: Option<Vec<String>>);

/// Updates local host name with the provided host name string. If requested host name
/// is not available (used by anyone else on the same network) then collision
/// resolution logic is triggered and alternative name is chosen automatically
/// (host name plus "-2", "-3" and etc. postfix). This function blocks until host name
/// is updated and returns actual host name.
///
/// # Panics
///
/// Panics if provided host name is not valid non-FQDN host name.
///
/// # Arguments
///
/// * `hostname` - host name name we'd like to set (should be a valid non-FQDN host name).
fn update_hostname(hostname: String) -> Option<String> {
    let host_manager = HostManager::new();

    if !host_manager.is_valid_name(&hostname) {
        panic!("Host name `{}` is not a valid host name!", &hostname);
    }

    Some(host_manager.set_name(&hostname))
}

// Handle SIGINT (Ctrl-C) for manual shutdown.
// Signal handlers must not do anything substantial. To trigger shutdown, we atomically
// flip this flag; the event loop checks the flag and exits accordingly.
static SHUTDOWN_FLAG: AtomicBool = ATOMIC_BOOL_INIT;
unsafe fn handle_sigint(_:i32) {
    SHUTDOWN_FLAG.store(true, Ordering::Release);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[inline]
fn tid_str() -> String {
    // gettid only exists for the linux and android variants of nix
    format!("({}) ", nix::unistd::gettid())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[inline]
fn tid_str() -> &'static str {
    ""
}

fn main() {
    unsafe {
        libc::signal(SIGINT, mem::transmute(handle_sigint));
    }

    let format = |record: &LogRecord| {
        let t = time::now();
        format!("{}.{:03}: {}{:5} {}",
            time::strftime("%Y-%m-%d %H:%M:%S", &t).unwrap(),
            t.tm_nsec / 1000_000,
            tid_str(),
            record.level(),
            record.args()
        )
    };
    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
       builder.parse(&env::var("RUST_LOG").unwrap());
    }
    builder.init().unwrap();

    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());

    let registrar = registration::Registrar::new();
    registrar.start(args.flag_register, args.flag_iface);

    // Start the tunnel.
    let mut tunnel: Option<Tunnel> = None;
    if let Some(host) = args.flag_tunnel {
        tunnel = Some(Tunnel::new(TunnelConfig::new(args.flag_port, host)));
        tunnel.as_mut().unwrap().start().unwrap();
    }

    let mut controller = FoxBox::new(
        args.flag_verbose, args.flag_name.map_or(None, update_hostname), args.flag_port,
        args.flag_wsport);

    // Override config values
    {
        if let Some(flags) = args.flag_config {
            for flag in flags {
                let items: Vec<String> = utils::split_escaped(&flag, ';');
                if items.len() >= 3 {
                    let namespace = items[0].clone();
                    let key = items[1].clone();
                    let value = items[2..].join(";");
                    warn!("Setting config override: {}::{}->{}", namespace, key, value);
                    controller.config.set_override(&namespace, &key, &value);
                } else {
                    error!("Config override requires three fields: {}", flag)
                }
            }
        }
    }

    controller.run(&SHUTDOWN_FLAG);

    if let Some(mut tunnel) = tunnel {
        tunnel.stop().unwrap();
    }
}

#[cfg(test)]
describe! main {
    describe! args {
        it "should have default values" {
            let argv = || vec!["foxbox"];
            let args: super::super::Args = super::super::Args::docopt().argv(argv().into_iter())
                .decode().unwrap();

            assert_eq!(args.flag_verbose, false);
            assert_eq!(args.flag_name, None);
            assert_eq!(args.flag_port, 3000);
            assert_eq!(args.flag_wsport, 4000);
            assert_eq!(args.flag_register, "http://localhost:4242/register");
            assert_eq!(args.flag_iface, None);
            assert_eq!(args.flag_tunnel, None);
            assert_eq!(args.flag_config, None);
            assert_eq!(args.flag_help, false);
        }

        it "should support short form" {
            let argv = || vec!["foxbox",
                               "-v",
                               "-p", "1234",
                               "-n", "foobar",
                               "-w", "4567",
                               "-r", "http://foo.bar:6868/register",
                               "-i", "eth99",
                               "-t", "tunnel.host",
                               "-c", "ns;key;value"];

           let args: super::super::Args = super::super::Args::docopt().argv(argv().into_iter())
               .decode().unwrap();

            assert_eq!(args.flag_verbose, true);
            assert_eq!(args.flag_name.unwrap(), "foobar");
            assert_eq!(args.flag_port, 1234);
            assert_eq!(args.flag_wsport, 4567);
            assert_eq!(args.flag_register, "http://foo.bar:6868/register");
            assert_eq!(args.flag_iface.unwrap(), "eth99");
            assert_eq!(args.flag_tunnel.unwrap(), "tunnel.host");
            assert_eq!(args.flag_config.unwrap(), vec!["ns;key;value"]);
        }

        it "should support long form" {
            let argv = || vec!["foxbox",
                               "--verbose",
                               "--port", "1234",
                               "--name", "foobar",
                               "--wsport", "4567",
                               "--register", "http://foo.bar:6868/register",
                               "--iface", "eth99",
                               "--tunnel", "tunnel.host",
                               "--config", "ns;key;value"];

            let args: super::super::Args = super::super::Args::docopt().argv(argv().into_iter())
                .decode().unwrap();

            assert_eq!(args.flag_verbose, true);
            assert_eq!(args.flag_name.unwrap(), "foobar");
            assert_eq!(args.flag_port, 1234);
            assert_eq!(args.flag_wsport, 4567);
            assert_eq!(args.flag_register, "http://foo.bar:6868/register");
            assert_eq!(args.flag_iface.unwrap(), "eth99");
            assert_eq!(args.flag_tunnel.unwrap(), "tunnel.host");
            assert_eq!(args.flag_config.unwrap(), vec!["ns;key;value"]);
        }
    }
}
