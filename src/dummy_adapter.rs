/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use context::SharedContext;
use events::*;
use iron::{ Request, Response, IronResult };
use iron::headers::ContentType;
use iron::status::Status;
use router::Router;
use service::{ Service, ServiceAdapter, ServiceProperties };
use std::time::Duration;
use std::thread;
use uuid::Uuid;

struct DummyService {
    properties: ServiceProperties,
    sender: EventSender,
    dont_kill: bool
}

impl DummyService {
    fn new(sender: EventSender, context: SharedContext, id: u32) -> DummyService {
        println!("Creating dummy service");
        let ctx_clone = context.clone();
        let ctx = ctx_clone.lock().unwrap();
        let service_id = Uuid::new_v4().to_simple_string();
        DummyService {
            properties: ServiceProperties {
                id: service_id.clone(),
                name: "dummy service".to_string(),
                description: "really nothing to see".to_string(),
                http_url: ctx.get_http_root_for_service(service_id.clone()),
                ws_url: ctx.get_ws_root_for_service(service_id)
            },
            sender: sender,
            dont_kill: id % 3 == 0
        }
    }
}

impl Service for DummyService {
    fn get_properties(&self) -> ServiceProperties {
        self.properties.clone()
    }

    // Starts the service, it will just spawn a thread and send messages once
    // in a while.
    fn start(&self) {
        let sender = self.sender.clone();
        let props = self.properties.clone();
        let can_kill = !self.dont_kill.clone();
        thread::spawn(move || {
            println!("Hello from dummy service thread!");
            let mut i = 0;
            loop {
                thread::sleep(Duration::from_millis(1000));
                println!("Bip #{} from {}", i, props.id);
                i += 1;
                if i == 3 && can_kill {
                    break;
                }
            }
            sender.send(EventData::ServiceStop { id: props.id.to_string() }).unwrap();
        });
    }

    fn stop(&self) {
        println!("Stopping dummy service");
    }

    // Processes a http request.
    fn process_request(&self, req: &Request) -> IronResult<Response> {
        let cmd = req.extensions.get::<Router>().unwrap().find("command").unwrap_or("");
        let mut response = Response::with(format!("Got command {} at url {}", cmd, req.url));
        response.status = Some(Status::Ok);
        response.headers.set(ContentType::plaintext());
        Ok(response)
    }
}

pub struct DummyAdapter {
    name: String,
    sender: EventSender,
    context: SharedContext
}

impl DummyAdapter {
    pub fn new(sender: EventSender,
           context: SharedContext) -> DummyAdapter {
        println!("Creating dummy adapter");
        DummyAdapter { name: "DummyAdapter".to_string(),
                       sender: sender,
                       context: context
                     }
    }
}

impl ServiceAdapter for DummyAdapter {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn start(&self) {
        let sender = self.sender.clone();
        let mut id = 0;
        let context = self.context.clone();
        thread::spawn(move || {
            sender.send(EventData::AdapterStart { name: "Dummy Service Adapter".to_string() }).unwrap();
            loop {
                thread::sleep(Duration::from_millis(2000));
                id += 1;
                let service = DummyService::new(sender.clone(), context.clone(), id);
                let service_id = service.get_properties().id;
                service.start();
                let mut ctx = context.lock().unwrap();
                ctx.add_service(Box::new(service));
                sender.send(EventData::ServiceStart { id: service_id }).unwrap();

                // Create at most 7 dummy services.
                if id == 7 {
                    break;
                }
            }
        });
    }

    fn stop(&self) {
        println!("Stopping dummy adapter");
    }
}
