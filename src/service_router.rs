/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use controller::Controller;
use iron::{Request, Response, IronResult};
use iron::headers::{ ContentType, AccessControlAllowOrigin };
use iron::status::Status;
use router::Router;

pub fn create<T: Controller>(controller: T) -> Router {
    let mut router = Router::new();

    let c1 = controller.clone();
    router.get("list.json", move |_: &mut Request| -> IronResult<Response> {
        // Build a json representation of the services.
        let serialized = itry!(c1.services_as_json());

        let mut response = Response::with(serialized);
        response.status = Some(Status::Ok);
        response.headers.set(AccessControlAllowOrigin::Any);
        response.headers.set(ContentType::json());

        Ok(response)
    });

    let c2 = controller.clone();
    router.get(":service/:command", move |req: &mut Request| -> IronResult<Response> {
        // Call a function on a service.
        let id = req.extensions.get::<Router>().unwrap().find("service").unwrap_or("");
        c2.dispatch_service_request(id.to_owned(), req)
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
        use iron::Headers;
        use controller::FoxBox;

        let controller = FoxBox::new(false, Some("localhost".to_owned()), None, None);
        let service_router = create(controller.clone());
    }

    it "should create list.json" {
        let response = request::get("http://localhost:3000/list.json",
                        Headers::new(),
                        &service_router).unwrap();

        let result = response::extract_body_to_string(response);
        assert_eq!(result, "{}");
    }

    it "should make service available" {
        use controller::Controller;
        use stubs::service::ServiceStub;
        controller.add_service(Box::new(ServiceStub));
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
