/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use context::SharedContext;
use events::*;
use iron::{Iron, Request, Response, IronResult};
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
    context: SharedContext,
    dontKill: bool
}

impl DummyService {
    fn new(sender: EventSender, context: SharedContext, id: u32) -> DummyService {
        println!("Creating dummy service");
        let mut ctxClone = context.clone();
        let ctx = ctxClone.lock().unwrap();
        let serviceId = Uuid::new_v4().to_simple_string();
        DummyService {
            properties: ServiceProperties {
                id: serviceId.clone(),
                name: "dummy service".to_string(),
                description: "really nothing to see".to_string(),
                http_url: format!("http://{}:{}/services/{}/", ctx.hostname, ctx.http_port, serviceId),
                ws_url: format!("ws://{}:{}/services/{}/", ctx.hostname, ctx.http_port, serviceId)
            },
            sender: sender,
            context: context,
            dontKill: id % 3 == 0
        }
    }
}

impl Drop for DummyService {
    fn drop(&mut self) {
        println!("Droping DummyService {}", self.properties.id);
    }
}

impl Service for DummyService {
    fn get_properties(&self) -> ServiceProperties {
        self.properties.clone()
    }

    // Starts the service, it will just spawn a thread and get messages once
    // in a while.
    fn start(&self) {
        let sender = self.sender.clone();
        let props = self.properties.clone();
        let canKill = !self.dontKill.clone();
        thread::spawn(move || {
            println!("Hello from dummy service thread!");
            let mut i = 0;
            loop {
                thread::sleep(Duration::from_millis(1000));
                println!("Bip #{} from {}", i, props.id);
                i += 1;
                if i == 3 && canKill {
                    break;
                }
            }
            sender.send(EventData::ServiceStop { id: props.id.to_string() });
        });
    }

    fn stop(&self) {
        println!("Stopping dummy service");
    }

    // Processes a request.
    fn process_request(&self, req: &    Request) -> IronResult<Response> {
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
            sender.send(EventData::AdapterStart { name: "Dummy Service Adapter".to_string() });
            loop {
                thread::sleep(Duration::from_millis(2000));
                id += 1;
                let service = DummyService::new(sender.clone(), context.clone(), id);
                let sId = service.get_properties().id;
                service.start();
                let mut ctx = context.lock().unwrap();
                ctx.services.insert(sId.clone(), Box::new(service));
                sender.send(EventData::ServiceStart { id: sId });

                // Create at most 5 dummy services.
                if id == 5 {
                    break;
                }
            }
        });
    }

    fn stop(&self) {
        println!("Stopping dummy adapter");
    }
}
