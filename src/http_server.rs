/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use context::{ ContextTrait, Shared };
use service_router;
use foxbox_users::users_router::UsersRouter;
use iron::Iron;
use mount::Mount;
use staticfile::Static;
use std::path::Path;
use std::thread;
use core::marker::Reflect;

pub struct HttpServer<T> {
    context: Shared<T>,
}

impl<T> HttpServer<T> where T: Send + Reflect + ContextTrait + 'static {
    pub fn new(context: Shared<T>) -> HttpServer<T> {
        HttpServer { context: context }
    }

    pub fn start(&mut self) {
        let handler = self.create_handler_and_its_routes();

        let thread_context = self.context.clone();
        let ctx = thread_context.lock().unwrap();
        let addrs: Vec<_> = ctx.http_as_addrs().unwrap().collect();

        thread::Builder::new().name("HttpServer".to_owned())
                              .spawn(move || {
            Iron::new(handler).http(addrs[0]).unwrap();
        }).unwrap();
    }

    fn create_handler_and_its_routes(& self) -> Mount {
        let router = service_router::create(self.context.clone());

        let mut mount = Mount::new();
        mount.mount("/", Static::new(Path::new("static")))
             .mount("/services", router)
             .mount("/users_admin", UsersRouter::new());
        mount
    }
}

// FIXME: See explanation in service_router.rs
extern crate iron_test;
pub use self::iron_test::{request, response};

#[cfg(test)]
describe! http_server {
    before_each {
        use stubs::context::ContextStub;
        use iron::Headers;

        let context = ContextStub::blank_shared();
        let http_server = HttpServer::new(context);
        let handler = http_server.create_handler_and_its_routes();
    }

    it "should expose static files" {
        // unwrap() panics if there is no route
        // FIXME: Mock paths, instead of relying on static/index.html
        request::get("http://localhost:3000/index.html", Headers::new(), &handler).unwrap();
    }
}
