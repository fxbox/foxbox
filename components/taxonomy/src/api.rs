//!
//! The API for communicating with devices.
//!
//! This API is provided as Trait to be implemented:
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

use std::time::Duration;

pub enum Error {
    /// There is no such hub connected to the Foxbox, even indirectly.
    NoSuchHub(HubId),

    /// There is no such endpoint connected to the Foxbox, even indirectly.
    NoSuchEndPoint(EndPointId),

    /// Attempting to set a value with the wrong type
    TypeError,
}

pub struct Value; // FIXME: Define

/// The public API.
pub trait API {
    /// The subset of the API dedicated to hubs.
    type HubAPI: HubAPI;
    fn get_hub_api(&self) -> Self::HubAPI;

    /// The subset of the API dedicated to endpoints.
    type EndPointAPI: EndPointAPI;
    fn get_endpoint_api(&self) -> Self::EndPointAPI;
}

// FIXME: We should probably use traits for building requests, as this
// will be more future-proof.
/// A request for one or more hubs.

pub struct HubRequest {
    /// If `Some(id)`, return only the hub with the corresponding id.
    pub id: Option<HubId>,

    ///  Restrict results to hubs that have all the tags in `tags`.
    pub tags: Vec<String>,

    /// Restrict results to hubs that have all the inputs in `inputs`.
    pub inputs: Vec<InputRequest>,

    /// Restrict results to hubs that have all the outputs in `outputs`.
    pub outputs: Vec<OutputRequest>,
}

pub struct Period {
    pub min: Option<Duration>,
    pub max: Option<Duration>,
}

pub struct InputRequest {
    /// If `Some(id)`, return only the endpoint with the corresponding id.
    pub id: Option<EndPointId>,

    /// If `Some(id)`, return only endpoints that are immediate children
    /// of hub `id`.
    pub parent: Option<HubId>,

    /// If `Some(id)`, return only endpoints that are descendants of hub
    /// `id`.
    pub ancestor: Option<HubId>,

    ///  Restrict results to endpoints that have all the tags in `tags`.
    pub tags: Vec<String>,

    /// If `Some(k)`, restrict results to endpoints that produce values
    /// of kind `k`.
    pub kind: Option<ValueKind>,

    /// If `Some(r)`, restrict results to endpoints that support polling
    /// with the acceptable period.
    pub poll: Option<Period>,

    /// If `Some(r)`, restrict results to endpoints that support trigger
    /// with the acceptable period.
    pub trigger: Option<Period>,
}

pub struct OutputRequest {
    /// If `Some(id)`, return only the endpoint with the corresponding id.
    pub id: Option<EndPointId>,

    /// If `Some(id)`, return only endpoints that are immediate children
    /// of hub `id`.
    pub parent: Option<HubId>,

    /// If `Some(id)`, return only endpoints that are descendants of hub
    /// `id`.
    pub ancestor: Option<HubId>,

    ///  Restrict results to endpoints that have all the tags in `tags`.
    pub tags: Vec<String>,

    /// If `Some(k)`, restrict results to endpoints that accept values
    /// of kind `k`.
    pub kind: Option<ValueKind>,

    /// If `Some(r)`, restrict results to endpoints that support pushing
    /// with the acceptable period.
    pub push: Option<Period>,
}


///
/// Hub-level API
///
pub trait HubAPI {
    /// Get a list of hubs matching some conditions
    ///
    /// # REST API
    ///
    /// GET /api/v1/hub/list
    fn get_list(&self, &HubRequest) -> Vec<Hub>;

    /// Add a tag to an existing hub.
    ///
    /// Tags can be used to locate hubs.
    ///
    /// # REST API
    ///
    /// PUT /api/v1/hub/tag/$HubId
    fn put_tag(&self, &HubId, String) -> Result<(), Error>;

    /// Add a tag to an existing hub.
    ///
    /// Tags can be used to locate hubs.
    ///
    /// # REST API
    ///
    /// DELETE /api/v1/hub/tag/$HubId
    fn delete_tag(&self, &HubId, String) -> Result<(), Error>;
}

///
/// Endpoint-level API
///
pub trait EndPointAPI {
    /// A value that causes a disconnection once it is dropped.
    type Guard;
    
    /// Get a list of inputs matching some conditions
    ///
    /// # REST API
    ///
    /// GET /api/v1/endpoint/list
    fn get_input_endpoints(&self, &InputRequest) -> Vec<EndPoint<Input>>;
    fn get_output_endpoints(&self, &OutputRequest) -> Vec<EndPoint<Output>>;

    /// Add a tag to an existing endpoint.
    ///
    /// Tags can be used to locate endpoint.
    ///
    /// # REST API
    ///
    /// PUT /api/v1/endpoint/tag/$EndPointId
    fn put_tag(&self, &EndPointId, String) -> Result<(), Error>;

    /// Add a tag to an existing endpoint.
    ///
    /// Tags can be used to locate endpoints.
    ///
    /// # REST API
    ///
    /// DELETE /api/v1/endpoint/tag/$EndPointId
    fn delete_tag(&self, &EndPointId, String) -> Result<(), Error>;

    /// Read one value from an input enpoint
    ///
    /// # REST API
    ///
    /// GET /api/v1/endpoint/value/$EndpointId
    fn get_endpoint_value(&self, &EndPoint<Input>) -> Result<Value, Error>;

    /// Send one value to an output enpoint
    ///
    /// # REST API
    ///
    /// PUT /api/v1/endpoint/value/$EndpointId
    fn put_endpoint_value(&self, &EndPoint<Output>, Value) -> Result<(), Error>;

    /// Watch for any change
    // FIXME: Optional argument.
    fn register_watch<F>(&self, &EndPoint<Input>, cb: F) -> Result<Self::Guard, Error>
        where F: Fn(Value) + Send;
}

