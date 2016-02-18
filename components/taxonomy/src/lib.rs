use std::time::Duration;

extern crate chrono;
use chrono::{DateTime, UTC};

///
/// Hubs
///

pub type HubId = String;

/// Metadata on a hub
#[derive(Debug)]
pub struct Hub {
    tags: Vec<String>,

    /// An id unique to this hub.
    id: HubId,
}

///
/// Endpoints
///

pub type EndPointId = String;

#[derive(Debug)]
pub enum IO {
    /// This endpoint supports inputs.
    Input {
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
        updated: DateTime<UTC>,
    },
    Output {
        /// If `Some(duration)`, this endpoint supports pushing,
        /// i.e. the FoxBox can send values.
        push: Option<Duration>,

        /// Date at which the latest value was sent to the endpoint.
        updated: DateTime<UTC>,
    }
}

/// An endpoint represents a single place where data can enter or
/// leave a device.
#[derive(Debug)]
pub struct Endpoint {
    /// Tags describing the endpoint.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used to regroup endpoints for rules.
    ///
    /// For instance "entrance".
    tags: Vec<String>,

    /// An id unique to this endpoint.
    id: EndPointId,

    io: IO,
}
