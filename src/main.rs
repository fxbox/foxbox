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

// Need to be declared first so to let the macros be visible from other modules.
#[macro_use]
mod utils;

mod controller;
mod adapters;
mod events;
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

docopt!(Args derive Debug, "
Usage: foxbox [-v] [-h] [-n <hostname>] [-p <port>] [-w <wsport>] [-r <url>] [-i <iface>] [-t <tunnel>]

Options:
    -v, --verbose            Toggle verbose output.
    -n, --name <hostname>    Set local hostname.
    -p, --port <port>        Set port to listen on for http connections.
    -w, --wsport <wsport>    Set port to listen on for websocket.
    -r, --register <url>     Change the url of the registration endpoint.
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
        args.flag_verbose, args.flag_name, args.flag_port, args.flag_wsport);

    controller.run();
}

#[test]
fn options_are_good() {
    // short form options
    {
        let argv = || vec!["foxbox", "-p", "1234", "-n", "foobar",
                           "-w", "4567", "-v"];

        let args: Args = Args::docopt().argv(argv().into_iter())
            .decode()
            .unwrap_or_else(|e| e.exit());

        assert_eq!(args.flag_verbose, true);
        assert_eq!(args.flag_name, Some("foobar".to_string()));
        assert_eq!(args.flag_port, Some(1234));
        assert_eq!(args.flag_wsport, Some(4567));
        assert_eq!(args.flag_help, false);
    }
    // long form options
    {
        let argv = || vec!["foxbox", "--port", "1234",
                           "--name", "foobar", "--wsport", "4567",
                           "--verbose"];

        let args: Args = Args::docopt().argv(argv().into_iter())
            .decode()
            .unwrap_or_else(|e| e.exit());

        assert_eq!(args.flag_verbose, true);
        assert_eq!(args.flag_name, Some("foobar".to_string()));
        assert_eq!(args.flag_port, Some(1234));
        assert_eq!(args.flag_wsport, Some(4567));
        assert_eq!(args.flag_help, false);
    }
}
