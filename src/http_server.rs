// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use foxbox_core::traits::Controller;
use foxbox_taxonomy::manager::*;
use iron::{AfterMiddleware, Chain, Handler, Iron, IronResult, Request, Response, Protocol};
use iron_cors::CORS;
use iron::error::IronError;
use iron::method::Method;
use iron::status::Status;
use mount::Mount;
use router::NoRoute;
use static_router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::thread;
use taxonomy_router;

const THREAD_COUNT: usize = 8;

// 404 middleware.
struct Custom404;

impl AfterMiddleware for Custom404 {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        use std::io::Error as StdError;
        use std::io::ErrorKind;

        if err.error.downcast::<NoRoute>().is_some() {
            // Router error
            return Ok(Response::with((Status::NotFound, format!("Unknown resource: {}", err))));
        } else if let Some(err) = err.error.downcast::<StdError>() {
            // StaticFile error
            if err.kind() == ErrorKind::NotFound {
                return Ok(Response::with((Status::NotFound, format!("Unknown resource: {}", err))));
            }
        }

        // Just let other errors go through, like 401.
        Err(err)
    }
}

// Middleware that adds security related headers.
// See https://wiki.mozilla.org/Security/Guidelines/Web_Security
struct SecurityHeaders;

impl AfterMiddleware for SecurityHeaders {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        use iron::{headers, Set};
        use iron::modifiers::Header;

        // HSTS
        let header = headers::StrictTransportSecurity {
            include_subdomains: false,
            max_age: 31536000u64,
        };
        res.set_mut(Header(header));

        // Referrer-Policy
        res.set_mut(Header(headers::ReferrerPolicy::NoReferrer));

        // X-Frame-Options
        header! { (XFrameOptions, "X-Frame-Options") => [String] }
        res.set_mut(Header(XFrameOptions("DENY".to_owned())));

        // X-Content-Type-Options
        header! { (XContentTypeOptions, "X-Content-Type-Options") => [String] }
        res.set_mut(Header(XContentTypeOptions("nosniff".to_owned())));

        // Content-Security-Policy
        // TODO: refine the value of this header.
        header! { (Xcsp, "Content-Security-Policy") => [String] }
        res.set_mut(Header(Xcsp("default-src 'self'; frame-ancestors 'none'".to_owned())));

        // X-XSS-Protection
        header! { (XXSSProtection, "X-XSS-Protection") => [String] }
        res.set_mut(Header(XXSSProtection("1; mode=block".to_owned())));

        Ok(res)
    }
}

struct Ping;

impl Handler for Ping {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        Ok(Response::with(Status::NoContent))
    }
}

pub struct HttpServer<T: Controller> {
    controller: T,
}

impl<T: Controller> HttpServer<T> {
    pub fn new(controller: T) -> Self {
        HttpServer { controller: controller }
    }

    pub fn start(&mut self, adapter_api: &Arc<AdapterManager>) {
        let taxonomy_chain = taxonomy_router::create(self.controller.clone(), adapter_api);

        let users_manager = self.controller.get_users_manager();
        let mut mount = Mount::new();
        mount.mount("/", static_router::create(users_manager.clone()))
            .mount("/ping", Ping)
            .mount("/api/v1", taxonomy_chain)
            .mount("/users", users_manager.get_router_chain());

        let mut chain = Chain::new(mount);
        chain.link_after(Custom404);

        let cors = CORS::new(vec![(vec![Method::Get], "ping".to_owned()),

                                  // Taxonomy router paths. Keep in sync with taxonomy_router.rs
                                  (vec![Method::Get, Method::Post], "api/v1/services".to_owned()),
                                  (vec![Method::Post, Method::Delete],
                                   "api/v1/services/tags".to_owned()),
                                  (vec![Method::Get, Method::Post], "api/v1/channels".to_owned()),
                                  (vec![Method::Put], "api/v1/channels/get".to_owned()),
                                  (vec![Method::Put], "api/v1/channels/set".to_owned()),
                                  (vec![Method::Post, Method::Delete],
                                   "api/v1/channels/tags".to_owned())]);
        chain.link_after(cors);

        let addrs: Vec<_> = self.controller.http_as_addrs().unwrap().collect();

        if self.controller.get_tls_enabled() {
            // When running with TLS enabled, add the security headers.
            chain.link_after(SecurityHeaders);

            // This will fail when starting without a certificate, so for now just loop until we generate one.
            loop {
                // Get the certificate record for the remote hostname, and use its certificate and
                // private key files.
                let record =
                    self.controller.get_certificate_manager().get_remote_hostname_certificate();
                if record.is_some() {
                    let record = record.unwrap();
                    start_server(addrs,
                                 chain,
                                 Protocol::Https {
                                     certificate: record.full_chain
                                         .unwrap_or(record.cert_file),
                                     key: record.private_key_file,
                                 });
                    break;
                }
                thread::sleep(Duration::new(10, 0));
            }
        } else {
            start_server(addrs, chain, Protocol::Http);
        }
    }
}

fn start_server(addrs: Vec<SocketAddr>, chain: Chain, protocol: Protocol) {

    thread::Builder::new()
        .name("HttpServer".to_owned())
        .spawn(move || {
            Iron::new(chain)
                .listen_with(addrs[0], THREAD_COUNT, protocol, None)
                .unwrap();
        })
        .unwrap();
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

    it "should response 204 NoContent" {
        let response = request::get("http://localhost:3000/ping",
                                    Headers::new(),
                                    &mount).unwrap();
        assert_eq!(response.status.unwrap(), Status::NoContent);
    }
}

#[cfg(test)]
describe! http_server {
    before_each {
        extern crate hyper;

        use foxbox_taxonomy::manager::AdapterManager;
        use std::thread;
        use std::sync::Arc;
        use std::time::Duration;
        use stubs::controller::ControllerStub;

        let taxo_manager = Arc::new(AdapterManager::new(None));

        let mut http_server = HttpServer::new(ControllerStub::new());
        http_server.start(&taxo_manager);
        // HACK: Let some time for the http server to start.
        thread::sleep(Duration::new(3, 0));
    }

    it "should get the appropriate CORS headers" {
        use iron::headers;
        use iron::method::Method;

        let endpoints = vec![
            (vec![Method::Get], "ping".to_owned())
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

    it "should respond with 404" {
        use iron::status::Status;
        use std::io::Read;

        let client = hyper::Client::new();
        let path = "http://localhost:3000/foo/bar".to_owned();
        let mut res = client.get(&path).send().unwrap();
        assert_eq!(res.status, Status::NotFound);
        let mut body = String::new();
        res.read_to_string(&mut body).unwrap();
        assert_eq!(body, "Unknown resource: No such file or \
                   directory (os error 2)".to_owned());
    }
}
