/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;
extern crate collections;

use context::{ ContextTrait };
use super::service::ServiceStub;
use super::super::service::Service;
use std::collections::{ HashMap, hash_map };
use std::sync::{ Arc, Mutex };
use std::io::Error;
use std::net::SocketAddr;
use self::collections::vec::IntoIter;
use ws;

pub struct ContextStub {
    // Needed so get_service can return a value that has a long-enough lifetime
    stubbed_service: Box<Service>,
    websockets: HashMap<ws::util::Token, ws::Sender>,
}

pub type SharedContextStub = Arc<Mutex<ContextStub>>;

impl ContextTrait for ContextStub {

    fn new(verbose: bool, hostname: Option<String>, http_port: Option<u16>, ws_port: Option<u16>)
           -> ContextStub {
        ContextStub {
            websockets: HashMap::new(),
            stubbed_service: Box::new(ServiceStub),
        }
    }

    fn shared(verbose: bool, hostname: Option<String>, http_port: Option<u16>,
              ws_port: Option<u16>) -> SharedContextStub {
        Arc::new(Mutex::new(ContextStub::new(verbose, hostname, http_port, ws_port)))
    }

    fn add_service(&mut self, service: Box<Service>) {}

    fn remove_service(&mut self, id: String) {}

    fn add_websocket(&mut self, socket: ws::Sender) {}

    fn remove_websocket(&mut self, socket: ws::Sender) {}

    fn services_count(&self) -> usize { 0 }

    fn get_service(&self, id: &str) -> Option<&Box<Service>> {
        if id == "1" {
            Some(&self.stubbed_service)
        } else {
            None
        }
    }

    fn services_as_json(&self) -> Result<String, serde_json::error::Error> { Ok("{}".to_owned()) }

    fn websockets_iter(&self) -> hash_map::Values<ws::util::Token, ws::Sender> {

        self.websockets.values()
    }

    fn get_http_root_for_service(&self, service_id: String) -> String { "".to_owned() }

    fn get_ws_root_for_service(&self, service_id: String) -> String { "".to_owned() }

    fn http_as_addrs(&self) -> Result<IntoIter<SocketAddr>, Error> {
        let server: SocketAddr = "127.0.0.1:80".parse().unwrap();
        Ok(vec![server].into_iter())
    }

}

impl ContextStub {
    pub fn blank_shared() -> SharedContextStub {
        ContextStub::shared(false, Some("".to_owned()), Some(0), Some(0))
    }
}
