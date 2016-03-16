/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

pub mod crypto;

use std::thread;
use std::sync::Arc;

use iron::status;
use router::Router;
use iron::prelude::*;
use iron::method::Method;
use rustc_serialize::json;
use controller::Controller;
use foxbox_users::{ AuthEndpoint, ReadFilter, User, UsersManager };
use hyper::header::{ ContentEncoding, Encoding };
use hyper::Client;
use hyper::client::Body;
use std::io::Read;

header! { (Encryption, "Encryption") => [String] }
header! { (EncryptionKey, "Encryption-Key") => [String] }

pub struct WebPushRouter;

impl WebPushRouter {
    pub fn create<T: Controller>(controller: T) -> Chain {
        let mut router = Router::new();

        let ctl = controller.clone();
        router.post("subscribe", move |req: &mut Request| -> IronResult<Response> {
            #[derive(RustcDecodable, Debug)]
            struct SubscribeBody {
                user: Option<String>,
                push_uri: String,
                push_key: String,
                resources: Vec<String>
            }

            let mut payload = String::new();
            req.body.read_to_string(&mut payload).unwrap();
            let body: SubscribeBody = match json::decode(&payload) {
                Ok(body) => body,
                Err(e) => {
                    warn!("invalid subscribe payload {:?}", e);
                    return Ok(Response::with(status::BadRequest));
                }
            };

            let db = ctl.get_users_manager().get_db();
            let mut user = if cfg!(feature = "authentication") {
                req.extensions.get::<User>().unwrap().clone()
            } else {
                let name = match body.user {
                    Some(n) => n,
                    None => {
                        warn!("not using auth and no user provided");
                        return Ok(Response::with(status::BadRequest));
                    }
                };

                let mut users = match db.read(ReadFilter::Name(name)) {
                    Ok(u) => u,
                    Err(_) => Vec::new()
                };

                if users.len() != 1 {
                    warn!("user not found in database");
                    return Ok(Response::with(status::BadRequest));
                }

                users.pop().unwrap().clone()
            };

            user.push_uri = if body.push_uri.is_empty() {
                None
            } else {
                Some(body.push_uri)
            };
            user.push_key = if body.push_key.is_empty() {
                None
            } else {
                Some(body.push_key)
            };

            let id = user.id.unwrap();
            if let Err(e) = db.update(id, &user) {
                warn!("cannot update push subscription for user {}: {:?}", user.name, e);
                return Ok(Response::with(status::InternalServerError));
            }
            if let Err(e) = db.update_push_resources(id, &body.resources) {
                warn!("cannot update push resources for user {}: {:?}", user.name, e);
                return Ok(Response::with(status::InternalServerError));
            }

            Ok(Response::with(status::Ok))
        });

        /*
        let wp = controller.get_web_push();
        router.post("test", move |_: &mut Request| -> IronResult<Response> {
            wp.notify(String::from("mygroup"), String::from("mymessage"));
            Ok(Response::with(status::Ok))
        });*/

        let auth_endpoints = if cfg!(feature = "authentication") {
            vec![
                AuthEndpoint(vec![Method::Post], "subscribe".to_owned())
            ]
        } else {
            vec![]
        };

        let mut chain = Chain::new(router);
        chain.around(controller.get_users_manager().get_middleware(auth_endpoints));

        chain
    }
}

pub struct WebPush {
    users_manager: Arc<UsersManager>
}

impl WebPush {
    pub fn new(users_manager: Arc<UsersManager>) -> Self {
        WebPush {
            users_manager: users_manager
        }
    }

    pub fn notify(&self, resource: String, message: String) {
        info!("notify on resource {}: {}", resource, message);

        let json = json!({resource: resource, message: message});
        let db = self.users_manager.get_db();
        let users = match db.read(ReadFilter::PushResource(resource)) {
            Ok(u) => u,
            Err(_) => Vec::new()
        };
        if users.is_empty() {
            debug!("no users listening on push resource");
            return;
        }

        thread::spawn(move || {
            for user in users {
                WebPush::notify_user(user, &json);
            }
        });
    }

    fn notify_user(user: User, message: &String) {
        let enc = match self::crypto::encrypt(&user.push_key.unwrap(), message.clone()) {
            Some(x) => x,
            None => {
                warn!("notity user {} failed for {}", user.name, message);
                return;
            }
        };

        let uri = user.push_uri.unwrap();
        let client = Client::new();
        let res = match client.post(&uri)
            .header(ContentEncoding(vec![Encoding::EncodingExt(String::from("aesgcm128"))]))
            .header(EncryptionKey(format!("keyid=p256dh;dh={}", enc.public_key)))
            .header(Encryption(format!("keyid=p256dh;salt={}", enc.salt)))
            .body(Body::BufBody(&enc.output, enc.output.len()))
            .send() {
                Ok(x) => x,
                Err(e) => { warn!("notify user {} via {} failed: {:?}", user.name, uri, e); return; }
            };

        info!("notified user {} (status {:?})", user.name, res.status);
    }
}
