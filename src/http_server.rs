/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use controller::Controller;
use iron::{ AfterMiddleware, Chain, Handler, Iron, IronResult,
            Request, Response };
use iron_cors::CORS;
use iron::error::{ IronError };
use iron::method::Method;
use iron::status::Status;
use mount::Mount;
use router::NoRoute;
use service_router;
use static_router;
use std::thread;
use webpush::WebPushRouter;

struct Custom404;

impl AfterMiddleware for Custom404 {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        use std::io::Error as StdError;
        use std::io::ErrorKind;

        if let Some(_) = err.error.downcast::<NoRoute>() {
            // Router error
            return Ok(Response::with((Status::NotFound,
                                      format!("Unknown resource: {}", err))));
        } else if let Some(err) = err.error.downcast::<StdError>() {
            // StaticFile error
            if err.kind() == ErrorKind::NotFound {
                return Ok(Response::with((Status::NotFound,
                                          format!("Unknown resource: {}", err))));
            }
        }

        // Just let other errors go through, like 401.
        Err(err)
    }
}

struct Ping;

impl Handler for Ping {
    fn handle (&self, _: &mut Request) -> IronResult<Response> {
        Ok(Response::with(Status::Ok))
    }
}

pub struct HttpServer<T: Controller> {
    controller: T
}

impl<T: Controller> HttpServer<T> {
    pub fn new(controller: T) -> Self {
        HttpServer { controller: controller }
    }

    pub fn start(&mut self) {
        let router = service_router::create(self.controller.clone());
        let wp_router = WebPushRouter::create(self.controller.clone());

        let users_manager = self.controller.get_users_manager();
        let mut mount = Mount::new();
        mount.mount("/", static_router::create(users_manager.clone()))
             .mount("/ping", Ping)
             .mount("/services", router)
             .mount("/users", users_manager.get_router_chain())
             .mount("/push", wp_router);

        let mut chain = Chain::new(mount);
        chain.link_after(Custom404);

        let cors = CORS::new(vec![
            (vec![Method::Get], "ping".to_owned()),
            (vec![Method::Get, Method::Post, Method::Put, Method::Delete],
             "services/:service/:command".to_owned()),
            (vec![Method::Get], "services/list".to_owned())
        ]);
        chain.link_after(cors);

        let addrs: Vec<_> = self.controller.http_as_addrs().unwrap().collect();

        thread::Builder::new().name("HttpServer".to_owned())
                              .spawn(move || {
            Iron::new(chain).http(addrs[0]).unwrap();
        }).unwrap();
    }
}

#[cfg(test)]
describe! ping {
    before_each {
        use mount::Mount;
        use iron::Headers;
        use iron::status::Status;
        use iron_test::request;
        use super::Ping;

        let mut mount = Mount::new();
        mount.mount("/ping", Ping);
    }

    it "should response 200 Ok" {
        let response = request::get("http://localhost:3000/ping",
                                    Headers::new(),
                                    &mount).unwrap();
        assert_eq!(response.status.unwrap(), Status::Ok);
    }
}

#[cfg(test)]
describe! cors {
    before_each {
        extern crate hyper;

        use iron::headers;
        use iron::method::Method;
        use stubs::controller::ControllerStub;

        let mut http_server = HttpServer::new(ControllerStub::new());
        http_server.start();
    }

    it "should get the appropriate CORS headers" {
        let endpoints = vec![
            (vec![Method::Get, Method::Post, Method::Put],
             "services/:service/:command".to_owned()),
            (vec![Method::Get], "services/list".to_owned())
        ];
        let client = hyper::Client::new();
        for endpoint in endpoints {
            let (_, path) = endpoint;
            let path = "http://localhost:3000/".to_owned() +
                       &(path.replace(":", "foo"));
            let res = client.get(&path).send();
            let headers = &res.unwrap().headers;
            assert!(headers.has::<headers::AccessControlAllowOrigin>());
            assert!(headers.has::<headers::AccessControlAllowHeaders>());
            assert!(headers.has::<headers::AccessControlAllowMethods>());
        };
    }
}

