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
use selector::*;
use values::Value;
use util::Id;

/// An error produced by one of the APIs in this module.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Error {
    /// There is no such node connected to the Foxbox, even indirectly.
    NoSuchNode(Id<NodeId>),

    /// There is no such input channel connected to the Foxbox, even indirectly.
    NoSuchGet(Id<Get>),

    /// There is no such output channel connected to the Foxbox, even indirectly.
    NoSuchSet(Id<Set>),

    /// Attempting to set a value with the wrong type
    TypeError,
}

/// An event during watching.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WatchEvent {
    /// A new value is available.
    Value {
        /// The channel that sent the value.
        from: Id<Get>,

        /// The actual value.
        value: Value
    },

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was
    /// removed. Payload is the id of the device that was removed.
    GetRemoved(Id<Get>),

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was
    /// added. Payload is the id of the device that was added.
    GetAdded(Id<Get>),
}

/// A handle to the public API.
pub trait API: Send {
    /// Get the metadata on nodes matching some conditions.
    ///
    /// A call to `API::get_nodes(vec![req1, req2, ...])` will return
    /// the metadata on all nodes matching _either_ `req1` or `req2`
    /// or ...
    ///
    /// # REST API
    ///
    /// `GET /api/v1/nodes`
    ///
    /// ## Gets
    ///
    /// Any JSON that can be deserialized to a `Vec<NodeSelector>`. See
    /// the implementation of `NodeSelector` for details.
    ///
    /// ### Example
    ///
    /// Selector all doors in the entrance (tags `door`, `entrance`)
    /// that support output channel `OpenClose`
    ///
    /// ```json
    /// [{
    ///   "tags": ["entrance", "door"],
    ///   "inputs": [
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
    /// A JSON representing an array of `Node`. See the implementation
    /// of `Node` for details.
    ///
    /// ### Example
    ///
    /// ```json
    /// [{
    ///   "tags": ["entrance", "door", "somevendor"],
    ///   "id: "some-node-id",
    ///   "inputs": [],
    ///   "outputs": [
    ///     "tags": [...],
    ///     "id": "some-channel-id",
    ///     "node": "some-node-id",
    ///     "last_seen": "some-date",
    ///     "mechanism": {
    ///       "Set":  {
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
    fn get_nodes(&self, &Vec<NodeSelector>) -> Vec<Node>;

    /// Label a set of nodes with a set of tags.
    ///
    /// A call to `API::put_node_tag(vec![req1, req2, ...], vec![tag1,
    /// ...])` will label all the nodes matching _either_ `req1` or
    /// `req2` or ... with `tag1`, ... and return the number of nodes
    /// matching any of the selectors.
    ///
    /// Some of the nodes may already be labelled with `tag1`, or
    /// `tag2`, ... They will not change state. They are counted in
    /// the resulting `usize` nevertheless.
    ///
    /// Note that this call is _not live_. In other words, if nodes
    /// are added after the call, they will not be affected.
    ///
    /// # REST API
    ///
    /// `POST /api/v1/nodes/tag`
    ///
    /// ## Gets
    ///
    /// Any JSON that can be deserialized to
    ///
    /// ```ignore
    /// {
    ///   set: Vec<NodeSelector>,
    ///   tags: Vec<String>,
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
    fn put_node_tag(&self, set: &Vec<NodeSelector>, tags: &Vec<String>) -> usize;

    /// Remove a set of tags from a set of nodes.
    ///
    /// A call to `API::delete_node_tag(vec![req1, req2, ...], vec![tag1,
    /// ...])` will remove from all the nodes matching _either_ `req1` or
    /// `req2` or ... all of the tags `tag1`, ... and return the number of nodes
    /// matching any of the selectors.
    ///
    /// Some of the nodes may not be labelled with `tag1`, or `tag2`,
    /// ... They will not change state. They are counted in the
    /// resulting `usize` nevertheless.
    ///
    /// Note that this call is _not live_. In other words, if nodes
    /// are added after the call, they will not be affected.
    ///
    /// # REST API
    ///
    /// `DELETE /api/v1/nodes/tag`
    ///
    /// ## Gets
    ///
    /// Any JSON that can be deserialized to
    ///
    /// ```ignore
    /// {
    ///   set: Vec<NodeSelector>,
    ///   tags: Vec<String>,
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
    fn delete_node_tag(&self, set: &Vec<NodeSelector>, tags: String) -> usize;
    
