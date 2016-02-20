use std::time::Duration;
use values;

extern crate chrono;

///
/// Hubs
///

pub type HubId = String;

/// A hub represents a node to which several endpoints, as well as
/// other hubs can be connected. The FoxBox is the root hub. Simple
/// devices that can do a single thing (e.g. a button) are endpoints,
/// while complex devices containing several sensors or effectors are
/// also hubs, in which each sensor and each effector is an endpoint.
#[derive(Debug, Clone)]
pub struct Hub {
    /// Tags describing the hub.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used by applications.
    ///
    /// For instance "entrance".
    tags: Vec<String>,

    /// An id unique to this hub.
    id: HubId,

    /// If this hub has a parent, the id of the parent.
    parent: Option<HubId>,

    /// Hubs depending on this hub.
    subhubs: Vec<Hub>,

    /// Endpoints connected directly to this hub.
    inputs: Vec<EndPoint<Input>>,
    outputs: Vec<EndPoint<Output>>,
}

impl Hub {
    /// Tags describing the hub.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used by applications.
    ///
    /// For instance "entrance".
    pub fn get_tags<'a>(&'a self) -> &'a Vec<String> {
        &self.tags
    }

    /// An id unique to this hub.
    pub fn get_id<'a>(&'a self) -> &'a HubId {
        &self.id
    }

    /// If this hub has a parent, the id of the parent.
    pub fn get_parent<'a>(&'a self) -> &'a Option<HubId> {
        &self.parent
    }

    /// Hubs depending on this hub.
    pub fn get_subhubs<'a>(&'a self) -> &'a Vec<Hub> {
        &self.subhubs
    }

    /// Input endpoints connected directly to this hub.
    pub fn get_inputs<'a>(&'a self) -> &'a Vec<EndPoint<Input>> {
        &self.inputs
    }

    /// Output endpoints connected directly to this hub.
    pub fn get_outputs<'a>(&'a self) -> &'a Vec<EndPoint<Output>> {
        &self.outputs
    }
}

///
/// Endpoints
///

pub type EndPointId = String;

/// The kind of value provided by an endpoint.
#[derive(Debug, Clone)]
pub enum ValueKind {
    ///
    /// # No payload
    ///

    /// The endpoint is ready. Used for instance once a countdown has
    /// reached completion.
    Ready,

    ///
    /// # Boolean
    ///

    /// The endpoint is used to detect or decide whether some device
    /// is on or off.
    OnOff,

    /// The endpoint is used to detect or decide whether some device
    /// is open or closed.
    OpenClosed,

    ///
    /// # Time
    ///

    /// The endpoint is used to read or set the current absolute time.
    /// Used for instance to wait until a specific time and day before
    /// triggering an action, or to set the appropriate time on a new
    /// device.
    CurrentTime,

    /// The endpoint is used to read or set the current time of day.
    /// Used for instance to trigger an action at a specific hour
    /// every day.
    CurrentTimeOfDay,

    /// The endpoint is part of a countdown. This is the time
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
        /// The vendor. An empty string for standardized value kinds,
        /// otherwise a string identifying the owner of this non-standard
        /// value (e.g. "Mozilla")
        vendor: String,

        /// Identification of the adapter introducing this operation.
        adapter: String,

        /// The nature of the value.
        ///
        /// For instance: "is-on", "is-open".
        nature: String,

        /// The data type of the value.
        typ: values::Type
    }
}

