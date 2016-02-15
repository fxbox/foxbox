/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use context::{ ContextTrait, Shared };
use service_router::ServiceRouter;
use foxbox_users::users_router::UsersRouter;
use iron::Iron;
use mount::Mount;
use staticfile::Static;
use std::path::Path;
use std::thread;
use core::marker::Reflect;

pub struct HttpServer<Ctx> where Ctx: ContextTrait {
    context: Shared<Ctx>,
}

impl<Ctx> HttpServer<Ctx> where Ctx: Send + Reflect + ContextTrait + 'static {
    pub fn new(context: Shared<Ctx>) -> HttpServer<Ctx> {
        HttpServer { context: context }
    }

    pub fn start(&mut self) {
        let router = ServiceRouter::new(self.context.clone()).generate_router();

        let mut mount = Mount::new();
        mount.mount("/", Static::new(Path::new("static")))
             .mount("/services", router)
             .mount("/users_admin", UsersRouter::new());

        let thread_context = self.context.clone();
        let ctx = thread_context.lock().unwrap();
        let addrs: Vec<_> = ctx.http_as_addrs().unwrap().collect();

        thread::Builder::new().name("HttpServer".to_owned())
                              .spawn(move || {
            Iron::new(mount).http(addrs[0]).unwrap();
        }).unwrap();
    }
}
