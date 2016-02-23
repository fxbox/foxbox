/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;
extern crate collections;

use self::collections::vec::IntoIter;
use service::Service;
use std::collections::{ HashMap, hash_map };
use std::io::Error;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::sync::{ Arc, Mutex };
use tunnel_controller:: { TunnelConfig, Tunnel };
use ws;

// The `global` context available to all.
pub struct Context {
    pub verbose: bool,
    pub hostname: String,
    pub http_port: u16,
    pub ws_port: u16,
    services: HashMap<String, Box<Service>>,
    tunnel: Option<Tunnel>,
    websockets: HashMap<ws::util::Token, ws::Sender>,
}

const DEFAULT_HTTP_PORT: u16 = 3000;
const DEFAULT_WS_PORT: u16 = 4000;
const DEFAULT_HOSTNAME: &'static str = "::"; // ipv6 default.

pub trait ContextTrait {
    fn new(verbose: bool, hostname: Option<String>,
           http_port: Option<u16>, ws_port: Option<u16>,
           tunnel_host: Option<String>) -> Self;
    fn shared(verbose: bool, hostname: Option<String>,
              http_port: Option<u16>, ws_port: Option<u16>,
              tunnel_host: Option<String>) -> Shared<Self>;
    fn add_service(&mut self, service: Box<Service>);
    fn remove_service(&mut self, id: String);
    fn add_websocket(&mut self, socket_out: ws::Sender);
    fn remove_websocket(&mut self, socket_out: ws::Sender);
    fn services_count(&self) -> usize;
    fn get_service(&self, id: &str) -> Option<&Box<Service>>;
    fn services_as_json(&self) -> Result<String, serde_json::error::Error>;
    fn websockets_iter(&self) -> hash_map::Values<ws::util::Token, ws::Sender>;
    fn get_http_root_for_service(&self, service_id: String) -> String;
    fn get_ws_root_for_service(&self, service_id: String) -> String;
    fn http_as_addrs(&self) -> Result<IntoIter<SocketAddr>, Error>;
    fn start_tunnel(&mut self) -> Result<(), Error>;
    fn stop_tunnel(&mut self) -> Result<(), Error>;
}

pub type Shared<T> = Arc<Mutex<T>>;
pub type SharedContext = Shared<Context>;

impl ContextTrait for Context {
    fn new(verbose: bool, hostname: Option<String>,
               http_port: Option<u16>, ws_port: Option<u16>,
               tunnel_host: Option<String>) -> Self {

        let http_port = http_port.unwrap_or(DEFAULT_HTTP_PORT);

        let tunnel = if let Some(host) = tunnel_host {
            Some(Tunnel::new(TunnelConfig::new(http_port, host)))
        } else {
            None
        };

        Context { services: HashMap::new(),
                  websockets: HashMap::new(),
                  tunnel: tunnel,
                  verbose: verbose,
                  hostname:  hostname.unwrap_or(DEFAULT_HOSTNAME.to_owned()),
                  http_port: http_port,
                  ws_port: ws_port.unwrap_or(DEFAULT_WS_PORT) }
    }

    fn shared(verbose: bool, hostname: Option<String>,
                  http_port: Option<u16>, ws_port: Option<u16>,
                  tunnel_host: Option<String>) -> SharedContext {
        Arc::new(Mutex::new(Context::new(verbose, hostname, http_port, ws_port, tunnel_host)))
    }

    fn add_service(&mut self, service: Box<Service>) {
        let service_id = service.get_properties().id;
        self.services.insert(service_id, service);
    }

    fn remove_service(&mut self, id: String) {
        self.services.remove(&id);
    }

    fn add_websocket(&mut self, socket: ws::Sender) {
        self.websockets.insert(socket.token(), socket);
    }

    fn remove_websocket(&mut self, socket: ws::Sender) {
        self.websockets.remove(&socket.token());
    }

    fn services_count(&self) -> usize {
        self.services.len()
    }

    fn get_service(&self, id: &str) -> Option<&Box<Service>> {
        self.services.get(id)
    }

    fn services_as_json(&self) -> Result<String, serde_json::error::Error> {
        serde_json::to_string(&self.services)
    }

    fn websockets_iter(&self) -> hash_map::Values<ws::util::Token, ws::Sender> {
        self.websockets.values()
    }

    fn get_http_root_for_service(&self, service_id: String) -> String {
        format!("http://{}:{}/services/{}/", self.hostname, self.http_port, service_id)
    }

    fn get_ws_root_for_service(&self, service_id: String) -> String {
        format!("ws://{}:{}/services/{}/", self.hostname, self.ws_port, service_id)
    }

    fn http_as_addrs(&self) -> Result<IntoIter<SocketAddr>, Error> {
        (self.hostname.as_str(), self.http_port).to_socket_addrs()
    }

    fn start_tunnel(&mut self) -> Result<(), Error> {
        match self.tunnel {
            Some(ref mut tunnel) => tunnel.start(),
            // If nothing is configured, just allow
            _ => Ok(())
        }
    }

    fn stop_tunnel(&mut self) -> Result<(), Error> {
        match self.tunnel {
            None => Ok(()),
            Some(ref mut tunnel) => tunnel.stop(),
        }
    }
}


#[cfg(test)]
describe! context {

    before_each {
        use stubs::service::ServiceStub;

        let service = ServiceStub;
        let context = Context::shared(false, Some("localhost".to_owned()), None, None, None);
        let mut locked_context = context.lock().unwrap();
    }

    describe! add_service {
        it "should increase number of services" {
            assert_eq!(locked_context.services.is_empty(), true);
            locked_context.add_service(Box::new(service));
            assert_eq!(locked_context.services.is_empty(), false);
            assert_eq!(locked_context.services_count(), 1);
        }

        it "should make service available" {
            locked_context.add_service(Box::new(service));

            let service1 = locked_context.get_service("1");
            match service1 {
                Some(svc) => {
                    assert_eq!(svc.get_properties().id, "1");
                }
                None => assert!(false, "No service with id 1")
            }
        }

        it "should create http root" {
            locked_context.add_service(Box::new(service));
            assert_eq!(locked_context.get_http_root_for_service("1".to_string()),
                       "http://localhost:3000/services/1/");
        }

        it "should create ws root" {
            locked_context.add_service(Box::new(service));
            assert_eq!(locked_context.get_ws_root_for_service("1".to_string()),
                       "ws://localhost:4000/services/1/");
        }

        it "should return a json" {
            locked_context.add_service(Box::new(service));

            match locked_context.services_as_json() {
                Ok(txt) => assert_eq!(txt, "{\"1\":{\"id\":\"1\",\"name\":\"dummy service\",\"description\":\"really nothing to see\",\"http_url\":\"2\",\"ws_url\":\"3\"}}"),
                Err(err) => assert!(false, err)
            }
        }
    }


    it "should delete a service" {
        locked_context.add_service(Box::new(service));
        let id = "1".to_owned();
        locked_context.remove_service(id);
        assert_eq!(locked_context.services_count(), 0);
        assert_eq!(locked_context.services.is_empty(), true);
    }
}
