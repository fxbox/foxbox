// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use foxbox_users::{UsersManager, UsersDb, ReadFilter};
use iron::middleware::Handler;
use iron::prelude::*;
use iron::status;
use router::Router;
use staticfile::Static;
use std::path::Path;
use std::sync::Arc;

fn handler(req: &mut Request, db: &UsersDb) -> IronResult<Response> {
    let handler = match db.read(ReadFilter::IsAdmin(true)) {
        Ok(users) => {
            if users.is_empty() {
                Static::new(Path::new("static/setup"))
            } else {
                Static::new(Path::new("static/main"))
            }
        }
        Err(_) => {
            return Ok(Response::with(status::InternalServerError));
        }
    };
    Handler::handle(&handler, req)
}

pub fn create(manager: Arc<UsersManager>) -> Router {
    let mut router = Router::new();
    let usersmanager = manager.clone();
    router.any("",
               move |req: &mut Request| -> IronResult<Response> {
                   handler(req, &usersmanager.get_db())
               },
               "_empty_");
    let usersmanager = manager.clone();
    router.any("*",
               move |req: &mut Request| -> IronResult<Response> {
                   handler(req, &usersmanager.get_db())
               },
               "_any_");
    router
}
