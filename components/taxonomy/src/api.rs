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

use devices::*;
use requests::*;
use values::Value;

/// An error produced by one of the APIs in this module.
pub enum Error {
    /// There is no such node connected to the Foxbox, even indirectly.
    NoSuchNode(NodeId),

    /// There is no such service connected to the Foxbox, even indirectly.
    NoSuchService(ServiceId),

    /// Attempting to set a value with the wrong type
    TypeError,
}

/// An event during watching.
pub enum WatchEvent {
    /// A new value is available.
    Value(Value),

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was connected or
    /// disconnected. Value `SetChanged(n)` means that the set now
    /// holds `n` input services.
    SetChanged(usize),
}

/// The public API.
pub trait API {
    /// Get a list of nodes matching some conditions
    ///
    /// # REST API
    ///
    /// `GET /api/v1/node/list`
    fn get_nodes(&NodeRequest) -> Vec<Node>;

    /// Add a tag to an existing node.
    ///
    /// Tags can be used to locate nodes.
    ///
    /// # REST API
    ///
    /// `PUT /api/v1/node/tag/$NodeId`
    fn put_node_tag(&NodeId, String) -> Result<(), Error>;

    /// Add a tag to an existing node.
    ///
    /// Tags can be used to locate nodes.
    ///
    /// # REST API
    ///
    /// `DELETE /api/v1/node/tag/$NodeId`
    fn delete_node_tag(&NodeId, String) -> Result<(), Error>;
    
    /// Get a list of inputs matching some conditions
    ///
    /// # REST API
    ///
    /// `GET /api/v1/service/list`
    fn get_input_services(&InputRequest) -> Vec<Service<Input>>;
    fn get_output_services(&OutputRequest) -> Vec<Service<Output>>;

    /// Add a tag to an existing service.
    ///
    /// Tags can be used to locate service.
    ///
    /// # REST API
    ///
    /// `PUT /api/v1/service/tag/$ServiceId`
    fn put_service_tag(&ServiceId, String) -> Result<(), Error>;

    /// Add a tag to an existing service.
    ///
    /// Tags can be used to locate services.
    ///
    /// # REST API
    ///
    /// `DELETE /api/v1/service/tag/$ServiceId`
    fn delete_service_tag(&ServiceId, String) -> Result<(), Error>;

    /// Read one value from an input enpoint
    ///
    /// # REST API
    ///
    /// GET /api/v1/service/value/$ServiceId
    fn get_service_value(&Service<Input>) -> Result<Value, Error>;

    /// Send one value to an output enpoint
    ///
    /// # REST API
    ///
    /// `PUT /api/v1/service/value/$ServiceId`
    fn put_service_value(&Service<Output>, Value) -> Result<(), Error>;

    /// Watch for any change
    ///
    /// # WebSocket API
    ///
    /// `/api/v1/service/watch/$ServiceId`
    fn register_service_watch<F>(WatchOptions, cb: F)
                                 -> Result<Self::WatchGuard, Error>
        where F: Fn(WatchEvent) + Send;

    /// A value that causes a disconnection once it is dropped.
    type WatchGuard;
}

/// Options for watching changes in one or more services.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WatchOptions {
    /// The set of inputs to watch. Note that the actual inputs in the
    /// set may change over time.
    source: InputRequest,
}

impl WatchOptions {
    pub fn new() -> Self {
        WatchOptions {
            source: InputRequest::new(),
        }
    }

    /// Restrict to input services in a given set. Note that if all
    /// the inputs do not have the same `ServiceKind`, the call to
    /// `register_service_watch` will fail.
    ///
    /// Also note that the actual input services that are part of the
    /// set may change with time, for instance if devices are added
    /// ore removed.  as it changes.
    pub fn with_inputs(self, req: InputRequest) -> Self {
        WatchOptions {
            source: self.source.and(req),
            ..self
        }
    }
}
