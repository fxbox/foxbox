//!
//! The API for communicating with devices.
//!
//! This module implements a JSON front-end for the API. This front-end is designed to implement
//! the REST API and for interaction with dynamic languages.
//! See also `native.rs` for a native front-end.
//!

use adapters::manager::*;
use api::error::*;
pub use api::native::{ TargetMap, Targetted, User };
use api::services::*;
use api::selector::*;
use io::parse::*;
use io::serialize::*;
use io::types::*;

use std::sync::Arc;

use transformable_channels::mpsc::*;

pub type WatchEvent = GenericWatchEvent<JSON>;

macro_rules! try_json {
    ($input: expr, $act: expr) => {
        match $act {
            Ok(ok) => ok,
            Err(err) => return Err(err.to_json(&*$input.serialize))
        }
    }
}

macro_rules! log_debug_assert {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            error!($($arg)*);
            panic!($($arg)*);
        }
    };
}

/// A struture used to (de)serialize values.
pub struct Request {
    pub json: JSON,
    pub deserialize: Arc<DeserializeSupport>,
    pub serialize: Arc<SerializeSupport>,
}

impl<K> Parser<Targetted<K, Vec<Id<TagId>>>> for Targetted<K, Vec<Id<TagId>>> where K: Parser<K> + Clone {
    fn description() -> String {
        format!("{{select: {}, tags: [tag_1, tag_2, ...]}}", K::description())
    }
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        if source.is_object() {
            // Default format: an object {select, value}.
            let select = try!(path.push("select", |path| Vec::<K>::take(path, source, "select", support)));
            let payload = try!(path.push("tags", |path| Vec::<Id<TagId>>::take(path, source, "tags", support)));
            Ok(Targetted {
                select: select,
                payload: payload
            })
        } else {
            Err(ParseError::type_error(&Self::description() as &str, &path, "an object {select, tags}"))
        }
    }
}


impl<K> Parser<Targetted<K, Option<JSON>>> for Targetted<K, Option<JSON>> where K: Parser<K> + Clone {
    fn description() -> String {
        format!("{{select: {}, value: value (optional)}}", K::description())
    }
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        if source.is_object() {
            // Default format: an object {select, value}.
            let select = try!(path.push("select", |path| Vec::<K>::take(path, source, "select", support)));
            let payload = match path.push("value", |path| JSON::take_opt(path, source, "value", support)) {
                None => None,
                Some(Err(err)) => return Err(err),
                Some(Ok(ok)) => Some(ok)
            };
            Ok(Targetted {
                select: select,
                payload: payload
            })
        } else {
            Err(ParseError::type_error(&Self::description() as &str, &path, "an object {select, value}"))
        }
    }
}


impl<K> Parser<Targetted<K, Exactly<JSON>>> for Targetted<K, Exactly<JSON>> where K: Parser<K> + Clone {
    fn description() -> String {
        format!("{{select: {}, when: value (optional)}}", K::description())
    }
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        if source.is_object() {
            // Default format: an object {select, value}.
            let select = try!(path.push("select", |path| Vec::<K>::take(path, source, "select", support)));
            let payload = match path.push("when", |path| JSON::take_opt(path, source, "when", support)) {
                None => {
                    match source.find("never") {
                        None => Exactly::Always,
                        Some(&JSON::Null) => Exactly::Always,
                        Some(&JSON::Bool(false)) => Exactly::Always,
                        _ => Exactly::Never
                    }
                }
                Some(Err(err)) => return Err(err),
                Some(Ok(ok)) => Exactly::Exactly(ok)
            };
            Ok(Targetted {
                select: select,
                payload: payload
            })
        } else {
            Err(ParseError::type_error(&Self::description() as &str, &path, "an object {select, when} or {select, never: true}"))
        }
    }
}

impl ToJSON for (Id<FeatureId>, Result<Option<JSON>, Error>) {
    fn to_json(&self, parts: &SerializeSupport) -> JSON
    {
        use std::collections::BTreeMap;
        let (ref id, ref result) = *self;
        let mut map = BTreeMap::new();
        let result = match *result {
            Ok(ref json) => json.to_json(parts),
            Err(ref err) => vec![("error", err)].to_json(parts)
        };
        map.insert(id.to_string(), result);
        JSON::Object(map)
    }
}

