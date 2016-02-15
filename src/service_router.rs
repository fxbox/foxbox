/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use context::{ ContextTrait, Shared };
use iron::{Request, Response, IronResult};
use iron::headers::ContentType;
use iron::status::Status;
use router::Router;
use core::marker::Reflect;

pub struct ServiceRouter<Ctx> where Ctx: ContextTrait {
    context: Shared<Ctx>,
}

impl<Ctx> ServiceRouter<Ctx> where Ctx: Send + Reflect + ContextTrait + 'static {
    pub fn new(context: Shared<Ctx>) -> ServiceRouter<Ctx> {
        ServiceRouter { context: context }
    }

    pub fn generate_router(&self) -> Router {
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
        router
    }

}

extern crate iron_test;

use stubs::context::ContextStub;
use iron::{Handler, Headers, status};
use self::iron_test::{request, response};

#[test]
fn test_toto() {
// describe! http_server {
    // before_each {


        let context = ContextStub::blank_shared();
        let http_server = ServiceRouter::<ContextStub>::new(context);
    // }

    // it "should create list.json" {
// pub struct Request<'a, 'b> {
//     pub url: Url,
//     pub remote_addr: SocketAddr,
//     pub local_addr: SocketAddr,
//     pub headers: Headers,
//     pub body: Body<'a, 'b>,
//     pub method: Method,
//     pub extensions: TypeMap,
// }

        let response = request::get("http://localhost:3000/list.json",
                        Headers::new(),
                        &http_server.generate_router()).unwrap();


        // let res = http_server.handle(req).unwrap();
    // }
}
