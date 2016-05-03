//! API for defining new Adapters.

use api::error::*;
use api::native::User;
use api::services::*;
use misc::util::Description;
use io::types::*;

use transformable_channels::mpsc::*;

use std::sync::Arc;

pub type PerFeature<T> = Vec<(Id<FeatureId>, T)>;
pub type PerFeatureResult<T> = PerFeature<Result<T, Error>>;

/// Description of a service.
///
/// Typically, a service represents a physical device, e.g. a light, a heater, a doorlock, etc.
///
/// Services typically implement several `Feature`s.
#[derive(Clone)]
pub struct Service {
    /// Tags describing the service.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used by applications to find services and
    /// services.
    ///
    /// For instance, a user may set tag "entrance" to all services
    /// placed in the entrance of his house, or a tag "blue" to a service
    /// controlling blue lights. An adapter may set tags "plugged" or
    /// "battery" to devices that respectively depend on a plugged
    /// power source or on a battery.
    pub tags: Vec<Id<TagId>>,

    /// An id unique to this service.
    pub id: Id<ServiceId>,

    /// Service properties that are set at creation time.
    /// For instance, these can be device manufacturer, model, etc.
    ///
    /// By opposition to tags, these cannot be used by applications to discover a service, but
    /// they can be used by applications to customize the experience. For instance, if a service
    /// represents a device manufactured by Initrode, it can offer a link to Initrode's customer
    /// support as a property.
    pub properties: Vec<(String, String)>,

    /// Identifier of the adapter for this service.
    pub adapter: Id<AdapterId>,
}

/// Description of a feature of a service.
///
/// Typically, a service implements several `Feature`s. For instance, a service dealing with lights
/// may implement:
///
/// - a `Feature` for turning the lights on and off;
/// - a `Feature` for controlling luminosity;
/// - a `Feature` for controlling color;
/// - ...
///
/// Features are attached to a `Type`, which specifies the values that can be sent to the feature
/// (if the feature supports SEND operations), fetched or watched from the feature (if the feature
/// supports READ/WATCH operations).
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::adapters::adapter::*;
/// use foxbox_taxonomy::misc::util::*;
/// use foxbox_taxonomy::io::serialize::*;
/// use foxbox_taxonomy::io::types::*;
/// use foxbox_taxonomy::library::*;
/// use std::sync::Arc;
///
/// // A Featured used to turn lights on or off.
/// let turn_light_on = Feature {
///   implements: vec!["light/on", "light/x-on"],
///
///   // We support sending on or off.
///   send: Some(Signature {
///     accepts: Expects::Requires(Arc::new(IsOnFormat)),
///     ..Signature::default()
///   }),
///
///   .. Feature::empty(&Id::new("light #1"), &Id::new("lights manager #1"))
/// };
/// ```
#[derive(Clone)]
pub struct Feature<'a> {
    /// Description of the feature, in a format designed for discovery by applications.
    ///
    /// By convention, developers should prefix with "x-" for features that are not standardized yet.
    ///
    /// # Picking a good name
    ///
    /// - `"string"` - **BAD**: not a feature. What does this string represent? Is it a name? A password? etc.
    /// - `"bool"` - **BAD**: not a feature. What does this bool represent?
    /// - `"onoff"` - **BAD**: ambigous feature. Does this turn off the light or the safety?
    ///      Could lead to physical accidents.
    /// - `"light/onoff"` - **OK**: unambiguous, if the format of messages is "on"/"off". Just don't
    ///      accept booleans here.
    /// - `"light/on"` - **GOOD**: unambiguous.
    /// - `"temperature"` - **BAD**: ambiguous feature. Does this set the temperature of a heater, an
    //       oven or a blowtorch? Could lead to physical accidents.
    /// - `"heater/temperature"` - **BAD** ambiguous format. Is the temperature in ºC, ºF or K? Could
    ///      lead to accident.
    /// - `"heater/temperature-c"` - **GOOD** we know the feature, the device and the unit.
    /// - `"camera/image"` - **OK** slightly ambiguous, as the camera could decide of a format,
    ///      a resolution, etc.
    /// - `"camera/image-png"` - **GOOD** we know the feature, the device and the format. Note that
    ///      we still don't know the resolution.
    /// - `"mydevice/warpspeed"` - **BAD** that doesn't look standard, prefer `"x-mydevice/x-warpspeed"`.
    ///
    /// # Good descriptions
    ///
    /// - "heater/temperature-c" - all the data is provided to avoid accidents.
    /// - "light/is-on"
    pub implements: Vec<&'a str>,

    /// Tags describing the feature. Tags are freeform and can also be setup by the user or
    /// third-party applications through the REST API.
    ///
    /// Example tags: "location: bedroom", "use: burglar alarm", "turn me off every night", ...
    pub tags: Vec<&'a str>,

    /// A unique id for this instance of the feature. This id must be unique to the system.
    pub id: Id<FeatureId>,

    /// The service to which this feature belongs. This must be an already-registered service.
    pub service: Id<ServiceId>,

    /// If `Some(sig)`, the feature supports sending values to the device. For instance, a
    /// heater supports sending desired temperatures, a light supports sending "on" or "off".
    /// On the other hand, most sensors do not support sending.
    ///
    /// Most `send` features do not return a value. However, some `send` features may use values,
    /// for instance to provide feedback about the send (e.g. time it took for sending, state
    /// of the device, warnings, etc.).
    pub send: Option<Signature>,

    /// If `Some(sig)`, the feature supports fetching values to the device. For instance, a
    /// temperature sensors supports fetching the temperature, but a text-to-speech device
    /// typically does not support fetching.
    ///
    /// Most `fetch` features do not accept an argument. However, some `fetch` features may
    /// accept an argument, for instance to provide details about how the value should be sent
    /// (e.g. )
    pub fetch: Option<Signature>,

    /// If `Some(sig)`, the feature supports deleting. For instance, a camera supports deleting
    /// values, many devices support deleting custom preferences, but an oven typically does not
    /// support deleting a temperature.
    pub delete: Option<Signature>,

    /// If `Some(sig)`, the feature supports watching changes to the device. For instance, a
    /// motion detector supports watching for motion being detected, or a thermometer for watching
    /// if temperature rises above a certain value.
    ///
    /// The signature's `accepts` represents the condition on values.
    pub watch: Option<Signature>,
}


