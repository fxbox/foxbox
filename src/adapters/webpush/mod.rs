/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Adapter for WebPush.
//!
//! Implemented as described in the draft IETF RFC:
//! https://tools.ietf.org/html/draft-ietf-webpush-protocol-04
//!
//! Encryption and sending of push notifications is controlled by the
//! "webpush" build feature. Older versions of OpenSSL (< 1.0.0) are
//! missing the necessary APIs to support the implementation.
//!

#[cfg(feature = "webpush")]
mod crypto;
mod db;

use foxbox_taxonomy::adapter::*;
use foxbox_taxonomy::api::{ Error, InternalError };
use foxbox_taxonomy::values::{ Range, Type, Value, Json };
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::util::Id;
#[cfg(feature = "webpush")]
use hyper::header::{ ContentEncoding, Encoding };
#[cfg(feature = "webpush")]
use hyper::Client;
#[cfg(feature = "webpush")]
use hyper::client::Body;
use rusqlite::{ self };
use serde_json;
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use std::thread;
use traits::Controller;
use transformable_channels::mpsc::*;

header! { (Encryption, "Encryption") => [String] }
header! { (EncryptionKey, "Encryption-Key") => [String] }

static ADAPTER_NAME: &'static str = "WebPush adapter (built-in)";
static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
static ADAPTER_VERSION: [u32;4] = [0, 0, 0, 0];

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Subscription {
    pub push_uri: String,
    pub public_key: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionGetter {
    subscriptions: Vec<Subscription>
}

impl SubscriptionGetter {
    fn new(subs: Vec<Subscription>) -> Self {
        SubscriptionGetter {
            subscriptions: subs
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceGetter {
    resources: Vec<String>
}

impl ResourceGetter {
    fn new(res: Vec<String>) -> Self {
        ResourceGetter {
            resources: res
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct NotifySetter {
    resource: String,
    message: String
}

impl Subscription {
    #[cfg(feature = "webpush")]
    fn notify(&self, message: &str) {
        let enc = match self::crypto::encrypt(&self.public_key, message.to_owned()) {
            Some(x) => x,
            None => {
                warn!("notity subscription {} failed for {}", self.push_uri, message);
                return;
            }
        };

        let client = Client::new();
        let res = match client.post(&self.push_uri)
            .header(ContentEncoding(vec![Encoding::EncodingExt(String::from("aesgcm128"))]))
            .header(EncryptionKey(format!("keyid=p256dh;dh={}", enc.public_key)))
            .header(Encryption(format!("keyid=p256dh;salt={}", enc.salt)))
            .body(Body::BufBody(&enc.output, enc.output.len()))
            .send() {
                Ok(x) => x,
                Err(e) => { warn!("notify subscription {} failed: {:?}", self.push_uri, e); return; }
            };

        info!("notified subscription {} (status {:?})", self.push_uri, res.status);
    }

    #[cfg(not(feature = "webpush"))]
    fn notify(&self, _: &str) {
        warn!("discard notification for subscription {}, webpush disabled at build time", self.push_uri);
    }
}

pub struct WebPush<C> {
    controller: C,
    getter_resource_id: Id<Getter>,
    getter_subscription_id: Id<Getter>,
    setter_resource_id: Id<Setter>,
    setter_subscribe_id: Id<Setter>,
    setter_unsubscribe_id: Id<Setter>,
    setter_notify_id: Id<Setter>,
}

impl<C: Controller> WebPush<C> {
    pub fn id() -> Id<AdapterId> {
        Id::new("webpush@link.mozilla.org")
    }

    pub fn service_webpush_id() -> Id<ServiceId> {
        Id::new("service:webpush@link.mozilla.org")
    }

    pub fn getter_resource_id() -> Id<Getter> {
        Id::new("getter:resource.webpush@link.mozilla.org")
    }

    pub fn getter_subscription_id() -> Id<Getter> {
        Id::new("getter:subscription.webpush@link.mozilla.org")
    }

    pub fn setter_resource_id() -> Id<Setter> {
        Id::new("setter:resource.webpush@link.mozilla.org")
    }

    pub fn setter_subscribe_id() -> Id<Setter> {
        Id::new("setter:subscribe.webpush@link.mozilla.org")
    }

    pub fn setter_unsubscribe_id() -> Id<Setter> {
        Id::new("setter:unsubscribe.webpush@link.mozilla.org")
    }

    pub fn setter_notify_id() -> Id<Setter> {
        Id::new("setter:notify.webpush@link.mozilla.org")
    }
}

impl<C: Controller> Adapter for WebPush<C> {
    fn id(&self) -> Id<AdapterId> {
        Self::id()
    }

    fn name(&self) -> &str {
        ADAPTER_NAME
    }

    fn vendor(&self) -> &str {
        ADAPTER_VENDOR
    }

    fn version(&self) -> &[u32;4] {
        &ADAPTER_VERSION
    }

    fn fetch_values(&self, mut set: Vec<Id<Getter>>) -> ResultMap<Id<Getter>, Option<Value>, Error> {
        set.drain(..).map(|id| {
            let user_id = 1; // FIXME: currently logged in user

            macro_rules! getter_api {
                ($getter:ident, $getter_id:ident, $getter_type:ident) => (
                    if id == self.$getter_id {
                        match self.$getter(user_id) {
                            Ok(data) => {
                                let rsp = $getter_type::new(data);
                                return (id, Ok(Some(Value::Json(Arc::new(Json(serde_json::to_value(&rsp)))))));
                            },
                            Err(_) => return (id, Err(Error::InternalError(InternalError::InvalidInitialService)))
                        };
                    }
                )
            }

            getter_api!(get_subscriptions, getter_subscription_id, SubscriptionGetter);
            getter_api!(get_resources, getter_resource_id, ResourceGetter);
            (id.clone(), Err(Error::InternalError(InternalError::NoSuchGetter(id))))
        }).collect()
    }

    fn send_values(&self, mut values: HashMap<Id<Setter>, Value>) -> ResultMap<Id<Setter>, (), Error> {
        values.drain().map(|(id, value)| {
            let user_id = 1; // FIXME: currently logged in user

            let arc_json_value = match value {
                Value::Json(v) => v,
                _ => return (id, Err(Error::InternalError(InternalError::InvalidInitialService)))
            };
            let Json(ref json_value) = *arc_json_value;

            macro_rules! setter_api {
                ($setter:ident, $setter_id:ident, $setter_type:ident) => (
                    if id == self.$setter_id {
                        let data: Result<$setter_type, _> = serde_json::from_value(json_value.clone());
                        match data {
                            Ok(x) => {
                                self.$setter(user_id, &x).unwrap();
                                return (id, Ok(()));
                            }
                            Err(_) => return (id, Err(Error::InternalError(InternalError::InvalidInitialService)))
                        }
                    }
                )
            }

            setter_api!(set_resources, setter_resource_id, ResourceGetter);
            setter_api!(set_subscribe, setter_subscribe_id, SubscriptionGetter);
            setter_api!(set_unsubscribe, setter_unsubscribe_id, SubscriptionGetter);
            setter_api!(set_notify, setter_notify_id, NotifySetter);
            (id.clone(), Err(Error::InternalError(InternalError::NoSuchSetter(id))))
        }).collect()
    }

    fn register_watch(&self, mut watch: Vec<(Id<Getter>, Option<Range>)>,
        _: Box<ExtSender<WatchEvent>>) ->
           ResultMap<Id<Getter>, Box<AdapterWatchGuard>, Error>
    {
        watch.drain(..).map(|(id, _)| {
            (id.clone(), Err(Error::GetterDoesNotSupportWatching(id)))
        }).collect()
    }
}

impl<C: Controller> WebPush<C> {
    pub fn init<A: AdapterManagerHandle>(controller: C, adapt: &A) -> Result<(), Error> {
        let wp = Box::new(Self::new(controller));
        let id = WebPush::<C>::id();
        let service_id = WebPush::<C>::service_webpush_id();
        let getter_resource_id = wp.getter_resource_id.clone();
        let getter_subscription_id = wp.getter_subscription_id.clone();
        let setter_resource_id = wp.setter_resource_id.clone();
        let setter_subscribe_id = wp.setter_subscribe_id.clone();
        let setter_unsubscribe_id = wp.setter_unsubscribe_id.clone();
        let setter_notify_id = wp.setter_notify_id.clone();

        try!(adapt.add_adapter(wp));
        try!(adapt.add_service(Service::empty(service_id.clone(), id.clone())));

        macro_rules! add_getter {
            ($id:ident, $kind_id:expr) => (
                try!(adapt.add_getter(Channel {
                    tags: HashSet::new(),
                    adapter: id.clone(),
                    id: $id,
                    last_seen: None,
                    service: service_id.clone(),
                    mechanism: Getter {
                        kind: ChannelKind::Extension {
                            vendor: ADAPTER_VENDOR.to_owned(),
                            adapter: ADAPTER_NAME.to_owned(),
                            kind: $kind_id.to_owned(),
                            typ: Type::Json,
                        },
                        updated: None
                    }
                }));
            )
        }

        macro_rules! add_setter {
            ($id:ident, $kind_id:expr) => (
                try!(adapt.add_setter(Channel {
                    tags: HashSet::new(),
                    adapter: id.clone(),
                    id: $id,
                    last_seen: None,
                    service: service_id.clone(),
                    mechanism: Setter {
                        kind: ChannelKind::Extension {
                            vendor: ADAPTER_VENDOR.to_owned(),
                            adapter: ADAPTER_NAME.to_owned(),
                            kind: $kind_id.to_owned(),
                            typ: Type::Json,
                        },
                        updated: None
                    }
                }));
            )
        }

        add_getter!(getter_resource_id, "WebPushResource");
        add_getter!(getter_subscription_id, "WebPushSubscription");
        add_setter!(setter_resource_id, "WebPushResource");
        add_setter!(setter_subscribe_id, "WebPushSubscription");
        add_setter!(setter_unsubscribe_id, "WebPushSubscription");
        add_setter!(setter_notify_id, "WebPushNotify");
        Ok(())
    }

    fn new(controller: C) -> Self
    {
        WebPush {
            controller: controller,
            getter_resource_id: Self::getter_resource_id(),
            getter_subscription_id: Self::getter_subscription_id(),
            setter_resource_id: Self::setter_resource_id(),
            setter_subscribe_id: Self::setter_subscribe_id(),
            setter_unsubscribe_id: Self::setter_unsubscribe_id(),
            setter_notify_id: Self::setter_notify_id(),
        }
    }

    fn get_db(&self) -> db::WebPushDb {
        db::WebPushDb::new(&self.controller.get_profile().path_for("webpush.sqlite"))
    }

    fn set_subscribe(&self, user_id: i32, setter: &SubscriptionGetter) -> rusqlite::Result<()> {
        let db = self.get_db();
        for sub in &setter.subscriptions {
            try!(db.subscribe(user_id, sub));
        }
        Ok(())
    }

    fn set_unsubscribe(&self, user_id: i32, setter: &SubscriptionGetter) -> rusqlite::Result<()> {
        let db = self.get_db();
        for sub in &setter.subscriptions {
            try!(db.unsubscribe(user_id, &sub.push_uri));
        }
        Ok(())
    }

    fn set_resources(&self, user_id: i32, setter: &ResourceGetter) -> rusqlite::Result<()> {
        try!(self.get_db().set_resources(user_id, &setter.resources));
        Ok(())
    }

    fn get_resources(&self, user_id: i32) -> rusqlite::Result<Vec<String>> {
        self.get_db().get_resources(user_id)
    }

    fn get_subscriptions(&self, user_id: i32) -> rusqlite::Result<Vec<Subscription>> {
        self.get_db().get_subscriptions(user_id)
    }

    fn get_resource_subscriptions(&self, resource: &str) -> rusqlite::Result<Vec<Subscription>> {
        self.get_db().get_resource_subscriptions(resource)
    }

    fn set_notify(&self, _: i32, setter: &NotifySetter) -> rusqlite::Result<()> {
        info!("notify on resource {}: {}", setter.resource, setter.message);

        let json = json!({resource: setter.resource, message: setter.message});
        let subscriptions = try!(self.get_resource_subscriptions(&setter.resource));
        if subscriptions.is_empty() {
            debug!("no users listening on push resource");
        } else {
            thread::spawn(move || {
                for sub in subscriptions {
                    sub.notify(&json);
                }
            });
        }
        Ok(())
    }
}
