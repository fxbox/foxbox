/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

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
    context: SharedContext
}

impl HttpServer {
    pub fn new(context: SharedContext) -> HttpServer {
        HttpServer { context: context }
    }

    pub fn start(&self) {
        let mut router = Router::new();

        let context1 = self.context.clone();
        router.get("list.json", move |_: &mut Request| -> IronResult<Response> {
            // Build a json representation of the services.
            let ctx = context1.lock().unwrap();
            let serialized = itry!(ctx.services_as_json());

            let mut response = Response::with(serialized);
            response.status = Some(Status::Ok);
            response.headers.set(ContentType::json());

            Ok(response)
        });

        let context2 = self.context.clone();
        router.get(":service/:command", move |req: &mut Request| -> IronResult<Response> {
            // Call a function on a service.
            let ctx = context2.lock().unwrap();

            let id = req.extensions.get::<Router>().unwrap().find("service").unwrap_or("");
            match ctx.get_service(id) {
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

        let thread_context = self.context.clone();
        let ctx = thread_context.lock().unwrap();
        let addrs: Vec<_> = ctx.http_as_addrs().unwrap().collect();

        thread::Builder::new().name("HttpServer".to_string())
                              .spawn(move || {
            Iron::new(mount).http(addrs[0]).unwrap();
        }).unwrap();
    }
}