impl<'a> Feature<'a> {
    /// Define an empty feature
    pub fn empty(id: &Id<FeatureId>, service: &Id<ServiceId>) -> Self {
        Feature {
            id: id.clone(),
            service: service.clone(),

            implements: vec![],
            tags: vec![],

            send: None,
            fetch: None,
            delete: None,
            watch: None,
        }
    }
}


/// The signature of a method in a `Feature`.
#[derive(Clone)]
pub struct Signature {
    /// If `Nothing`, applying the method to the `Feature` never accepts an argument.
    ///
    /// If `Requires(format)`, applying the method to the `Feature` always requires an argument with
    /// format `format`.
    ///
    /// If `Optional(format)`, applying the method to the `Feature` accepts an optional argument with
    /// format `format`.
    pub accepts: Expects<Arc<Format>>,

    /// If `Nothing`, applying the method to the `Feature` never returns a result.
    ///
    /// If `Requires(format)`, applying the method to the `Feature` always returns a result with
    /// format `format`.
    ///
    /// If `Optional(format)`, applying the method to the `Feature` may optionally return a result
    /// with format `format`.
    pub returns: Expects<Arc<Format>>,
}
impl Default for Signature {
    fn default() -> Self {
        Signature {
            accepts: Expects::Nothing,
            returns: Expects::Nothing,
        }
    }
}
impl Description for Signature {
    fn description(&self) -> String {
        format!("accepts: {}, returns: {}", self.accepts.description(), self.returns.description())
    }
}

impl Description for Expects<Arc<Format>> {
    fn description(&self) -> String {
        match *self {
            Expects::Nothing => "(nothing)".to_owned(),
            Expects::Requires(ref format) => format.description(),
            Expects::Optional(ref format) => format!("{} (optional)", format.description())
        }
    }
}

impl Description for Option<Signature> {
    fn description(&self) -> String {
        match *self {
            None => "not supported".to_owned(),
            Some(ref sig) => sig.description()
        }
    }
}

/// A witness that we are currently watching for a value.
/// Watching stops when the guard is dropped.
pub trait AdapterWatchGuard : Send + Sync {
}

