/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Needed to derive `Serialize` on ServiceProperties
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

// Needed for IntoIter in context.rs
#![feature(collections)]

// Make linter fail for every warning
#![plugin(clippy)]
#![deny(clippy)]

extern crate core;
extern crate docopt;
extern crate env_logger;
#[macro_use]
extern crate iron;
#[macro_use]
extern crate log;
extern crate mio;
extern crate mount;
extern crate router;
extern crate rustc_serialize;
extern crate serde;
extern crate staticfile;
extern crate uuid;

mod context;
mod dummy_adapter;
mod events;
mod http_server;
mod service;
mod controler;

use context::Context;
use controler::Controler;
use docopt::Docopt;

const USAGE: &'static str = "
Usage: foxbox [-v] [-h] [-n <hostname>]

Options:
    -v, --verbose            Toggle verbose output.
    -n, --name               Set local hostname.
    -h, --help               Print this help menu.
";

#[derive(RustcDecodable)]
struct Args {
    flag_verbose: bool,
    arg_hostname: Option<String>,
    flag_help: bool,
}

fn main() {
    env_logger::init().unwrap();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    if args.flag_help {
        println!("{}", USAGE);
        return
    }

    if let Ok(mut event_loop) = mio::EventLoop::new() {
        let sender = event_loop.channel();

        let context = Context::shared(args.flag_verbose, args.arg_hostname);
        let mut controler = Controler::new(sender, context);
        controler.start();

        event_loop.run(&mut controler)
                  .unwrap_or_else(|_| {  panic!("Starting the event loop failed!"); });
    } else {
        panic!("Creating the event loop failed!");
    }
}
