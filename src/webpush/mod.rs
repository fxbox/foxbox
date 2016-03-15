/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

pub mod crypto;

use std::collections::HashMap;
use std::thread;
use std::sync::{ Arc, Mutex };

use iron::status;
use router::Router;
use iron::prelude::*;
use rustc_serialize::json;
/*
use foxbox_users::users_db::{ UsersDb, ReadFilter };
use iron::middleware::Handler;
use staticfile::Static;
use std::path::Path;*/
use hyper::header::{ ContentEncoding, Encoding };
use hyper::Client;
use hyper::client::Body;
use std::io::Read;

header! { (Encryption, "Encryption") => [String] }
header! { (EncryptionKey, "Encryption-Key") => [String] }

pub struct WebPushRouter;

impl WebPushRouter {
    pub fn create(web_push: Arc<WebPush>) -> Router {
        let mut router = Router::new();

        let wp1 = web_push.clone();
        router.post("subscribe", move |req1: &mut Request| -> IronResult<Response> {
            #[derive(RustcDecodable, Debug)]
            struct SubscribeBody {
                username: String,
                pushuri: String,
                publickey: String,
                groups: Vec<String>
            }

            let mut payload = String::new();
            req1.body.read_to_string(&mut payload).unwrap();
            let body: SubscribeBody = match json::decode(&payload) {
                Ok(body) => body,
                Err(e) => {
                    warn!("invalid subscribe payload {:?}", e);
                    return Ok(Response::with(status::BadRequest));
                }
            };

            let pushuri = if body.pushuri.is_empty() {
                None
            } else {
                Some(body.pushuri)
            };
            let publickey = if body.publickey.is_empty() {
                None
            } else {
                Some(body.publickey)
            };

            wp1.add_user(body.username.clone(), pushuri, publickey);
            for group in &body.groups {
                wp1.add_group(&body.username, group.clone());
            }

            Ok(Response::with(status::Ok))
        });

        let wp2 = web_push.clone();
        router.post("unsubscribe", move |req2: &mut Request| -> IronResult<Response> {
            #[derive(RustcDecodable, Debug)]
            struct UnsubscribeBody {
                username: String,
                groups: Vec<String>
            }

            let mut payload = String::new();
            req2.body.read_to_string(&mut payload).unwrap();
            let body: UnsubscribeBody = match json::decode(&payload) {
                Ok(body) => body,
                Err(e) => {
                    warn!("invalid unsubscribe payload {:?}", e);
                    return Ok(Response::with(status::BadRequest));
                }
            };

            for group in &body.groups {
                wp2.remove_group(&body.username, group);
            }

            Ok(Response::with(status::Ok))
        });

        let wp3 = web_push.clone();
        router.post("test", move |_: &mut Request| -> IronResult<Response> {
            wp3.notify(String::from("mygroup"), String::from("mymessage"));
            Ok(Response::with(status::Ok))
        });

        router
    }
}

struct Subscription {
    user: String,
    url: Option<String>,
    public_key: Option<String>,
    groups: HashMap<String, String>
}

impl Subscription {
    fn notify(&self, grp: &String, msg: &String) {
        info!("notify user={}", self.user);

        // Not all users will be subscribed for these notifications nor will they
        // always have an active subscription
        if !self.groups.contains_key(grp) {
            return;
        }

        let url = match self.url {
            Some(ref x) => x.clone(),
            None => { return; }
        };

        let public_key = match self.public_key {
            Some(ref x) => x.clone(),
            None => { return; }
        };

        info!("prepare notify user={}", self.user);
        let enc = match self::crypto::encrypt(&public_key, msg) {
            Some(x) => x,
            None => {
                warn!("failed to encrypt {} for {}", msg, self.user);
                return;
            }
        };

        let client = Client::new();
        let res = match client.post(&url)
            .header(ContentEncoding(vec![Encoding::EncodingExt(String::from("aesgcm128"))]))
            .header(EncryptionKey(format!("keyid=p256dh;dh={}", enc.public_key)))
            .header(Encryption(format!("keyid=p256dh;salt={}", enc.salt)))
            .body(Body::BufBody(&enc.output, enc.output.len()))
            .send() {
                Ok(x) => x,
                Err(e) => { warn!("failed to push to {} at {}: {:?}", self.user, url, e); return; }
            };

        info!("notified {} of {} ({:?})", self.user, grp, res.status);
    }
}

pub struct WebPush {
    subscriptions: Arc<Mutex<HashMap<String, Subscription>>>
}

impl WebPush {
    pub fn new() -> Self {
        WebPush {
            subscriptions: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn add_user(&self, user: String, url: Option<String>, public_key: Option<String>) {
        info!("add/update user={} url={:?} public_key={:?}", user, url, public_key);
        let mut subs = self.subscriptions.lock().unwrap();
        if let Some(mut sub) = subs.get_mut(&user) {
            sub.url = url;
            sub.public_key = public_key;
            return;
        }

        subs.insert(user.clone(), Subscription {
            user: user,
            url: url,
            public_key: public_key,
            groups: HashMap::new()
        });
    }

    pub fn remove_user(&self, user: &String) {
        self.subscriptions.lock().unwrap().remove(user);
    }

    pub fn add_group(&self, user: &String, grp: String) {
        info!("add group user={} group={}", user, grp);
        if let Some(mut sub) = self.subscriptions.lock().unwrap().get_mut(user) {
            sub.groups.insert(grp.clone(), grp.clone());
        }
    }

    pub fn remove_group(&self, user: &String, grp: &String) {
        info!("remove group user={} group={}", user, grp);
        if let Some(mut sub) = self.subscriptions.lock().unwrap().get_mut(user) {
            sub.groups.remove(grp);
        }
    }

    pub fn notify(&self, grp: String, msg: String) {
        info!("notify group={} msg={}", grp, msg);
        let json = String::from("test");//json!({group: grp, message: msg});
        let subs = self.subscriptions.clone();

        thread::spawn(move || {
            for sub in subs.lock().unwrap().values() {
                sub.notify(&grp, &json);
            }
        });
    }
}
