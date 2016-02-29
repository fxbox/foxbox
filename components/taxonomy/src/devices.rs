//! This module defines the metadata on devices and services.
//!
//! Note that all the data structures in this module represent
//! snapshots of subsets of the devices available. None of these data
//! structures are live, so there is always the possibility that
//! devices may have been added or removed from the FoxBox by the time
//! these data structures are read.

use values::*;
use util::Id;

use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Error};


/// A marker for Id.
/// Only useful for writing `Id<NodeId>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct NodeId;

/// Metadata on a node. A node is a device or collection of devices
/// that may offer services. The FoxBox itself a node offering
/// services such as a clock, communication with the user through her
/// smart devices, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Tags describing the node.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used by applications to find nodes and
    /// services.
    ///
    /// For instance, a user may set tag "entrance" to all nodes
    /// placed in the entrance of his house, or a tag "blue" to a node
    /// controlling blue lights. An adapter may set tags "plugged" or
    /// "battery" to devices that respectively depend on a plugged
    /// power source or on a battery.
    pub tags: Vec<String>,

    /// An id unique to this node.
    pub id: Id<NodeId>,

    /// Channels connected directly to this node.
    pub inputs: Vec<Channel<Get>>,
    pub outputs: Vec<Channel<Set>>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}


/// The kind of the service, i.e. a strongly-typed description of
/// _what_ the service can do. Used both for locating services
/// (e.g. "I need a clock" or "I need something that can provide
/// pictures") and for determining the data structure that these
/// services can provide or consume.
///
/// A number of service kinds are standardized, and provided as a set
/// of strongly-typed enum constructors. It is clear, however, that
/// many devices will offer services that cannot be described by
/// pre-existing constructors. For this purpose, this enumeration
/// offers a constructor `Extension`, designed to describe novel
/// services.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelKind {
    ///
    /// # No payload
    ///

    /// The service is ready. Used for instance once a countdown has
    /// reached completion.
    Ready,

    ///
    /// # Boolean
    ///

    /// The service is used to detect or decide whether some device
    /// is on or off.
    OnOff,

    /// The service is used to detect or decide whether some device
    /// is open or closed.
    OpenClosed,

    ///
    /// # Time
    ///

    /// The service is used to read or set the current absolute time.
    /// Used for instance to wait until a specific time and day before
    /// triggering an action, or to set the appropriate time on a new
    /// device.
    CurrentTime,

    /// The service is used to read or set the current time of day.
    /// Used for instance to trigger an action at a specific hour
    /// every day.
    CurrentTimeOfDay,

    /// The service is part of a countdown. This is the time
    /// remaining until the countdown is elapsed.
    RemainingTime,

    ///
    /// # Temperature
    ///

    Thermostat,
    ActualTemperature,

    /// TODO: Add more

    /// An operation of a kind that has not been standardized yet.
    Extension {
        /// The vendor. Used for namespacing purposes, to avoid
        /// confusing two incompatible extensions with similar
        /// names. For instance, "foxlink@mozilla.com".
        vendor: String,

        /// Identification of the adapter introducing this operation.
        /// Designed to aid with tracing and debugging.
        adapter: String,

        /// A string describing the nature of the value, designed to
        /// let applications discover the devices.
        ///
        /// Examples: `"GroundHumidity"`.
        kind: String,

        /// The data type of the value.
        typ: Type
    }
}

impl ChannelKind {
    /// Get the type of values used to communicate with this service.
    pub fn get_type(&self) -> Type {
        use self::ChannelKind::*;
        use values::Type::*;
        match *self {
            Ready => Unit,
            OnOff | OpenClosed => Bool,
            CurrentTime => TimeStamp,
            CurrentTimeOfDay | RemainingTime => Duration,
            Thermostat | ActualTemperature => Temperature,
            Extension { ref typ, ..} => typ.clone(),
        }
    }
}


/// An input operation available on an service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Get {
    /// The kind of value that can be obtained from this service.
    pub kind: ChannelKind,

    /// If `Some(duration)`, this service can be polled, i.e. it
    /// will respond when the FoxBox requests the latest value.
    /// Parameter `duration` indicates the smallest interval
    /// between two updates.
    ///
    /// Otherwise, the service cannot be polled and will push
    /// data to the FoxBox when it is available.
    ///
    /// # Examples
    ///
    /// - Long-running pollution or humidity sensors typically
    ///   do not accept requests and rather send batches of
    ///   data every 24h.
    #[serde(default)]
    pub poll: Option<ValDuration>,

    /// If `Some(duration)`, this service can send the data to
    /// the FoxBox whenever it is updated. Parameter `duration`
    /// indicates the smallest interval between two updates.
    ///
    /// Otherwise, the service cannot send data to the FoxBox
    /// and needs to be polled.
    #[serde(default)]
    pub trigger: Option<ValDuration>,

    /// Date at which the latest value was received, whether through
    /// polling or through a trigger.
    pub updated: Option<TimeStamp>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}
impl IOMechanism for Get {
}

/// An output operation available on an service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Set {
    /// The kind of value that can be sent to this service.
    pub kind: ChannelKind,

    /// If `Some(duration)`, this service supports pushing,
    /// i.e. the FoxBox can send values.
    #[serde(default)]
    pub push: Option<ValDuration>,

    /// Date at which the latest value was sent to the service.
    #[serde(default)]
    pub updated: Option<TimeStamp>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}
impl IOMechanism for Set {
}

/// An service represents a single place where data can enter or
/// leave a device. Note that services support either a single kind
/// of input or a single kind of output. Devices that support both
/// inputs or outputs, or several kinds of inputs, or several kinds of
/// outputs, are represented as nodes containing several services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel<IO> where IO: IOMechanism {
    /// Tags describing the service.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used to regroup services for rules.
    ///
    /// For instance "entrance".
    #[serde(default)]
    pub tags: Vec<String>,

    /// An id unique to this service.
    pub id: Id<IO>,

    /// The node owning this service.
    pub node: Id<NodeId>,

    /// The update mechanism for this service.
    pub mechanism: IO,

    /// The last time the device was seen.
    #[serde(default)]
    pub last_seen: Option<TimeStamp>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}

/// A mechanism used for communicating between the application and the
/// service.
pub trait IOMechanism: Deserialize + Serialize {
}

