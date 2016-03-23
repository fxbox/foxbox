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
use values::{ Value, Range, TypeError };
use util::{ Exactly, Id };

use transformable_channels::mpsc::*;

use std::collections::HashMap;

use std::{ error, fmt };
use std::error::Error as std_error;

/// An error that arose during interaction with either a device, an adapter or the
/// adapter manager
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Error {
    /// Attempting to fetch a value from a Channel<Getter> that doesn't support this operation.
    GetterDoesNotSupportPolling(Id<Getter>),

    /// Attempting to watch a value from a Channel<Getter> that doesn't support this operation.
    GetterDoesNotSupportWatching(Id<Getter>),

    /// Attempting to watch all values from a Channel<Getter> that requires a filter.
    /// For instance, some Channel<Getter> may be updated 60 times per second. Attempting to
    /// watch all values could easily exceed the capacity of the network or exhaust the battery.
    /// In such a case, the adapter should return this error.
    GetterRequiresThresholdForWatching(Id<Getter>),

    /// Attempting to send a value with a wrong type.
    TypeError(TypeError),

    /// Attempting to use an inconsistent range. For instance, one with `min > max`.
    RangeError(Range),

    /// Attempting to send an invalid value. For instance, a time of day larger than 24h.
    InvalidValue(Value),

    /// An error internal to the foxbox or an adapter. Normally, these errors should never
    /// arise from the high-level API.
    InternalError(InternalError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::GetterDoesNotSupportPolling(ref getter) |
            Error::GetterDoesNotSupportWatching(ref getter) |
            Error::GetterRequiresThresholdForWatching(ref getter) => write!(f, "{}: {}", self.description(), getter),
            Error::TypeError(ref err) => write!(f, "{}: {}", self.description(), err),
            Error::RangeError(ref range) => write!(f, "{}: {:?}", self.description(), range),
            Error::InvalidValue(ref value) => write!(f, "{}: {:?}",self.description(), value),
            Error::InternalError(ref err) => write!(f, "{}: {:?}", self.description(), err), // TODO implement Display for InternalError as well
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::GetterDoesNotSupportPolling(_) => "Attempting to fetch a value from a Channel<Getter> that doesn't support this operation",
            Error::GetterDoesNotSupportWatching(_) => "Attempting to watch a value from a Channel<Getter> that doesn't support this operation",
            Error::GetterRequiresThresholdForWatching(_) => "Attempting to watch all value from a Channel<Getter> that requires a filter",
            Error::TypeError(_) => "Attempting to send a value with a wrong type",
            Error::RangeError(_) => "Attempting to use an inconsistent range",
            Error::InvalidValue(_) => "Attempting to send an invalid value",
            Error::InternalError(_) => "Internal Error" // TODO implement Error for InternalError as well
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::TypeError(ref err) => Some(err),
            _ => None
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum InternalError {
    /// Attempting to fetch or watch a getter that isn't registered.
    NoSuchGetter(Id<Getter>),
    /// Attempting to send values to a setter that isn't registered.
    NoSuchSetter(Id<Setter>),
    /// Attempting to access a service that isn't registered.
    NoSuchService(Id<ServiceId>),
    /// Attempting to access an adapter that isn't registered.
    NoSuchAdapter(Id<AdapterId>),

    /// Attempting to register a getter with an id that is already used.
    DuplicateGetter(Id<Getter>),
    /// Attempting to register a setter with an id that is already used.
    DuplicateSetter(Id<Setter>),
    /// Attempting to register a service with an id that is already used.
    DuplicateService(Id<ServiceId>),
    /// Attempting to register an adapter with an id that is already used.
    DuplicateAdapter(Id<AdapterId>),

    /// Attempting to register a channel with an adapter that doesn't match that of its service.
    ConflictingAdapter(Id<AdapterId>, Id<AdapterId>),

    /// Attempting to register a service in an invalid initial state. Typically, a service that
    /// pretends that it already has channels.
    InvalidInitialService,
}

/// An event during watching.
#[derive(Serialize, Debug, Clone)]
pub enum WatchEvent {
    /// If a range was specified when we registered for watching, `EnterRange` is fired whenever
    /// we enter this range. If `Always` was specified, `EnterRange` is fired whenever a new value
    /// is available. Otherwise, never fired.
    EnterRange {
        /// The channel that sent the value.
        from: Id<Getter>,

        /// The actual value.
        value: Value
    },

    /// If a range was specified when we registered for watching, `ExitRange` is fired whenever
    /// we exit this range. Otherwise, never fired.
    ExitRange {
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

    /// One of the channels encountered an error during initialization.
    /// This channel will not be watched, but other channels will remain
    /// watched.
    InitializationError {
        channel: Id<Getter>,
        error: Error
    },
}

/// A bunch of results coming from different sources.
pub type ResultMap<K, T, E> = HashMap<K, Result<T, E>>;

/// A bunch of instructions, going to different targets.
pub type TargetMap<K, T> = Vec<(Vec<K>, T)>;

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
    fn get_services(&self, Vec<ServiceSelector>) -> Vec<Service>;

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
    /// A JSON string representing a number.
    fn add_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize;

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
    fn remove_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize;

    /// Get a list of getters matching some conditions
    ///
    /// # REST API
    ///
    /// `GET /api/v1/channels`
    fn get_getter_channels(&self, selectors: Vec<GetterSelector>) -> Vec<Channel<Getter>>;
    fn get_setter_channels(&self, selectors: Vec<SetterSelector>) -> Vec<Channel<Setter>>;

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
    fn add_getter_tags(&self, selectors: Vec<GetterSelector>, tags: Vec<Id<TagId>>) -> usize;
    fn add_setter_tags(&self, selectors: Vec<SetterSelector>, tags: Vec<Id<TagId>>) -> usize;

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
    fn remove_getter_tags(&self, selectors: Vec<GetterSelector>, tags: Vec<Id<TagId>>) -> usize;
    fn remove_setter_tags(&self, selectors: Vec<SetterSelector>, tags: Vec<Id<TagId>>) -> usize;

    /// Read the latest value from a set of channels
    ///
    /// # REST API
    ///
    /// `GET /api/v1/channels/value`
    fn fetch_values(&self, Vec<GetterSelector>) -> ResultMap<Id<Getter>, Option<Value>, Error>;

    /// Send a bunch of values to a set of channels
    ///
    /// # REST API
    ///
    /// `POST /api/v1/channels/value`
    fn send_values(&self, TargetMap<SetterSelector, Value>) -> ResultMap<Id<Setter>, (), Error>;

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
    fn watch_values(&self, watch: TargetMap<GetterSelector, Exactly<Range>>,
            on_event: Box<ExtSender<WatchEvent>>) -> Self::WatchGuard;

    /// A value that causes a disconnection once it is dropped.
    type WatchGuard;
}
