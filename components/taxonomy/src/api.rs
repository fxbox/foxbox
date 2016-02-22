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

/// The public API.
///
/// This API is subdivided in traits purely for the sake of
/// namespacing.
pub trait API {
    /// The subset of the API dedicated to nodes.
    type NodeAPI: NodeAPI;
    fn get_node_api(&self) -> Self::NodeAPI;

    /// The subset of the API dedicated to services.
    type ServiceAPI: ServiceAPI;
    fn get_service_api(&self) -> Self::ServiceAPI;
}

/// Node-level API
pub trait NodeAPI {
    /// Get a list of nodes matching some conditions
    ///
    /// # REST API
    ///
    /// `GET /api/v1/node/list`
    fn get_list(&self, &NodeRequest) -> Vec<Node>;

    /// Add a tag to an existing node.
    ///
    /// Tags can be used to locate nodes.
    ///
    /// # REST API
    ///
    /// `PUT /api/v1/node/tag/$NodeId`
    fn put_tag(&self, &NodeId, String) -> Result<(), Error>;

    /// Add a tag to an existing node.
    ///
    /// Tags can be used to locate nodes.
    ///
    /// # REST API
    ///
    /// `DELETE /api/v1/node/tag/$NodeId`
    fn delete_tag(&self, &NodeId, String) -> Result<(), Error>;
}

/// Service-level API
pub trait ServiceAPI {
    /// A value that causes a disconnection once it is dropped.
    type Guard;
    
    /// Get a list of inputs matching some conditions
    ///
    /// # REST API
    ///
    /// `GET /api/v1/service/list`
    fn get_input_services(&self, &InputRequest) -> Vec<Service<Input>>;
    fn get_output_services(&self, &OutputRequest) -> Vec<Service<Output>>;

    /// Add a tag to an existing service.
    ///
    /// Tags can be used to locate service.
    ///
    /// # REST API
    ///
    /// `PUT /api/v1/service/tag/$ServiceId`
    fn put_tag(&self, &ServiceId, String) -> Result<(), Error>;

    /// Add a tag to an existing service.
    ///
    /// Tags can be used to locate services.
    ///
    /// # REST API
    ///
    /// `DELETE /api/v1/service/tag/$ServiceId`
    fn delete_tag(&self, &ServiceId, String) -> Result<(), Error>;

    /// Read one value from an input enpoint
    ///
    /// # REST API
    ///
    /// GET /api/v1/service/value/$ServiceId
    fn get_service_value(&self, &Service<Input>) -> Result<Value, Error>;

    /// Send one value to an output enpoint
    ///
    /// # REST API
    ///
    /// `PUT /api/v1/service/value/$ServiceId`
    fn put_service_value(&self, &Service<Output>, Value) -> Result<(), Error>;

    /// Watch for any change
    ///
    /// # WebSocket API
    ///
    /// `/api/v1/service/watch/$ServiceId`
    fn register_watch<F>(&self, &Service<Input>, &WatchOptions, cb: F)
                         -> Result<Self::Guard, Error>
        where F: Fn(Value) + Send;
}

/// Options for watching a service.
/// FIXME: Define.
pub struct WatchOptions;
