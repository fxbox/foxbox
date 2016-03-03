/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use controller::Controller;
use service_router;
use static_router;
use foxbox_users::users_router::UsersRouter;
use iron::Iron;
use mount::Mount;
use std::thread;

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

        let addrs: Vec<_> = self.controller.http_as_addrs().unwrap().collect();

        thread::Builder::new().name("HttpServer".to_owned())
                              .spawn(move || {
            Iron::new(mount).http(addrs[0]).unwrap();
        }).unwrap();
    }
}