/// A handle to the public API.
pub struct API {
    manager: AdapterManager
}
impl API {
    pub fn new(manager: &AdapterManager) -> Self {
        API {
            manager: (*manager).clone()
        }
    }
}
impl API {
    /// Get the metadata on services matching some conditions.
    ///
    /// A call to `API::get_services(vec![req1, req2, ...])` will return
    /// the metadata on all services matching _either_ `req1` or `req2`
    /// or ...
    ///
    /// # REST API
    ///
    /// `GET /api/v1/services`
    ///
    /// ### JSON
    ///
    /// This call accepts as JSON argument a vector of `ServiceSelector`. See the documentation
    /// of `ServiceSelector` for more details.
    ///
    /// Example: Select all doors in the entrance (tags `door`, `entrance`)
    /// that implement feature `door/is-open`
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate serde_json;
    ///
    /// use foxbox_taxonomy::api::json::*;
    /// use foxbox_taxonomy::io::parse::*;
    /// use foxbox_taxonomy::io::serialize::*;
    /// use foxbox_taxonomy::adapters::manager::AdapterManager;
    ///
    /// use std::sync::Arc;
    ///
    /// # fn main() {
    /// let manager = AdapterManager::new(None);
    /// let api = API::new(&manager);
    ///
    /// let source = r#"[{
    ///   "tags": ["entrance", "door"],
    ///   "features": [{
    ///     "implements": "door/is-open"
    ///   }]
    /// }]"#;
    ///
    /// let request = Request {
    ///   json: serde_json::from_str(&source).unwrap(),
    ///   deserialize: Arc::new(EmptyDeserializeSupportForTests),
    ///   serialize: Arc::new(EmptySerializeSupportForTests),
    /// };
    ///
    /// api.get_services(request).unwrap();
    /// # }
    /// ```
    ///
    ///
    /// ## Errors
    ///
    /// In case of syntax error, Error 400, accompanied with a
    /// somewhat human-readable JSON string detailing the error.
    ///
    /// ## Success
    ///
    /// A JSON representing an array of `Service`. See the implementation
    /// of `Service` for details.
    pub fn get_services(&self, input: Request) -> Result<JSON, JSON> {
        let selectors = try_json!(input,
            Vec::<ServiceSelector>::parse(Path::named("body"), &input.json, &*input.deserialize)
        );
        let returned = self.manager.get_services(selectors);
        Ok(returned.to_json(&*input.serialize))
    }

    /// Label a set of services with a set of tags.
    ///
    /// A call to `API::put_service_tag(vec![req1, req2, ...], vec![tag1,
    /// ...])` will label all the services matching _either_ `req1` or
    /// `req2` or ... with `tag1`, ... and return the number of services
    /// matching any of the selectors.
    ///
    /// Some of the services may already be labelled with `tag1`, or
    /// `tag2`, ... They will not change state. They are counted in
    /// the resulting `usize` nevertheless.
    ///
    /// Note that this call is _not live_. In other words, if services
    /// are added after the call, they will not be affected.
    ///
    /// # REST API
    ///
    /// `POST /api/v1/services/tag`
    ///
    /// ## JSON
    ///
    /// A JSON object with the following fields:
    /// - services: array - an array of ServiceSelector;
    /// - tags: array - an array of string
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate serde_json;
    ///
    /// use foxbox_taxonomy::api::json::*;
    /// use foxbox_taxonomy::io::parse::*;
    /// use foxbox_taxonomy::io::serialize::*;
    /// use foxbox_taxonomy::adapters::manager::AdapterManager;
    ///
    /// use std::sync::Arc;
    ///
    /// # fn main() {
    /// let manager = AdapterManager::new(None);
    /// let api = API::new(&manager);
    ///
    /// let source = r#"{
    ///   "select": [{
    ///     "id": "id_1"
    ///   }, {
    ///     "id": "id_2"
    ///   }],
    ///   "tags": ["entrance", "door"]
    /// }"#;
    ///
    /// let request = Request {
    ///   json: serde_json::from_str(&source).unwrap(),
    ///   deserialize: Arc::new(EmptyDeserializeSupportForTests),
    ///   serialize: Arc::new(EmptySerializeSupportForTests),
    /// };
    ///
    /// api.add_service_tags(request).unwrap();
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// In case of syntax error, Error 400, accompanied with a
    /// somewhat human-readable JSON string detailing the error.
    ///
    /// ## Success
    ///
    /// A JSON string representing a number.
    pub fn add_service_tags(&self, input: Request) -> Result<JSON, JSON> {
        let Targetted { select, payload } = try_json!(input,
            Targetted::<ServiceSelector, Vec<Id<TagId>>>::parse(Path::named("body"), &input.json, &*input.deserialize)
        );
        let returned = self.manager.add_service_tags(select, payload);
        Ok(returned.to_json(&*input.serialize))
    }

