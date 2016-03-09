/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use controller::Controller;
use foxbox_users::users_router::UsersRouter;
use iron::{ Iron, Request, Response, IronResult, AfterMiddleware, Chain };
use iron::error::{ IronError };
use iron::status::Status;
use mount::Mount;
use router::NoRoute;
use service_router;
use static_router;
use std::thread;

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

pub struct HttpServer<T: Controller> {
    controller: T
}

impl<T: Controller> HttpServer<T> {
    pub fn new(controller: T) -> Self {
        HttpServer { controller: controller }
    }

    pub fn start(&mut self) {
        let router = service_router::create(self.controller.clone());

        let mut mount = Mount::new();
        mount.mount("/", static_router::create())
             .mount("/services", router)
             .mount("/users", UsersRouter::init());

        let mut chain = Chain::new(mount);
        chain.link_after(Custom404);

        let addrs: Vec<_> = self.controller.http_as_addrs().unwrap().collect();

        thread::Builder::new().name("HttpServer".to_owned())
                              .spawn(move || {
            Iron::new(chain).http(addrs[0]).unwrap();
        }).unwrap();
    }
}
