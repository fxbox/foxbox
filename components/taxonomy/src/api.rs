/// The API for communicating with devices.

use devices::*;

pub enum Error {
    /// There is no such hub connected to the Foxbox, even indirectly.
    NoSuchHub(HubId),

    /// There is no such endpoint connected to the Foxbox, even indirectly.
    NoSuchEndPoint(EndPointId),

    /// Attempting to set a value with the wrong type
    TypeError,
}

pub struct Value; // FIXME: Define

pub trait Env {
    type Witness;

    ///
    /// # Global API
    ///

    /// Get a snapshot of the entire tree.
    fn get_snapshot(&self) -> Hub;

    ///
    /// # Per hub API
    ///

    fn put_hub_tag(&self, &HubId, String) -> Result<(), Error>;
    fn delete_hub_tag(&self, &HubId, String) -> Result<(), Error>;
    fn get_hub(&self, &HubId) -> Result<Hub, Error>;

    ///
    /// # Per endpoint API
    ///

    fn get_input_endpoint(&self, &EndPointId) -> Result<EndPoint<Input>, Error>;
    fn get_output_endpoint(&self, &EndPointId) -> Result<EndPoint<Output>, Error>;

    fn get_endpoint_value(&self, &EndPoint<Input>) -> Result<Value, Error>;
    fn put_endpoint_value(&self, &EndPoint<Output>, Value) -> Result<(), Error>;

    fn register_watch<F>(&self, &EndPoint<Input>, cb: F) -> Result<Self::Witness, Error>
        where F: Fn(Value) + Send;
}