    /// Remove a set of tags from a set of services.
    ///
    /// A call to `API::delete_service_tag(vec![req1, req2, ...], vec![tag1,
    /// ...])` will remove from all the services matching _either_ `req1` or
    /// `req2` or ... all of the tags `tag1`, ... and return the number of services
    /// matching any of the selectors.
    ///
    /// Some of the services may not be labelled with `tag1`, or `tag2`,
    /// ... They will not change state. They are counted in the
    /// resulting `usize` nevertheless.
    ///
    /// Note that this call is _not live_. In other words, if services
    /// are added after the call, they will not be affected.
    ///
    /// # REST API
    ///
    /// `DELETE /api/v1/services/tag`
    ///
    /// ## JSON
    ///
    /// A JSON object with the following fields:
    /// - services: array - an array of ServiceSelector;
    /// - tags: array - an array of string
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate serde_json;
    ///
    /// use foxbox_taxonomy::api::json::*;
    /// use foxbox_taxonomy::io::parse::*;
    /// use foxbox_taxonomy::io::serialize::*;
    /// use foxbox_taxonomy::adapters::manager::AdapterManager;
    ///
    /// use std::sync::Arc;
    ///
    /// # fn main() {
    /// let manager = AdapterManager::new(None);
    /// let api = API::new(&manager);
    ///
    /// let source = r#"{
    ///   "select": [{
    ///     "id": "id_1"
    ///   }, {
    ///     "id": "id_2"
    ///   }],
    ///   "tags": ["entrance", "door"]
    /// }"#;
    ///
    /// let request = Request {
    ///   json: serde_json::from_str(&source).unwrap(),
    ///   deserialize: Arc::new(EmptyDeserializeSupportForTests),
    ///   serialize: Arc::new(EmptySerializeSupportForTests),
    /// };
    ///
    /// api.add_service_tags(request).unwrap();
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// In case of syntax error, Error 400, accompanied with a
    /// somewhat human-readable JSON string detailing the error.
    ///
    /// ## Success
    ///
    /// A JSON string representing a number.
    pub fn remove_service_tags(&self, input: Request) -> Result<JSON, JSON> {
        let Targetted { select, payload } = try_json!(input,
            Targetted::<ServiceSelector, Vec<Id<TagId>>>::parse(Path::named("body"), &input.json, &*input.deserialize)
        );
        let returned = self.manager.remove_service_tags(select, payload);
        Ok(returned.to_json(&*input.serialize))
    }

    /// Get a list of getters matching some conditions
    ///
    /// # REST API
    ///
    /// `GET /api/v1/channels/getters`
    ///
    /// ### JSON
    ///
    /// This call accepts as JSON argument a vector of `GetterSelector`. See the documentation
    /// of `GetterSelector` for more details.
    ///
    /// Example: Select all doors in the entrance (tags `door`, `entrance`)
    /// that support setter channel `OpenClosed`
    ///
    /// ```
    /// extern crate foxbox_taxonomy;
    /// extern crate serde_json;
    ///
    /// use foxbox_taxonomy::api::json::*;
    /// use foxbox_taxonomy::io::parse::*;
    /// use foxbox_taxonomy::io::serialize::*;
    /// use foxbox_taxonomy::adapters::manager::AdapterManager;
    ///
    /// use std::sync::Arc;
    ///
    /// # fn main() {
    /// let manager = AdapterManager::new(None);
    /// let api = API::new(&manager);
    ///
    /// let source = r#"[{
    ///   "tags": ["entrance", "door"],
    ///   "implements": "door/is-open"
    /// }]"#;
    ///
    /// let request = Request {
    ///   json: serde_json::from_str(&source).unwrap(),
    ///   deserialize: Arc::new(EmptyDeserializeSupportForTests),
    ///   serialize: Arc::new(EmptySerializeSupportForTests),
    /// };
    ///
    /// api.get_features(request).unwrap();
    /// # }
    /// ```
    pub fn get_features(&self, input: Request) -> Result<JSON, JSON> {
        let selectors = try_json!(input,
            Vec::<FeatureSelector>::parse(Path::named("body"), &input.json, &*input.deserialize)
        );
        let returned = self.manager.get_features(selectors);
        Ok(returned.to_json(&*input.serialize))
    }


