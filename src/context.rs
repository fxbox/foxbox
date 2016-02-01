/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;
extern crate collections;

use self::collections::vec::IntoIter;
use service::Service;
use std::collections::HashMap;
use std::io::Error;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::sync::{ Arc, Mutex };

// The `global` context available to all.
pub struct Context {
    pub verbose: bool,

    hostname: String,
    http_port: u16,
    ws_port: u16,
    services: HashMap<String, Box<Service>>
}

const DEFAULT_HTTP_PORT: &'static str = "3000";
const DEFAULT_WS_PORT: &'static str = "4000";
const DEFAULT_HOSTNAME: &'static str = "0.0.0.0";

pub type SharedContext = Arc<Mutex<Context>>;

impl Context {
    pub fn new(verbose: bool, hostname: Option<String>,
               http_port: Option<String>, ws_port: Option<String>) -> Context {
        Context { services: HashMap::new(),
                  verbose: verbose,
                  hostname:  hostname.unwrap_or(DEFAULT_HOSTNAME.to_string()),
                  http_port: http_port.unwrap_or(DEFAULT_HTTP_PORT.to_string()).parse::<u16>().unwrap(),
                  ws_port: ws_port.unwrap_or(DEFAULT_WS_PORT.to_string()).parse::<u16>().unwrap() }
    }

    pub fn shared(verbose: bool, hostname: Option<String>,
                  http_port:Option<String>, ws_port:Option<String>) -> SharedContext {
        Arc::new(Mutex::new(Context::new(verbose, hostname, http_port, ws_port)))
    }

    pub fn add_service(&mut self, service: Box<Service>) {
        let service_id = service.get_properties().id;
        self.services.insert(service_id, service);
    }

    pub fn remove_service(&mut self, id: String) {
        self.services.remove(&id);
    }

    pub fn services_count(&self) -> usize {
        self.services.len()
    }

    pub fn get_service(&self, id: &str) -> Option<&Box<Service>> {
        self.services.get(id)
    }

    pub fn services_as_json(&self) -> Result<String, serde_json::error::Error> {
        serde_json::to_string(&self.services)
    }

    pub fn get_http_root_for_service(&self, service_id: String) -> String {
        format!("http://{}:{}/services/{}/", self.hostname, self.http_port, service_id)
    }

    pub fn get_ws_root_for_service(&self, service_id: String) -> String {
        format!("ws://{}:{}/services/{}/", self.hostname, self.ws_port, service_id)
    }

    pub fn http_as_addrs(&self) -> Result<IntoIter<SocketAddr>, Error> {
        (self.hostname.as_str(), self.http_port).to_socket_addrs()
    }
}
