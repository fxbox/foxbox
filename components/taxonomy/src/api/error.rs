//! Error-handling.

use api::services::*;

use io::parse::*;
use io::serialize::*;
use io::types::*;

use std::{ error, fmt };
use std::error::Error as std_error;

/// An error that arose during interaction with either a device, an adapter or the
/// adapter manager
#[derive(Deserialize, Debug, Clone)]
pub enum Error {
    /// Attempting to send a value with a wrong type.
    TypeError(String),

    /// Attempting to send an invalid value. For instance, a time of day larger than 24h.
    InvalidValue(String),

    /// An error internal to the foxbox or an adapter. Normally, these errors should never
    /// arise from the high-level API.
    InternalError(InternalError),

    ParseError(ParseError),
    SerializeError(SerializeError)
}

impl ToJSON for Error {
    fn to_json(&self, support: &SerializeSupport) -> JSON {
        vec![("API Error", format!("{:?}", self))].to_json(support)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidValue(ref value) => write!(f, "{}: {:?}",self.description(), value),
            Error::InternalError(ref err) => write!(f, "{}: {:?}", self.description(), err), // TODO implement Display for InternalError as well
            _ => unimplemented!()
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::InvalidValue(_) => "Attempting to send an invalid value",
            Error::InternalError(_) => "Internal Error", // TODO implement Error for InternalError as well
            _ => unimplemented!()
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum InternalError {
    /// Attempting to call `send`, `fetch`, `delete`, `watch` on a device that does not support this.
    NoSuchMethod(Id<FeatureId>, String),

    /// Attempting to access a feature that isn't registered.
    NoSuchFeature(Id<FeatureId>),
    /// Attempting to access a service that isn't registered.
    NoSuchService(Id<ServiceId>),
    /// Attempting to access an adapter that isn't registered.
    NoSuchAdapter(Id<AdapterId>),

    DuplicateFeature(Id<FeatureId>),
    /// Attempting to register a service with an id that is already used.
    DuplicateService(Id<ServiceId>),
    /// Attempting to register an adapter with an id that is already used.
    DuplicateAdapter(Id<AdapterId>),

    /// Attempting to register a channel with an adapter that doesn't match that of its service.
    ConflictingAdapter(Id<AdapterId>, Id<AdapterId>),

    /// Open question: Individual adapters will have errors of many adapter-specific types.
    /// How do we make this best represent those?
    GenericError(String),

    /// Attempting to register a service in an invalid initial state. Typically, a service that
    /// pretends that it already has channels.
    InvalidInitialService,

    SerializationError(SerializeError),
    DeserializationError(ParseError),
}
