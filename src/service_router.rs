/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use controller::Controller;
use foxbox_users::auth_middleware::{ AuthEndpoint, AuthMiddleware };
use iron::{ AfterMiddleware, headers, IronResult, Request, Response };
use iron::headers::ContentType;
use iron::method::Method;
use iron::method::Method::*;
use iron::prelude::Chain;
use iron::status::Status;
use router::Router;
use unicase::UniCase;

type Endpoint = (&'static[Method], &'static str);

struct CORS;

impl CORS {
    // Only endpoints listed here will allow CORS.
    // Endpoints containing a variable path part can use ':foo' like in:
    // "/foo/:bar" for a URL like https://domain.com/foo/123 where 123 is
    // variable.
    pub const ENDPOINTS: &'static[Endpoint] = &[
        (&[Method::Get], "list.json"),
        (&[Method::Get, Method::Post, Method::Put], ":service/:command")
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

            let path: Vec<&str> = if path.starts_with('/') {
                path[1..].split('/').collect()
            } else {
                path[0..].split('/').collect()
            };

            if path.len() != req.url.path.len() {
                continue;
            }

            for (i, req_path) in req.url.path.iter().enumerate() {
                is_cors_endpoint = false;
                if req_path != path[i] && !path[i].starts_with(':') {
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
                UniCase(String::from("authorization")),
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
        match req.method {
            Method::Get |
            Method::Post |
            Method::Put => {
                // Call a function on a service.
                let id = req.extensions.get::<Router>().unwrap()
                    .find("service").unwrap_or("").to_owned();
                c2.dispatch_service_request(id, req)
            },
            _ => Ok(Response::with(Status::NotImplemented))
        }
    });

    let auth_endpoints = if cfg!(feature = "authentication") {
        vec![
            AuthEndpoint(vec![Method::Get], "list.json".to_owned()),
            AuthEndpoint(vec![Method::Get, Method::Post, Method::Put],
                         ":service/:command".to_owned())
        ]
    } else {
        vec![]
    };

    let mut chain = Chain::new(router);
    chain.around(AuthMiddleware {
        auth_endpoints: auth_endpoints
    });

    chain.link_after(CORS);

    chain
}

#[cfg(test)]
describe! service_router {
    before_each {
        use controller::FoxBox;
        use foxbox_users::users_db::{ UserBuilder, UsersDb };
        use foxbox_users::users_router::UsersRouter;
        use iron::Headers;
        use iron_test::request;
        use mount::Mount;

        let controller = FoxBox::new(false, Some("localhost".to_owned()), 1234, 5678);
        let service_router = create(controller.clone());

        let mut mount = Mount::new();
        mount.mount("", service_router)
             .mount("/users", UsersRouter::init());

        let db = UsersDb::new();
        db.clear().ok();
        let user = UserBuilder::new()
            .id(1).name(String::from("username"))
            .password(String::from("password"))
            .email(String::from("username@example.com"))
            .finalize().unwrap();
        db.create(&user).ok();
    }

    describe! services {
        before_each {
            extern crate serde_json;

            use iron::headers::{ Authorization, Basic, Bearer };
            use iron_test::response;

            let mut headers = Headers::new();
            headers.set(Authorization(Basic {
                username: "username".to_owned(),
                password: Some("password".to_owned())
            }));
            let response = request::post("http://localhost:1234/users/login",
                                         headers,
                                         "{}",
                                         &mount).unwrap();
            #[derive(Debug, PartialEq, Serialize, Deserialize)]
            struct Token {
                session_token: String
            };

            let result: Token = serde_json::from_str(
                &response::extract_body_to_string(response)
            ).unwrap();
            let mut auth_header = Headers::new();
            auth_header.set(Authorization(Bearer {
                token: result.session_token
            }));
        }

        it "should create list.json" {
            let response = request::get("http://localhost:1234/list.json",
                            auth_header,
                            &mount).unwrap();

            let result = response::extract_body_to_string(response);
            assert_eq!(result, "[]");
        }

        it "should make service available" {
            use controller::Controller;
            use stubs::service::ServiceStub;
            controller.add_service(Box::new(ServiceStub));
            let response = request::get("http://localhost:1234/1/a-command",
                            auth_header,
                            &mount).unwrap();

            let result = response::extract_body_to_string(response);
            assert_eq!(result, "request processed");
        }

        it "should return an error if no service was found" {
            let response = request::get("http://localhost:1234/unknown-id/a-command",
                            auth_header,
                            &mount).unwrap();

            let result = response::extract_body_to_string(response);
            assert_eq!(result, r#"{"error":"NoSuchService","id":"unknown-id"}"#);
        }
    }

    describe! cors {
        before_each {
            use iron::headers;
            use super::super::CORS;
        }

        it "should get the appropriate CORS headers" {
            for endpoint in CORS::ENDPOINTS {
                let (_, path) = *endpoint;
                let path = "http://localhost:1234/".to_owned() +
                           &(path.replace(":", "foo"));
                let response = request::options(&path, Headers::new(), &mount).unwrap();
                let headers = &response.headers;
                assert!(headers.has::<headers::AccessControlAllowOrigin>());
                assert!(headers.has::<headers::AccessControlAllowHeaders>());
                assert!(headers.has::<headers::AccessControlAllowMethods>());
            }
        }
    }
}