pub enum WatchEvent {
    /// Fired when we enter the range specified when we started watching, or if no range was
    /// specified, fired whenever a new value is available.
    Enter {
        id: Id<FeatureId>,
        value: Value
    },

    /// Fired when we exit the range specified when we started watching. If no range was
    /// specified, never fired.
    Exit {
        id: Id<FeatureId>,
        value: Value
    }
}


/// API that adapters must implement.
///
/// # Requirements
///
/// Channels and Services are expected to have a stable id, which persists between reboots
/// and { dis, re }connections.
///
/// Note that all methods are blocking. However, the underlying implementatino of adapters is
/// expected to either return quickly or be able to handle several requests concurrently.
pub trait Adapter: Send + Sync {
    /// An id unique to this adapter. This id must persist between
    /// reboots/reconnections.
    fn id(&self) -> Id<AdapterId>;

    /// The name of the adapter.
    fn name(&self) -> &str;
    fn vendor(&self) -> &str;
    fn version(&self) -> &[u32;4];
    // ... more metadata

    /// Fetch a batch of values provided by features on behalf of a user.
    ///
    /// Most implementations will return `None` for each successful fetch.
    ///
    /// The default implementation returns an error mentioning that these features do not offer
    /// the `fetch` method.
    fn fetch_values(&self, batch: PerFeature<Option<Value>>, _: User)
        -> PerFeature<Result<Option<Value>, Error>>
    {
        undefined_method("fetch", batch)
    }

    /// Send a batch of values through features on behalf of a user.
    ///
    /// Most implementations will expect `None` for each send.
    ///
    /// The default implementation returns an error mentioning that these features do not offer
    /// the `fetch` method.
    fn send_values(&self, batch: PerFeature<Option<Value>>, _: User)
        -> PerFeature<Result<Option<Value>, Error>>
    {
        undefined_method("send", batch)
    }

    /// Send a batch of values through features on behalf of a user.
    ///
    /// Most implementations will expect `None` for each send.
    ///
    /// The default implementation returns an error mentioning that these features do not offer
    /// the `fetch` method.
    fn delete_values(&self, batch: PerFeature<Option<Value>>, _: User)
        -> PerFeature<Result<Option<Value>, Error>>
    {
        undefined_method("delete", batch)
    }

    /// Watch a bunch of getters as they change.
    ///
    /// The `AdapterManager` always attempts to group calls to `fetch_values` by `Adapter`, and
    /// then expects the adapter to attempt to minimize the connections with the actual devices.
    /// The Adapter should however be ready to handle concurrent `register_watch` on the same
    /// devices, possibly with distinct `Option<Range>` options.
    ///
    /// If a `Range` option is set, the watcher expects to receive `EnterRange`/`ExitRange` events
    /// whenever the value available on the device enters/exits the range. If the `Range` is
    /// a `Range::Eq(x)`, the adapter may decide to reject the request or to interpret it as
    /// a `Range::BetweenEq { min: x, max: x }`.
    ///
    /// If no `Range` option is set, the watcher expects to receive `EnterRange` events whenever
    /// a new value is available on the device. The adapter may decide to reject the request if
    /// this is clearly not the expected usage for a device, or to throttle it.
    ///
    /// # Edge cases
    ///
    /// Note that the same `Id<FeatureId>` may appear several times. This is by design and adapters
    /// should handle this case, optimizing it if possible.
    ///
    /// Similarly, successive calls to `register_watch` may end up watching the same getter. The
    /// adapter should handle this case, optimizing it if possible.
    fn register_watch(&self, batch: PerFeature<(Option<Value>, Box<ExtSender<WatchEvent>>)>) ->
        PerFeatureResult<Box<AdapterWatchGuard>>
    {
        undefined_method("watch", batch)
    }

    /// Signal the adapter that it is time to stop.
    ///
    /// Ideally, the adapter should not return until all its threads have been stopped.
    ///
    /// The default implementation does nothing.
    fn stop(&self) {
        // By default, do nothing.
    }
}

/// Utility function
fn undefined_method<T, U>(method_name: &str, mut batch: PerFeature<T>)
    -> PerFeature<Result<U, Error>>
{
    batch.drain(..)
        .map(|(id, _)| {
            (id.clone(), Err(Error::InternalError(InternalError::NoSuchMethod(id, method_name.to_owned()))))
        })
        .collect()
}