    /// Label a set of channels with a set of tags.
    ///
    /// A call to `API::put_{getter, setter}_tag(vec![req1, req2, ...], vec![tag1,
    /// ...])` will label all the channels matching _either_ `req1` or
    /// `req2` or ... with `tag1`, ... and return the number of channels
    /// matching any of the selectors.
    ///
    /// Some of the channels may already be labelled with `tag1`, or
    /// `tag2`, ... They will not change state. They are counted in
    /// the resulting `usize` nevertheless.
    ///
    /// Note that this call is _not live_. In other words, if channels
    /// are added after the call, they will not be affected.
    ///
    /// # REST API
    ///
    /// `POST /api/v1/channels/tag`
    ///
    /// ## Requests
    ///
    /// Any JSON that can be deserialized to
    ///
    /// ```ignore
    /// {
    ///   set: Vec<GetterSelector>,
    ///   tags: Vec<Id<TagId>>,
    /// }
    /// ```
    /// or
    /// ```ignore
    /// {
    ///   set: Vec<SetterSelector>,
    ///   tags: Vec<Id<TagId>>,
    /// }
    /// ```
    ///
    /// ## Errors
    ///
    /// In case of syntax error, Error 400, accompanied with a
    /// somewhat human-readable JSON string detailing the error.
    ///
    /// ## Success
    ///
    /// A JSON representing a number.
    pub fn add_feature_tags(&self, input: Request) -> Result<JSON, JSON> {
        let Targetted { select, payload } = try_json!(input,
            Targetted::<FeatureSelector, Vec<Id<TagId>>>::parse(Path::named("body"), &input.json, &*input.deserialize)
        );
        let returned = self.manager.add_feature_tags(select, payload);
        Ok(returned.to_json(&*input.serialize))
    }

    /// Remove a set of tags from a set of channels.
    ///
    /// A call to `API::delete_{getter, setter}_tag(vec![req1, req2, ...], vec![tag1,
    /// ...])` will remove from all the channels matching _either_ `req1` or
    /// `req2` or ... all of the tags `tag1`, ... and return the number of channels
    /// matching any of the selectors.
    ///
    /// Some of the channels may not be labelled with `tag1`, or `tag2`,
    /// ... They will not change state. They are counted in the
    /// resulting `usize` nevertheless.
    ///
    /// Note that this call is _not live_. In other words, if channels
    /// are added after the call, they will not be affected.
    ///
    /// ## Errors
    ///
    /// In case of syntax error, Error 400, accompanied with a
    /// somewhat human-readable JSON string detailing the error.
    ///
    /// ## Success
    ///
    /// A JSON representing a number.
    pub fn remove_feature_tags(&self, input: Request) -> Result<JSON, JSON> {
        let Targetted { select, payload } = try_json!(input,
            Targetted::<FeatureSelector, Vec<Id<TagId>>>::parse(Path::named("body"), &input.json, &*input.deserialize)
        );
        let returned = self.manager.remove_feature_tags(select, payload);
        Ok(returned.to_json(&*input.serialize))
    }

    pub fn place_method_call(&self, method: MethodCall, input: Request, user: User) ->
        Result<JSON, JSON>
    {
        let request : Vec<Targetted<FeatureSelector, Option<JSON>>> = try_json!(input,
            Vec::<Targetted<FeatureSelector, Option<JSON>>>::parse(Path::named("body"), &input.json, &*input.deserialize)
        );
        let serialize = input.serialize.clone();
        let deserialize = input.deserialize.clone();

        let returned = self.manager.place_method_call(method, request, user,
            // Decoder
            move |type_: &Arc<Format + 'static>, value: JSON| {
                let deserialize = deserialize.clone();
                let borrow = &*deserialize;
                type_.deserialize(Path::new(), &value, borrow).
                    map_err(|error| {Error::InternalError(InternalError::DeserializationError(error))})
            },
            // Encoder
            move |type_: &Arc<Format + 'static>, value: Value| {
                let serialize = serialize.clone();
                let borrow = &*serialize;
                type_.serialize(&value, borrow).
                    map_err(|error| {Error::InternalError(InternalError::SerializationError(error))})
            });
        Ok(returned.to_json(&*input.serialize))
    }

    pub fn register_watch(&self, input: Request,
        on_event: Box<ExtSender<WatchEvent>>) -> Result<WatchGuard, JSON>
    {
        let mut watch: Vec<Targetted<FeatureSelector, Exactly<JSON>>> = try_json!(input,
            Vec::<Targetted<FeatureSelector, Exactly<JSON>>>::parse(Path::named("body"), &input.json, &*input.deserialize)
        );

        // Convert `Exactly<JSON>` to `Exactly<Arc<AsValue>>`.
        let watch : TargetMap<_, Exactly<Arc<AsValue>>> = watch.drain(..)
            .map(|Targetted { select, payload }| {
                let payload = match payload {
                    Exactly::Always => Exactly::Always,
                    Exactly::Never => Exactly::Never,
                    Exactly::Exactly(json) => Exactly::Exactly(Arc::new(json) as Arc<AsValue>)
                };
                Targetted {
                    select: select,
                    payload: payload
                }
            }).collect();

        let serialize = input.serialize.clone();
        let on_event = Box::new(on_event.map(move |event: ManagerWatchEvent| {
            let serialize = serialize.clone();
            event.convert(|WatchEventInternals { value, format }| {
                format.serialize(&value, &*serialize)
                    .map_err(Error::SerializeError)
            })
        }));

        let deserialize = input.deserialize.clone();
        Ok(self.manager.register_watch(watch, on_event, deserialize))
    }
}
