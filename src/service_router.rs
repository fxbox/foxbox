/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use context::{ ContextTrait, Shared };
use iron::{Request, Response, IronResult};
use iron::headers::{ ContentType, AccessControlAllowOrigin };
use iron::status::Status;
use router::Router;
use core::marker::Reflect;


pub fn create<T: Send + Reflect + ContextTrait + 'static> (context: Shared<T>) -> Router {

    let mut router = Router::new();
    let context1 = context.clone();
    router.get("list.json", move |_: &mut Request| -> IronResult<Response> {
        // Build a json representation of the services.
        let ctx = context1.lock().unwrap();
        let serialized = itry!(ctx.services_as_json());

        let mut response = Response::with(serialized);
        response.status = Some(Status::Ok);
        response.headers.set(AccessControlAllowOrigin::Any);
        response.headers.set(ContentType::json());

        Ok(response)
    });

    let context2 = context.clone();
    router.get(":service/:command", move |req: &mut Request| -> IronResult<Response> {
        // Call a function on a service.
        let ctx = context2.lock().unwrap();

        let id = req.extensions.get::<Router>().unwrap().find("service").unwrap_or("");
        match ctx.get_service(id) {
            None => {
                let mut response = Response::with(format!("No Such Service: {}", id));
                response.status = Some(Status::BadRequest);
                response.headers.set(AccessControlAllowOrigin::Any);
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


// FIXME: Declare this crate in the test module, and avoid the use of pub
// When this comment was written, putting these 2 lines in before_each returns a compile error:
// ``` error: unresolved import `self::iron_test::request`. Could not find `iron_test` in
// service_router::service_router` [E0432] ```
extern crate iron_test;
pub use self::iron_test::{request, response};

#[cfg(test)]
describe! service_router {
    before_each {
        use stubs::context::ContextStub;
        use iron::Headers;

        let context = ContextStub::blank_shared();
        let service_router = create(context);
    }

    it "should create list.json" {
        let response = request::get("http://localhost:3000/list.json",
                        Headers::new(),
                        &service_router).unwrap();

        let result = response::extract_body_to_string(response);
        assert_eq!(result, "{}");
    }

    it "should make service available" {
        let response = request::get("http://localhost:3000/1/a-command",
                        Headers::new(),
                        &service_router).unwrap();

        let result = response::extract_body_to_string(response);
        assert_eq!(result, "request processed");
    }

    it "should return an error if no service was found" {
        let response = request::get("http://localhost:3000/unknown-id/a-command",
                        Headers::new(),
                        &service_router).unwrap();

        let result = response::extract_body_to_string(response);
        assert_eq!(result, "No Such Service: unknown-id");
    }
}
