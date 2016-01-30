/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

use context::SharedContext;
use iron::{Iron, Request, Response, IronResult};
use iron::headers::ContentType;
use iron::status::Status;
use mount::Mount;
use router::Router;
use staticfile::Static;
use std::path::Path;
use std::thread;

pub struct HttpServer {
    context: SharedContext,
    router: Router
}

impl HttpServer {
    pub fn new(context: SharedContext) -> HttpServer {
        let mut router = Router::new();
        HttpServer { context: context,
                     router: router }
    }

    pub fn start(&self) {

        let server = "localhost:3000";
        let mut router = Router::new();

        let context1 = self.context.clone();
        router.get("list.json", move |req: &mut Request| -> IronResult<Response> {
            // Build a json representation of the services.
            let mut ctx = context1.lock().unwrap();
            let ref services = ctx.services;
            let serialized = itry!(serde_json::to_string(services));

            let mut response = Response::with(serialized);
            response.status = Some(Status::Ok);
            response.headers.set(ContentType::json());

            Ok(response)
        });

        let context2 = self.context.clone();
        router.get(":service/:command", move |req: &mut Request| -> IronResult<Response> {
            // Call a function on a service.
            let mut ctx = context2.lock().unwrap();
            let ref services = ctx.services;

            let id = req.extensions.get::<Router>().unwrap().find("service").unwrap_or("");
            match services.get(id) {
                None => {
                    let mut response = Response::with(format!("No Such Service : {}", id));
                    response.status = Some(Status::BadRequest);
                    response.headers.set(ContentType::plaintext());
                    Ok(response)
                }
                Some(service) => {
                    service.process_request(req)
                }
            }
        });

        let mut mount = Mount::new();
        mount.mount("/", Static::new(Path::new("static")))
             .mount("/services", router);

        thread::Builder::new().name("HttpServer".to_string())
                              .spawn(move || {
            Iron::new(mount).http(server).unwrap();
        });
    }
}
