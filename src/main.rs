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

// Needed for time functions
#![feature(time2)]

// Make linter fail for every warning
#![plugin(clippy)]
#![deny(clippy)]

#![cfg_attr(test, feature(const_fn))] // Dependency of stainless
#![cfg_attr(test, plugin(stainless))] // Test runner

#![feature(reflect_marker)]

#![feature(associated_consts)]

extern crate core;
extern crate docopt;
extern crate env_logger;
extern crate foxbox_users;
#[macro_use]
extern crate iron;
#[cfg(test)]
extern crate iron_test;
extern crate libc;
#[macro_use]
extern crate log;
extern crate mio;
extern crate mount;
extern crate router;
extern crate rustc_serialize;
extern crate serde;
extern crate staticfile;
extern crate unicase;
extern crate uuid;
extern crate ws;
extern crate multicast_dns;

// Need to be declared first so to let the macros be visible from other modules.
#[macro_use]
mod utils;

mod controller;
mod adapters;
mod http_server;
mod managed_process;
mod registration;
mod service;
mod service_router;
mod tunnel_controller;
mod ws_server;

mod stubs {
    #![allow(dead_code)]
    #![allow(unused_variables)]
    #![allow(boxed_local)]
    pub mod service;
}

use controller::{ Controller, FoxBox, DEFAULT_HTTP_PORT };
use tunnel_controller:: { TunnelConfig, Tunnel };
use multicast_dns::host::HostManager;

docopt!(Args derive Debug, "
Usage: foxbox [-v] [-h] [-n <hostname>] [-p <port>] [-w <wsport>] [-r <url>] [-i <iface>] [-t <tunnel>]

Options:
    -v, --verbose            Toggle verbose output.
    -n, --name <hostname>    Set local hostname. [default: foxbox]
    -p, --port <port>        Set port to listen on for http connections. [default: 3000]
    -w, --wsport <wsport>    Set port to listen on for websocket. [default: 4000]
    -r, --register <url>     Change the url of the registration endpoint. [default: http://localhost:4242/register]
    -i, --iface <iface>      Specify the local IP interface.
    -t, --tunnel <tunnel>    Set the tunnel endpoint's hostname. If omitted, the tunnel is disabled.
    -h, --help               Print this help menu.
",
        flag_name: Option<String>,
        flag_port: Option<u16>,
        flag_wsport: Option<u16>,
        flag_iface: Option<String>,
        flag_register: Option<String>,
        flag_tunnel: Option<String>);

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

fn main() {
    env_logger::init().unwrap();

    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());

    let registrar = registration::Registrar::new();
    registrar.start(args.flag_register, args.flag_iface);

    // Start the tunnel.
    if let Some(host) = args.flag_tunnel {
        let mut tunnel =
            Tunnel::new(TunnelConfig::new(args.flag_port.unwrap_or(DEFAULT_HTTP_PORT), host));
        tunnel.start().unwrap();
    }

    let mut controller = FoxBox::new(
        args.flag_verbose, args.flag_name.map_or(None, update_hostname), args.flag_port,
        args.flag_wsport);

    controller.run();
}

#[cfg(test)]
describe! main {
    describe! args {
        it "should have default values" {
            let argv = || vec!["foxbox"];
            let args: super::super::Args = super::super::Args::docopt().argv(argv().into_iter())
                .decode().unwrap();

            assert_eq!(args.flag_verbose, false);
            assert_eq!(args.flag_name.unwrap(), "foxbox");
            assert_eq!(args.flag_port.unwrap(), 3000);
            assert_eq!(args.flag_wsport.unwrap(), 4000);
            assert_eq!(args.flag_register.unwrap(), "http://localhost:4242/register");
            assert_eq!(args.flag_iface, None);
            assert_eq!(args.flag_tunnel, None);
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
                               "-t", "tunnel.host"];

           let args: super::super::Args = super::super::Args::docopt().argv(argv().into_iter())
               .decode().unwrap();

            assert_eq!(args.flag_verbose, true);
            assert_eq!(args.flag_name.unwrap(), "foobar");
            assert_eq!(args.flag_port.unwrap(), 1234);
            assert_eq!(args.flag_wsport.unwrap(), 4567);
            assert_eq!(args.flag_register.unwrap(), "http://foo.bar:6868/register");
            assert_eq!(args.flag_iface.unwrap(), "eth99");
            assert_eq!(args.flag_tunnel.unwrap(), "tunnel.host");
        }

        it "should support long form" {
            let argv = || vec!["foxbox",
                               "--verbose",
                               "--port", "1234",
                               "--name", "foobar",
                               "--wsport", "4567",
                               "--register", "http://foo.bar:6868/register",
                               "--iface", "eth99",
                               "--tunnel", "tunnel.host"];

            let args: super::super::Args = super::super::Args::docopt().argv(argv().into_iter())
                .decode().unwrap();

            assert_eq!(args.flag_verbose, true);
            assert_eq!(args.flag_name.unwrap(), "foobar");
            assert_eq!(args.flag_port.unwrap(), 1234);
            assert_eq!(args.flag_wsport.unwrap(), 4567);
            assert_eq!(args.flag_register.unwrap(), "http://foo.bar:6868/register");
            assert_eq!(args.flag_iface.unwrap(), "eth99");
            assert_eq!(args.flag_tunnel.unwrap(), "tunnel.host");
        }
    }
}
