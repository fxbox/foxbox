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

const DEFAULT_HTTP_PORT: u16 = 3000;
const DEFAULT_WS_PORT: u16 = 4000;
const DEFAULT_HOSTNAME: &'static str = "::"; // ipv6 default.

pub type SharedContext = Arc<Mutex<Context>>;

impl Context {
    pub fn new(verbose: bool, hostname: Option<String>,
               http_port: Option<u16>, ws_port: Option<u16>) -> Context {

        Context { services: HashMap::new(),
                  verbose: verbose,
                  hostname:  hostname.unwrap_or(DEFAULT_HOSTNAME.to_owned()),
                  http_port: http_port.unwrap_or(DEFAULT_HTTP_PORT),
                  ws_port: ws_port.unwrap_or(DEFAULT_WS_PORT) }
    }

    pub fn shared(verbose: bool, hostname: Option<String>,
                  http_port: Option<u16>,
                  ws_port: Option<u16>) -> SharedContext {
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


describe! context {

    before_each {
        use stubs::service::ServiceStub;

        let service = ServiceStub;
        let context = Context::shared(false, Some("localhost".to_owned()), None, None);
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
