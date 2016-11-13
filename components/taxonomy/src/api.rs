//! The API for communicating with devices.
//!
//! This API is provided as Traits to be implemented:
//!
//! - by the low-level layers of the `FoxBox`, including the adapters;
//! - by test suites and tools that need to simulate connected devices.
//!
//! In turn, this API is used to implement:
//!
//! - the public-facing REST and `WebSocket` API;
//! - the rules API (`ThinkerBell`).
//!
//!

use channel::Channel;
use io::*;
use services::*;
use selector::*;
pub use util::{ResultMap, TargetMap, Targetted};
use values::TypeError;

use transformable_channels::mpsc::*;

use std::{error, fmt};
use std::error::Error as std_error;
use std::sync::Arc;

use serde_json;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Operation {
    Fetch,
    Send,
    Watch,
}
impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Operation::*;
        match *self {
            Fetch => f.write_str("Fetch"),
            Send => f.write_str("Send"),
            Watch => f.write_str("Watch"),
        }
    }
}
impl ToJSON for Operation {
    fn to_json(&self) -> JSON {
        use self::Operation::*;
        match *self {
                Fetch => "Fetch",
                Send => "Send",
                Watch => "Watch",
            }
            .to_json()
    }
}

/// An error that arose during interaction with either a device, an adapter or the
/// adapter manager
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Attempting to execute a value from a Channel that doesn't support this operation.
    OperationNotSupported(Operation, Id<Channel>),

    /// Attempting to watch all values from a Channel that requires a filter.
    /// For instance, some Channel may be updated 60 times per second. Attempting to
    /// watch all values could easily exceed the capacity of the network or exhaust the battery.
    /// In such a case, the adapter should return this error.
    GetterRequiresThresholdForWatching(Id<Channel>),

    /// Attempting to send a value with a wrong type.
    WrongType(TypeError),

    /// Attempting to send an invalid value. For instance, a time of day larger than 24h.
    InvalidValue,

    /// An error internal to the foxbox or an adapter. Normally, these errors should never
    /// arise from the high-level API.
    Internal(InternalError),

    // An error happened while attempting to parse a value.
    Parsing(ParseError),

    // An error happened while attempting to serialize a value.
    Serializing(SerializeError),
}