    /// Get a list of inputs matching some conditions
    ///
    /// # REST API
    ///
    /// `GET /api/v1/channels`
    fn get_input_channels(&self, &Vec<GetSelector>) -> Vec<Channel<Get>>;
    fn get_output_channels(&self, &Vec<SetSelector>) -> Vec<Channel<Set>>;

    /// Label a set of channels with a set of tags.
    ///
    /// A call to `API::put_{input, output}_tag(vec![req1, req2, ...], vec![tag1,
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
    /// ## Gets
    ///
    /// Any JSON that can be deserialized to
    ///
    /// ```ignore
    /// {
    ///   set: Vec<GetSelector>,
    ///   tags: Vec<String>,
    /// }
    /// ```
    /// or
    /// ```ignore
    /// {
    ///   set: Vec<SetSelector>,
    ///   tags: Vec<String>,
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
    fn put_input_tag(&self, &Vec<GetSelector>, &Vec<String>) -> usize;
    fn put_output_tag(&self, &Vec<SetSelector>, &Vec<String>) -> usize;

    /// Remove a set of tags from a set of channels.
    ///
    /// A call to `API::delete_{input, output}_tag(vec![req1, req2, ...], vec![tag1,
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
    /// ## Gets
    ///
    /// Any JSON that can be deserialized to
    ///
    /// ```ignore
    /// {
    ///   set: Vec<GetSelector>,
    ///   tags: Vec<String>,
    /// }
    /// ```
    /// or
    /// ```ignore
    /// {
    ///   set: Vec<SetSelector>,
    ///   tags: Vec<String>,
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
    fn delete_input_tag(&self, &Vec<GetSelector>, &Vec<String>) -> usize;
    fn delete_output_tag(&self, &Vec<GetSelector>, &Vec<String>) -> usize;

    /// Read the latest value from a set of channels
    ///
    /// # REST API
    ///
    /// `GET /api/v1/channels/value`
    fn get_channel_value(&self, &Vec<GetSelector>) -> Vec<(Id<Get>, Result<Value, Error>)>;

    /// Send one value to a set of channels
    ///
    /// # REST API
    ///
    /// `POST /api/v1/channels/value`
    fn put_channel_value(&self, &Vec<SetSelector>, Value) -> Vec<(Id<Set>, Result<(), Error>)>;

    /// Watch for any change
    ///
    /// # WebSocket API
    ///
    /// `/api/v1/channels/watch`
    fn register_channel_watch(&self, Vec<WatchOptions>, cb: Box<Fn(WatchEvent) + Send + 'static>) -> Self::WatchGuard;

    /// A value that causes a disconnection once it is dropped.
    type WatchGuard;
}

/// Options for watching changes in one or more channels.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WatchOptions {
    /// The set of inputs to watch. Note that the actual inputs in the
    /// set may change over time.
    pub source: GetSelector,

    /// If `true`, watch as new values become available.
    pub should_watch_values: bool,

    /// If `true`, watch as nodes are connected/disconnected.
    pub should_watch_topology: bool,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}

impl WatchOptions {
    pub fn new() -> Self {
        WatchOptions {
            source: GetSelector::new(),
            should_watch_values: false,
            should_watch_topology: false,
            private: (),
        }
    }

    /// Restrict to input channels in a given set.
    ///
    /// Also note that the actual input channels that are part of the
    /// set may change with time, for instance if devices are added
    /// ore removed.  The selector _is live_, i.e. the channel watch
    /// will continue watching any input channels that match `req`.
    pub fn with_inputs(self, req: GetSelector) -> Self {
        WatchOptions {
            source: self.source.and(req),
            ..self
        }
    }

    pub fn with_watch_values(self, should: bool) -> Self {
        WatchOptions {
            should_watch_values: should,
            ..self
        }
    }

    pub fn with_watch_topology(self, should: bool) -> Self {
        WatchOptions {
            should_watch_topology: should,
            ..self
        }
    }
}