impl ValueKind {
    pub fn get_type(&self) -> values::Type {
        use self::ValueKind::*;
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


/// An input operation available on an endpoint.
#[derive(Debug, Clone)]
pub struct Input {
    /// The kind of value that can be obtained from this endpoint.
    kind: ValueKind,

    /// If `Some(duration)`, this endpoint can be polled, i.e. it
    /// will respond when the FoxBox requests the latest value.
    /// Parameter `duration` indicates the smallest interval
    /// between two updates.
    ///
    /// Otherwise, the endpoint cannot be polled and will push
    /// data to the FoxBox when it is available.
    ///
    /// # Examples
    ///
    /// - Long-running pollution or humidity sensors typically
    ///   do not accept requests and rather send batches of
    ///   data every 24h.
    poll: Option<Duration>,

    /// If `Some(duration)`, this endpoint can send the data to
    /// the FoxBox whenever it is updated. Parameter `duration`
    /// indicates the smallest interval between two updates.
    ///
    /// Otherwise, the endpoint cannot send data to the FoxBox
    /// and needs to be polled.
    trigger: Option<Duration>,

    /// Date at which the latest value was received, whether through
    /// polling or through a trigger.
    updated: chrono::DateTime<chrono::UTC>,
}

impl Input {
    /// The kind of value that can be obtained from this endpoint.
    pub fn get_kind(&self) -> ValueKind {
        self.kind.clone()
    }

    /// If `Some(duration)`, this endpoint can be polled, i.e. it
    /// will respond when the FoxBox requests the latest value.
    /// Parameter `duration` indicates the smallest interval
    /// between two updates.
    ///
    /// Otherwise, the endpoint cannot be polled and will push
    /// data to the FoxBox when it is available.
    ///
    /// # Examples
    ///
    /// - Long-running pollution or humidity sensors typically
    ///   do not accept requests and rather send batches of
    ///   data every 24h.
    pub fn get_poll(&self) -> Option<Duration> {
        self.poll.clone()
    }

    /// If `Some(duration)`, this endpoint can send the data to
    /// the FoxBox whenever it is updated. Parameter `duration`
    /// indicates the smallest interval between two updates.
    ///
    /// Otherwise, the endpoint cannot send data to the FoxBox
    /// and needs to be polled.
    pub fn get_trigger(&self) -> Option<Duration> {
        self.trigger.clone()
    }

    /// Date at which the latest value was received, whether through
    /// polling or through a trigger.
    ///
    /// # Limitation
    ///
    /// This is *not* a live view.
    pub fn get_updated(&self) -> chrono::DateTime<chrono::UTC> {
        self.updated.clone()
    }
}

/// An output operation available on an endpoint.
#[derive(Debug, Clone)]
pub struct Output {
    /// The kind of value that can be sent to this endpoint.
    kind: ValueKind,

    /// If `Some(duration)`, this endpoint supports pushing,
    /// i.e. the FoxBox can send values.
    push: Option<Duration>,

    /// Date at which the latest value was sent to the endpoint.
    updated: chrono::DateTime<chrono::UTC>,
}

impl Output {
    /// The kind of value that can be sent to this endpoint.
    pub fn get_kind(&self) -> ValueKind {
        self.kind.clone()
    }

    /// If `Some(duration)`, this endpoint supports pushing,
    /// i.e. the FoxBox can send values.
    pub fn get_push(&self) -> Option<Duration> {
        self.push.clone()
    }

    /// Date at which the latest value was sent.
    ///
    /// # Limitation
    ///
    /// This is *not* a live view.
    pub fn get_updated(&self) -> chrono::DateTime<chrono::UTC> {
        self.updated.clone()
    }
}

/// An endpoint represents a single place where data can enter or
/// leave a device. Note that endpoints support either a single kind
/// of input or a single kind of output. Devices that support both
/// inputs or outputs, or several kinds of inputs, or several kinds of
/// outputs, are represented as hubs containing several endpoints.
#[derive(Debug, Clone)]
pub struct EndPoint<IO> {
    /// Tags describing the endpoint.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used to regroup endpoints for rules.
    ///
    /// For instance "entrance".
    tags: Vec<String>,

    /// An id unique to this endpoint.
    id: EndPointId,

    /// The hub owning this endpoint.
    hub: HubId,

    /// The update mechanism for this endpoint.
    mechanism: IO,

    /// The last time the device was seen.
    last_seen: chrono::DateTime<chrono::UTC>,
}

impl<IO> EndPoint<IO> {
    /// Tags describing the endpoint.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used to regroup endpoints for rules.
    ///
    /// For instance "entrance".
    pub fn get_tags<'a>(&'a self) -> &'a Vec<String> {
        &self.tags
    }

    /// An id unique to this endpoint.
    pub fn get_id<'a>(&'a self) -> &'a EndPointId {
        &self.id
    }

    /// The hub owning this endpoint.
    pub fn get_hub_id<'a>(&'a self) -> &'a HubId {
        &self.hub
    }

    /// The update mechanism for this endpoint.
    pub fn get_mechanism<'a>(&'a self) -> &'a IO {
        &self.mechanism
    }

    /// The last time the device was seen.
    pub fn get_last_seen(&self) -> chrono::DateTime<chrono::UTC> {
        self.last_seen
    }
}

