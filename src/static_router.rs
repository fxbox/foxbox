/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use foxbox_users::users_db::{ UsersDb, ReadFilter };
use iron::middleware::Handler;
use iron::prelude::*;
use iron::status;
use router::Router;
use staticfile::Static;
use std::path::Path;

fn handler(req: &mut Request) -> IronResult<Response> {
    let db = UsersDb::new();
    let handler = match db.read(ReadFilter::IsAdmin(true)) {
        Ok(users) => {
            if users.is_empty() {
                Static::new(Path::new("static/setup"))
            } else {
                Static::new(Path::new("static/main"))
            }
        },
        Err(_) => {
            return Ok(Response::with(status::InternalServerError));
        }
    };
    Handler::handle(&handler, req)
}

pub fn create() -> Router {
    let mut router = Router::new();
    router.any("", handler);
    router.any("*", handler);
    router
}
