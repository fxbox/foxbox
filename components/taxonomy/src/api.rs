//!
//! The API for communicating with devices.
//!
//! This API is provided as Traits to be implemented:
//!
//! - by the low-level layers of the FoxBox, including the adapters;
//! - by test suites and tools that need to simulate connected devices.
//!
//! In turn, this API is used to implement:
//!
//! - the public-facing REST and WebSocket API;
//! - the rules API (ThinkerBell).
//!
//!

use services::*;
use selector::*;
use values::Value;
use util::{ Id, ResultSet };

use std::boxed::FnBox;

/// An error produced by one of the APIs in this module.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Error {
    /// There is no such service connected to the Foxbox, even indirectly.
    NoSuchService(Id<ServiceId>),

    /// There is no such getter channel connected to the Foxbox, even indirectly.
    NoSuchGetter(Id<Getter>),

    /// There is no such setter channel connected to the Foxbox, even indirectly.
    NoSuchSetter(Id<Setter>),

    /// Attempting to set a value with the wrong type
    TypeError,
}

/// An event during watching.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WatchEvent {
    /// A new value is available.
    Value {
        /// The channel that sent the value.
        from: Id<Getter>,

        /// The actual value.
        value: Value
    },

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was
    /// removed. Payload is the id of the device that was removed.
    GetterRemoved(Id<Getter>),

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was
    /// added. Payload is the id of the device that was added.
    GetterAdded(Id<Getter>),
}

/// A handle to the public API.
pub trait API: Send {
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
    /// ## Requests
    ///
    /// Any JSON that can be deserialized to a `Vec<ServiceSelector>`. See
    /// the implementation of `ServiceSelector` for details.
    ///
    /// ### Example
    ///
    /// Selector all doors in the entrance (tags `door`, `entrance`)
    /// that support setter channel `OpenClose`
    ///
    /// ```json
    /// [{
    ///   "tags": ["entrance", "door"],
    ///   "getters": [
    ///     {
    ///       "kind": {
    ///         "Exactly": {
    ///           "OpenClose": []
    ///         }
    ///       }
    ///     }
    ///   ]
    /// }]
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
    ///
    /// ### Example
    ///
    /// ```json
    /// [{
    ///   "tags": ["entrance", "door", "somevendor"],
    ///   "id: "some-service-id",
    ///   "getters": [],
    ///   "setters": [
    ///     "tags": [...],
    ///     "id": "some-channel-id",
    ///     "service": "some-service-id",
    ///     "last_seen": "some-date",
    ///     "mechanism": {
    ///       "Setter":  {
    ///         "kind": {
    ///           "OnOff": []
    ///         },
    ///         "push": [5000],
    ///         "updated": "some-date",
    ///       }
    ///     }
    ///   ]
    /// }]
    /// ```
    fn get_services(&self, &[ServiceSelector], Box<FnBox(Vec<Service>)>);

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
    /// ## Getters
    ///
    /// Any JSON that can be deserialized to
    ///
    /// ```ignore
    /// {
    ///   set: Vec<ServiceSelector>,
    ///   tags: Vec<String>,
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
    /// A JSON string representing a number.
    fn add_service_tag(&self, set: &[ServiceSelector], tags: &[String], Box<FnBox(usize)>);

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
    /// ## Getters
    ///
    /// Any JSON that can be deserialized to
    ///
    /// ```ignore
    /// {
    ///   set: Vec<ServiceSelector>,
    ///   tags: Vec<String>,
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
    fn remove_service_tag(&self, set: &[ServiceSelector], tags: &[String], Box<FnBox(usize)>);

    /// Get a list of getters matching some conditions
    ///
    /// # REST API
    ///
    /// `GET /api/v1/channels`
    fn get_getter_channels(&self, &[GetterSelector], Box<FnBox(Vec<Channel<Getter>>)>);
    fn get_setter_channels(&self, &[SetterSelector], Box<FnBox(Vec<Channel<Setter>>)>);

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
    ///   tags: Vec<String>,
    /// }
    /// ```
    /// or
    /// ```ignore
    /// {
    ///   set: Vec<SetterSelector>,
    ///   tags: Vec<String>,
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
    fn add_getter_tag(&self, &[GetterSelector], &[String], Box<FnBox(usize)>);
    fn add_setter_tag(&self, &[SetterSelector], &[String], Box<FnBox(usize)>);

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
    /// # REST API
    ///
    /// `DELETE /api/v1/channels/tag`
    ///
    /// ## Requests
    ///
    /// Any JSON that can be deserialized to
    ///
    /// ```ignore
    /// {
    ///   set: Vec<GetterSelector>,
    ///   tags: Vec<String>,
    /// }
    /// ```
    /// or
    /// ```ignore
    /// {
    ///   set: Vec<SetterSelector>,
    ///   tags: Vec<String>,
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
    fn remove_getter_tag(&self, &[GetterSelector], &[String], Box<FnBox(usize)>);
    fn remove_setter_tag(&self, &[SetterSelector], &[String], Box<FnBox(usize)>);

    /// Read the latest value from a set of channels
    ///
    /// # REST API
    ///
    /// `GET /api/v1/channels/value`
    fn get_channel_value(&self, &[GetterSelector], Box<FnBox(ResultSet<Id<Getter>, Value, Error>)>);

    /// Send one value to a set of channels
    ///
    /// # REST API
    ///
    /// `POST /api/v1/channels/value`
    fn set_channel_value(&self, &[Vec<SetterSelector>], Vec<Value>, Box<FnBox(ResultSet<Id<Setter>, (), Error>)>);

    /// Watch for any change
    ///
    /// # WebSocket API
    ///
    /// `/api/v1/channels/watch`
    fn register_channel_watch(&self, Vec<WatchOptions>, cb: Box<Fn(WatchEvent) + Send + 'static>) -> Self::WatchGuard;

    /// A value that causes a disconnection once it is dropped.
    type WatchGuard;
}

/// Options for watching changes in one or more channels.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WatchOptions {
    /// The set of getters to watch. Note that the actual getters in the
    /// set may change over time.
    pub source: GetterSelector,

    /// If `true`, watch as new values become available.
    pub should_watch_values: bool,

    /// If `true`, watch as services are connected/disconnected.
    pub should_watch_topology: bool,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}

impl WatchOptions {
    pub fn new() -> Self {
        WatchOptions {
            source: GetterSelector::new(),
            should_watch_values: false,
            should_watch_topology: false,
            private: (),
        }
    }

    /// Restrict to getter channels in a given set.
    ///
    /// Also note that the actual getter channels that are part of the
    /// set may change with time, for instance if devices are added
    /// ore removed.  The selector _is live_, i.e. the channel watch
    /// will continue watching any getter channels that match `req`.
    pub fn with_getters(self, req: GetterSelector) -> Self {
        WatchOptions {
            source: self.source.and(req),
            ..self
        }
    }

    pub fn with_watch_values(self, should: bool) -> Self {
        WatchOptions {
            should_watch_values: should,
            ..self
        }
    }

    pub fn with_watch_topology(self, should: bool) -> Self {
        WatchOptions {
            should_watch_topology: should,
            ..self
        }
    }
}