impl ToJSON for Error {
    fn to_json(&self) -> JSON {
        use self::Error::*;
        match *self {
            OperationNotSupported(ref op, ref id) => {
                vec![("OperationNotSupported",
                      vec![("operation", op.to_json()), ("channel", id.to_json())])]
                    .to_json()
            }
            GetterRequiresThresholdForWatching(ref id) => {
                vec![("GetterRequiresThresholdForWatching", id.to_json())].to_json()
            }
            InvalidValue => "InvalidValue".to_json(),
            Internal(_) => "Internal Error".to_json(), // FIXME: Implement ToJSON for InternalError as well
            Parsing(ref err) => vec![("ParseError", serde_json::to_value(err))].to_json(),
            Serializing(ref err) => vec![("SerializeError", serde_json::to_value(err))].to_json(),
            WrongType(ref err) => vec![("TypeError", serde_json::to_value(err))].to_json(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::OperationNotSupported(ref operation, ref channel) => {
                write!(f, "{}: {} {}", self.description(), operation, channel)
            }
            Error::GetterRequiresThresholdForWatching(ref getter) => {
                write!(f, "{}: {}", self.description(), getter)
            }
            Error::WrongType(ref err) => write!(f, "{}: {}", self.description(), err),
            Error::InvalidValue => write!(f, "{}", self.description()),
            Error::Internal(ref err) => write!(f, "{}: {:?}", self.description(), err), // TODO implement Display for InternalError as well
            Error::Parsing(ref err) => write!(f, "{}: {:?}", self.description(), err), // TODO implement Display for ParseError as well
            Error::Serializing(ref err) => write!(f, "{}: {:?}", self.description(), err), // TODO implement Display for ParseError as well
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::OperationNotSupported(_, _) => {
                "Attempting to perform a call to a Channel that does not support such calls"
            }
            Error::GetterRequiresThresholdForWatching(_) => {
                "Attempting to watch all value from a Channel that requires a filter"
            }
            Error::WrongType(_) => "Attempting to send a value with a wrong type",
            Error::InvalidValue => "Attempting to send an invalid value",
            Error::Internal(_) => "Internal Error", // TODO implement Error for InternalError as well
            Error::Parsing(ref err) => err.description(),
            Error::Serializing(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::WrongType(ref err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum InternalError {
    /// Attempting to use a channel that isn't registered.
    NoSuchChannel(Id<Channel>),
    /// Attempting to access a service that isn't registered.
    NoSuchService(Id<ServiceId>),
    /// Attempting to access an adapter that isn't registered.
    NoSuchAdapter(Id<AdapterId>),

    /// Attempting to register a channel with an id that is already used.
    DuplicateChannel(Id<Channel>),
    /// Attempting to register a service with an id that is already used.
    DuplicateService(Id<ServiceId>),
    /// Attempting to register an adapter with an id that is already used.
    DuplicateAdapter(Id<AdapterId>),

    WrongChannel(Id<Channel>),

    /// Attempting to register a channel with an adapter that doesn't match that of its service.
    ConflictingAdapter(Id<AdapterId>, Id<AdapterId>),

    /// Open question: Individual adapters will have errors of many adapter-specific types.
    /// How do we make this best represent those?
    GenericError(String),

    /// Attempting to register a service in an invalid initial state. Typically, a service that
    /// pretends that it already has channels.
    InvalidInitialService,
}

/// An event during watching.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// If a range was specified when we registered for watching, `EnterRange` is fired whenever
    /// we enter this range. If `Always` was specified, `EnterRange` is fired whenever a new value
    /// is available. Otherwise, never fired.
    EnterRange {
        /// The channel that sent the value.
        channel: Id<Channel>,

        /// The actual value.
        value: Payload,

        format: Arc<Format>,
    },

    /// If a range was specified when we registered for watching, `ExitRange` is fired whenever
    /// we exit this range. Otherwise, never fired.
    ExitRange {
        /// The channel that sent the value.
        channel: Id<Channel>,

        /// The actual value.
        value: Payload,

        format: Arc<Format>,
    },

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was
    /// removed. Payload is the id of the device that was removed.
    ChannelRemoved(Id<Channel>),

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was
    /// added. Payload is the id of the device that was added.
    ChannelAdded(Id<Channel>),

    Error { channel: Id<Channel>, error: Error },
}

/// User identifier that will be passed from the REST API handlers to the
/// adapters.
#[derive(Debug, Clone, PartialEq)]
pub enum User {
    None,
    Id(String),
}

#[test]
fn test_user_partialeq() {
    assert_eq!(User::None, User::None);
    assert_eq!(User::Id(String::from("1")), User::Id(String::from("1")));
}

impl<P, T> Parser<Targetted<T, Payload>> for Targetted<P, Payload>
    where P: Parser<T>,
          T: Clone
{
    fn description() -> String {
        format!("Targetted<{}, Value>", P::description())
    }
    fn parse(path: Path, source: &JSON) -> Result<Targetted<T, Payload>, ParseError> {
        if source.is_object() {
            // Default format: an object {select, value}.
            let select = try!(path.push("select", |path| Vec::<P>::take(path, source, "select")));
            let payload = try!(path.push("value", |path| Payload::take(path, source, "value")));
            Ok(Targetted {
                select: select,
                payload: payload,
            })
        } else if let JSON::Array(ref array) = *source {
            // Fallback format: an array of two values.
            if array.len() != 2 {
                return Err(ParseError::type_error(&Self::description() as &str,
                                                  &path,
                                                  "an array of length 2"));
            }
            let select = try!(path.push_index(0, |path| Vec::<P>::parse(path, &array[0])));
            let payload = try!(path.push_index(1, |path| Payload::parse(path, &array[1])));
            Ok(Targetted {
                select: select,
                payload: payload,
            })
        } else {
            Err(ParseError::type_error(&Self::description() as &str,
                                       &path,
                                       "an object {select, value}"))
        }
    }
}

impl<P, T> Parser<Targetted<T, Exactly<Payload>>> for Targetted<P, Exactly<Payload>>
    where P: Parser<T>,
          T: Clone
{
    fn description() -> String {
        format!("Targetted<{}, range>", P::description())
    }
    fn parse(path: Path, source: &JSON) -> Result<Targetted<T, Exactly<Payload>>, ParseError> {
        let select = try!(path.push("select", |path| Vec::<P>::take(path, source, "select")));
        if let Some(&JSON::String(ref str)) = source.find("range") {
            if str == "Never" {
                return Ok(Targetted {
                    select: select,
                    payload: Exactly::Never,
                });
            }
        }
        let result = match path.push("range",
                                     |path| Exactly::<Payload>::take_opt(path, source, "range")) {
            Some(Ok(Exactly::Exactly(payload))) => Exactly::Exactly(payload),
            Some(Ok(Exactly::Always)) |
            None => Exactly::Always,
            Some(Ok(Exactly::Never)) => Exactly::Never,
            Some(Err(err)) => return Err(err),
        };
        Ok(Targetted {
            select: select,
            payload: result,
        })
    }
}

/// A handle to the public API.
pub trait API: Send {
    /// Get the metadata on services matching some conditions.
    ///
    /// A call to `API::get_services(vec![req1, req2, ...])` will return
    /// the metadata on all services matching _either_ `req1` or `req2`
    /// or ...
    ///
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
    fn remove_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize;


    /// Get a list of channels matching some conditions
    fn get_channels(&self, selectors: Vec<ChannelSelector>) -> Vec<Channel>;

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
    fn add_channel_tags(&self, selectors: Vec<ChannelSelector>, tags: Vec<Id<TagId>>) -> usize;

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
    fn remove_channel_tags(&self, selectors: Vec<ChannelSelector>, tags: Vec<Id<TagId>>) -> usize;

    /// Read the latest value from a set of channels
    fn fetch_values(&self, Vec<ChannelSelector>, user: User) -> OpResult<(Payload, Arc<Format>)>;

    /// Send a bunch of values to a set of channels.
    ///
    /// Sending values to several setters of the same service in a single call will generally
    /// be much faster than calling this method several times.
    fn send_values(&self,
                   TargetMap<ChannelSelector, Payload>,
                   user: User)
                   -> ResultMap<Id<Channel>, (), Error>;

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
    fn watch_values(&self,
                    watch: TargetMap<ChannelSelector, Exactly<Payload>>,
                    on_event: Box<ExtSender<WatchEvent>>)
                    -> Self::WatchGuard;

    /// A value that causes a disconnection once it is dropped.
    type WatchGuard;
}

pub type OpResult<T> = ResultMap<Id<Channel>, Option<T>, Error>;
