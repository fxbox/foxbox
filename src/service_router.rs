/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use controller::Controller;
use iron::{ AfterMiddleware, headers, IronResult, Request, Response };
use iron::headers::ContentType;
use iron::method::Method;
use iron::method::Method::*;
use iron::prelude::Chain;
use iron::status::Status;
use router::Router;
use unicase::UniCase;

type Endpoint = (&'static[Method], &'static[&'static str]);

struct CORS;

impl CORS {
    // Only endpoints listed here will allow CORS.
    // Endpoints containing a variable path part can use '*' like in:
    // &["bar", "*"] for a URL like https://foo.com/bar/123
    pub const ENDPOINTS: &'static[Endpoint] = &[
        (&[Method::Get], &["list.json"]),
        (&[Method::Get, Method::Post, Method::Put], &["*", "*"])
    ];
}

impl AfterMiddleware for CORS {
    fn after(&self, req: &mut Request, mut res: Response)
        -> IronResult<Response> {

        let mut is_cors_endpoint = false;
        for endpoint in CORS::ENDPOINTS {
            let (ref methods, path) = *endpoint;

            if !methods.contains(&req.method) &&
               req.method != Method::Options {
                continue;
            }

            if path.len() != req.url.path.len() {
                continue;
            }

            for (i, path) in path.iter().enumerate() {
                is_cors_endpoint = false;
                if req.url.path[i] != path.to_owned() &&
                   "*" != path.to_owned() {
                    break;
                }
                is_cors_endpoint = true;
            }
            if is_cors_endpoint {
                break;
            }
        }

        if !is_cors_endpoint {
            return Ok(res);
        }

        res.headers.set(headers::AccessControlAllowOrigin::Any);
        res.headers.set(headers::AccessControlAllowHeaders(
            vec![
                UniCase(String::from("accept")),
                UniCase(String::from("content-type"))
            ]
        ));
        res.headers.set(headers::AccessControlAllowMethods(
            vec![Get, Post, Put]
        ));
        res.status = Some(Status::Ok);
        Ok(res)
    }
}

pub fn create<T: Controller>(controller: T) -> Chain {
    let mut router = Router::new();

    let c1 = controller.clone();
    router.get("list.json", move |_: &mut Request| -> IronResult<Response> {
        // Build a json representation of the services.
        let serialized = itry!(c1.services_as_json());

        let mut response = Response::with(serialized);
        response.status = Some(Status::Ok);
        response.headers.set(ContentType::json());

        Ok(response)
    });

    let c2 = controller.clone();
    router.any(":service/:command", move |req: &mut Request| -> IronResult<Response> {
        // Call a function on a service.
        let id = req.extensions.get::<Router>().unwrap()
            .find("service").unwrap_or("").to_owned();
        c2.dispatch_service_request(id, req)
    });

    let mut chain = Chain::new(router);
    chain.link_after(CORS);

    chain
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
        assert_eq!(result, "[]");
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

    it "should get the appropriate CORS headers" {
        use iron::headers;
        use super::CORS;

        for endpoint in CORS::ENDPOINTS {
            let (_, path) = *endpoint;
            let path = "http://localhost:3000/".to_owned() +
                       &(path.join("/").replace("*", "foo"));
            match request::options(&path, Headers::new(), &service_router) {
                Ok(res) => {
                    let headers = &res.headers;
                    assert!(headers.has::<headers::AccessControlAllowOrigin>());
                    assert!(headers.has::<headers::AccessControlAllowHeaders>());
                    assert!(headers.has::<headers::AccessControlAllowMethods>());
                },
                _ => {
                    assert!(false)
                }
            }
        }
    }
}
