// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
extern crate serde_json;

use std::collections::BTreeMap;
use std::thread;

use iron::{Request, Response, IronResult};
use iron::headers::{ContentType, AccessControlAllowOrigin};
use iron::status::Status;
use iron::method::Method;

use multicast_dns::discovery::discovery_manager::*;

use controller::Controller;
use service::{Service, ServiceAdapter, ServiceProperties};
use uuid::Uuid;

const ADAPTER_NAME: &'static str = "WebServerAdapter";
const WEB_SERVER_SERVICE_TYPE: &'static str = "_http._tcp";

pub struct WebServerAdapter<T> {
    name: String,
    controller: T,
}

impl<T: Controller> WebServerAdapter<T> {
    pub fn new(controller: T) -> Self {
        info!("Creating Web Server adapter.");

        WebServerAdapter {
            name: ADAPTER_NAME.to_owned(),
            controller: controller,
        }
    }
}

impl<T: Controller> ServiceAdapter for WebServerAdapter<T> {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn start(&self) {
        let controller = self.controller.clone();

        thread::spawn(move || {
            controller.adapter_started("WebServer Service Adapter".to_owned());

            let manager = DiscoveryManager::new();

            let on_service_resolved = |service: ServiceInfo| {
                let service = WebServerService::new(controller.clone(),
                                                    WebServer {
                                                        name: service.name.unwrap(),
                                                        host: service.host_name.unwrap(),
                                                        port: service.port,
                                                        protocol: service.protocol,
                                                    });
                service.start();
                controller.add_service(Box::new(service));
            };

            let on_service_discovered = |service: ServiceInfo| {
                info!("Service discovered: {:?}", service);

                manager.resolve_service(service,
                                        ResolveListeners {
                                            on_service_resolved: Some(&on_service_resolved),
                                        });
            };

            let on_all_discovered = || {
                info!("All service discovered for now");
                manager.stop_service_discovery();
            };

            manager.discover_services(WEB_SERVER_SERVICE_TYPE,
                                      DiscoveryListeners {
                                          on_service_discovered: Some(&on_service_discovered),
                                          on_all_discovered: Some(&on_all_discovered),
                                      });
        });
    }

    fn stop(&self) {
        info!("Stopping WebServer Service Adapter.");
    }
}

struct WebServer {
    name: String,
    host: String,
    port: u16,
    protocol: ServiceProtocol,
}

struct WebServerService {
    properties: ServiceProperties,
    server: WebServer,
}

impl WebServerService {
    fn new<T>(controller: T, server: WebServer) -> Self
        where T: Controller
    {
        let service_id = Uuid::new_v4().to_simple_string();

        WebServerService {
            properties: ServiceProperties {
                id: service_id.clone(),
                name: format!("{} {:?}", server.name, server.protocol),
                description: format!("{} {:?}", server.name, server.protocol),
                http_url: format!("{}view",
                                  controller.get_http_root_for_service(service_id.clone())),
                ws_url: "".to_owned(),
                custom_properties: BTreeMap::new(),
            },
            server: server,
        }
    }
}

impl Service for WebServerService {
    fn get_properties(&self) -> ServiceProperties {
        self.properties.clone()
    }

    fn start(&self) {
        info!("Starting service for {} ({}:{} via {:?})",
              self.server.name,
              self.server.host,
              self.server.port,
              self.server.protocol);
    }

    fn stop(&self) {
        info!("Stopping service for {} ({}:{} via {:?})",
              self.server.name,
              self.server.host,
              self.server.port,
              self.server.protocol);
    }

    fn process_request(&self, req: &mut Request) -> IronResult<Response> {
        match req.method {
            Method::Get => {
                let server_address = format!("http://{}:{}", self.server.host, self.server.port);
                let json = json!(
                    { type: "webserver",
                      address: server_address,
                      name: self.server.name,
                      protocol: format!("{:?}", self.server.protocol)});

                let mut response = Response::with(json);
                response.status = Some(Status::Ok);
                response.headers.set(ContentType::json());
                response.headers.set(AccessControlAllowOrigin::Any);
                Ok(response)
            }
            _ => {
                let mut response = Response::with("{\"result\": \"error\"}".to_owned());
                response.status = Some(Status::MethodNotAllowed);
                response.headers.set(ContentType::json());
                response.headers.set(AccessControlAllowOrigin::Any);
                Ok(response)
            }
        }
    }
}
