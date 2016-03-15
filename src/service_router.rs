/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use controller::Controller;
use foxbox_users::AuthEndpoint;
use iron::{ IronResult, Request, Response };
use iron::headers::ContentType;
use iron::method::Method;
use iron::prelude::Chain;
use iron::status::Status;
use router::Router;

pub fn create<T: Controller>(controller: T) -> Chain {
    let mut router = Router::new();

    let c1 = controller.clone();
    router.get("list", move |_: &mut Request| -> IronResult<Response> {
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
            Method::Put |
            Method::Delete => {
                // Call a function on a service.
                let id = req.extensions.get::<Router>().unwrap()
                    .find("service").unwrap_or("").to_owned();
                c2.dispatch_service_request(id, req)
            },
            // CORS middleware will take care of adding the CORS headers
            // if they are allowed.
            Method::Options => Ok(Response::with(Status::Ok)),
            _ => Ok(Response::with(Status::NotImplemented))
        }
    });

    let auth_endpoints = if cfg!(feature = "authentication") {
        vec![
            AuthEndpoint(vec![Method::Get], "list".to_owned()),
            AuthEndpoint(vec![Method::Get, Method::Post, Method::Put],
                         ":service/:command".to_owned())
        ]
    } else {
        vec![]
    };

    let mut chain = Chain::new(router);
    chain.around(controller.get_users_manager().get_middleware(auth_endpoints));

    chain
}

#[cfg(test)]
describe! service_router {
    before_each {
        use controller::FoxBox;
        use iron::Headers;
        use iron_test::request;
        use mount::Mount;
        use tls::TlsOption;
        use profile_service::ProfilePath;

        let controller = FoxBox::new(false, Some("localhost".to_owned()),
                                     1234, 5678, TlsOption::Disabled,
                                     ProfilePath::Default);
        let service_router = create(controller.clone());

        let mut mount = Mount::new();
        // This is ugly here, but 1/ I can do 'use controller::Controller'
        // in this block and 2/ if I don't, I get told I need to do it
        // for the trait.
        let manager = {
            use controller::Controller;
            let manager = controller.get_users_manager();
            mount.mount("", service_router)
                .mount("/users", manager.get_router_chain());
            manager
        };
    }

    describe! services {
        before_each {
            extern crate serde_json;

            use foxbox_users::UserBuilder;
            use iron::headers::{ Authorization, Basic, Bearer };
            use iron_test::response;

            let db = manager.get_db();
            db.clear().ok();
            let user = UserBuilder::new()
                .id(1).name(String::from("username"))
                .password(String::from("password"))
                .email(String::from("username@example.com"))
                .finalize().unwrap();
            db.create(&user).ok();
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

        it "should create list" {
            let response = request::get("http://localhost:1234/list",
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
}
