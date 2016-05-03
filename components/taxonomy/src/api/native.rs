//!
//! The API for communicating with devices.
//!
//! This API is provided as Traits to be implemented:
//!
//! - by the low-level layers of the foxbox, including the adapters;
//! - by test suites and tools that need to simulate connected devices.
//!
//! In turn, this API is used to implement:
//!
//! - the public-facing REST and `WebSocket` API;
//! - the rules API (Thinkerbell).
//!
//!

use adapters::manager:: { AdapterManager, GenericWatchEvent, MethodCall, ManagerWatchEvent,
    PerFeatureResult, WatchEventInternals, WatchGuard };
use api::services::*;
use api::selector::*;
use io::types::*;

pub use misc::util::{ TargetMap, Targetted };

use std::sync::Arc;

use transformable_channels::mpsc::*;

pub type WatchEvent = GenericWatchEvent<Value>;

/// User identifier that will be passed from the REST API handlers to the
/// adapters.
#[derive(Debug, Clone, PartialEq)]
pub enum User {
    None,
    Id(i32)
}

#[test]
fn test_user_partialeq() {
    assert_eq!(User::None, User::None);
    assert_eq!(User::Id(1), User::Id(1));
}

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

/// A handle to the public API.
impl API {
    /// Get the metadata on services matching some conditions.
    ///
    /// A call to `API::get_services(vec![req1, req2, ...])` will return
    /// the metadata on all services matching _either_ `req1` or `req2`
    /// or ...
    pub fn get_services(&self, selectors: Vec<ServiceSelector>) -> Vec<ServiceDescription> {
        self.manager.get_services(selectors)
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
    pub fn add_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize {
        self.manager.add_service_tags(selectors, tags)
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
    pub fn remove_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize {
        self.manager.remove_service_tags(selectors, tags)
    }

    /// Get a list of getters matching some conditions
    pub fn get_features(&self, selectors: Vec<FeatureSelector>) -> Vec<FeatureDescription> {
        self.manager.get_features(selectors)
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
    pub fn add_feature_tags(&self, selectors: Vec<FeatureSelector>, tags: Vec<Id<TagId>>) -> usize {
        self.manager.add_feature_tags(selectors, tags)
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
    pub fn remove_feature_tags(&self, selectors: Vec<FeatureSelector>, tags: Vec<Id<TagId>>) -> usize {
        self.manager.remove_feature_tags(selectors, tags)
    }

    /// Place a method call to a Feature.
    pub fn place_method_call(&self, method: MethodCall, request: TargetMap<FeatureSelector, Option<Value>>, user: User) ->
        PerFeatureResult<Option<Value>>
    {
        self.manager.place_method_call(method, request, user, |_, value| Ok(value), |_, value| Ok(value))
    }

    /// Watch for changes from channels.
    ///
    /// This method registers a closure to watch over events on a set of channels. Argument `watch`
    /// specifies which channels to watch and which events are of interest.
    ///
    /// - If argument `Exactly<Range>` is `Exactly::Exactly(range)`, the watch is interested in
    /// values coming from these channels, if they fall within `range`. This is the most common
    /// case. In this case, `on_event` receives `WatcherEvent::GetterAdded`,
    /// `WatcherEvent::GetterRemoved` and `WatcherEvent::Value`, whenever a new value is available
    /// in the range. Values that do not have the same type as `range` are dropped silently.
    ///
    /// - If argument `Exactly<Range>` is `Exactly::Never`, the watch is not interested in the
    /// values coming from these channels, only in connection/disconnection events. Argument
    /// `on_event` receives `WatchEvent::GetterAdded` and `WatchEvent::GetterRemoved`.
    ///
    /// - If the `Exactly<Range>` argument is `Exactly::Always`, the watch is interested in
    /// receiving *every single value coming from the channels*. This is very rarely a good idea.
    /// Many devices may reject such requests.
    ///
    /// The watcher is disconnected once the `WatchGuard` returned by this method is dropped.
    ///
    /// # WebSocket API
    ///
    /// `/api/v1/channels/watch`
    pub fn register_watch(&self, mut watch: TargetMap<FeatureSelector, Exactly<Value>>,
        on_event: Box<ExtSender<WatchEvent>>) -> WatchGuard
    {
        use io::parse::{ DeserializeSupport, ParseError };
        struct EmptyDeserializeSupport;
        impl DeserializeSupport for EmptyDeserializeSupport {
            fn get_binary(&self, _: usize) -> Result<&[u8], ParseError> {
                panic!("This DeserializeSupport should be used only with instances of `Value`.");
            }
        }

        // Convert `Exactly<Value>` to `Exactly<Arc<AsValue>>`.
        let watch : TargetMap<_, Exactly<Arc<AsValue>>> = watch.drain(..)
            .map(|Targetted { select, payload }| {
                let payload = match payload {
                    Exactly::Always => Exactly::Always,
                    Exactly::Never => Exactly::Never,
                    Exactly::Exactly(value) => Exactly::Exactly(Arc::new(value) as Arc<AsValue>)
                };
                Targetted {
                    select: select,
                    payload: payload
                }
            }).collect();

        let on_event = Box::new(on_event.map(|event: ManagerWatchEvent| {
            event.convert(|WatchEventInternals { value, .. }| {
                Ok(value)
            })
        }));

        self.manager.register_watch(watch, on_event, Arc::new(EmptyDeserializeSupport))
    }
}
