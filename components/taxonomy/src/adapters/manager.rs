//! The Adapter Manager.
//!
//! This module offers an API to (un)register `Adapter`s, `Service`s and `Feature`s dynamically.

pub use adapters::adapter::{ Adapter, PerFeature, PerFeatureResult, Service, Feature };
use adapters::backend;
use adapters::backend::*;
use api::error::*;
use api::native::{ TargetMap, User };
use api::selector::*;
use api::services::*;
use io::parse::DeserializeSupport;
use io::types::*;
use misc::util::is_sync;

use std::collections::HashMap;
use std::path::PathBuf;
use std::fmt::Debug;
use std::sync::{ Arc, Mutex, Weak };
use std::sync::atomic::{ AtomicBool, Ordering };
use std::thread;

use sublock::atomlock::*;
use transformable_channels::mpsc::*;

#[derive(Clone, Copy)]
pub enum MethodCall {
    Fetch,
    Send,
    Delete,
}

pub type WatchEventInternals = backend::WatchEventDetails;
pub type ManagerWatchEvent = GenericWatchEvent<backend::WatchEventDetails>;

/// An implementation of the `AdapterManager`.
///
/// This implementation is `Sync` and supports any number of concurrent
/// readers *or* a single writer.
#[derive(Clone)]
pub struct AdapterManager {
    /// The in-memory database is protected by a read-write lock.
    ///
    /// Each method is responsible for determining whether it needs read() or write()
    /// and releasing the lock as soon as possible.
    ///
    /// The Arc is necessary to let WatchGuard release watches upon Drop.
    back_end: Arc<MainLock<State>>,

    tx_watch: Arc<Mutex<RawSender<WatchOp>>>,
}


impl AdapterManager {
    /// Create an empty `AdapterManager`.
    /// This function does not attempt to load any state from the disk.
    pub fn new(db_path: Option<PathBuf>) -> Self {
        // The code should build only if AdapterManager implements Sync.
        is_sync::<AdapterManager>();

        let state = Arc::new(MainLock::new(|liveness| State::new(liveness, db_path)));
        let tx_watch = Arc::new(Mutex::new(Self::handle_watches(Arc::downgrade(&state))));
        AdapterManager {
            back_end: state,
            tx_watch: tx_watch,
        }
    }
}

impl Default for AdapterManager {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AdapterManager {
    /// Add an adapter to the system.
    ///
    /// # Errors
    ///
    /// Returns an error if an adapter with the same id is already present.
    pub fn add_adapter(&self, adapter: Arc<Adapter>) -> Result<(), Error> {
        self.back_end.write().unwrap().add_adapter(adapter)
    }

    /// Remove an adapter from the system, including all its services and channels.
    ///
    /// # Errors
    ///
    /// Returns an error if no adapter with this identifier exists. Otherwise, attempts
    /// to cleanup as much as possible, even if for some reason the system is in an
    /// inconsistent state.
    pub fn remove_adapter(&self, id: &Id<AdapterId>) -> Result<(), Error> {
        self.back_end.write().unwrap().remove_adapter(id)
    }

    /// Add a service to the system. Called by the adapter when a new
    /// service (typically a new device) has been detected/configured.
    ///
    /// The `service` must NOT have any channels yet. Channels must be added through
    /// `add_channel`.
    ///
    /// # Requirements
    ///
    /// The adapter is in charge of making sure that identifiers persist across reboots.
    ///
    /// # Errors
    ///
    /// Returns an error if any of:
    /// - `service` has channels;
    /// - a service with id `service.id` is already installed on the system;
    /// - there is no adapter with id `service.adapter`.
    pub fn add_service(&self, service: Service) -> Result<(), Error> {
        self.back_end.write().unwrap().add_service(service)
    }

    /// Remove a service previously registered on the system. Typically, called by
    /// an adapter when a service (e.g. a device) is disconnected.
    ///
    /// # Error
    ///
    /// Returns an error if any of:
    /// - there is no such service;
    /// - there is an internal inconsistency, in which case this method will still attempt to
    /// cleanup before returning an error.
    pub fn remove_service(&self, id: &Id<ServiceId>) -> Result<(), Error> {
        self.back_end.write().unwrap().remove_service(id)
    }

    /// Add a setter to the system. Typically, this is called by the adapter when a new
    /// service has been detected/configured. Some services may gain/lose getters at
    /// runtime depending on their configuration.
    ///
    /// # Requirements
    ///
    /// The adapter is in charge of making sure that identifiers persist across reboots.
    ///
    /// # Errors
    ///
    /// Returns an error if the adapter is not registered, the parent service is not
    /// registered, or a channel with the same identifier is already registered.
    /// In either cases, this method reverts all its changes.
    pub fn add_feature(&self, feature: Feature) -> Result<(), Error>
    {
        let request = {
            // Acquire and release lock asap.
            try!(self.back_end.write().unwrap().add_feature(feature))
        };
        self.register_watches(request);
        Ok(())
    }

    /// Remove a setter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its getters.
    ///
    /// # Error
    ///
    /// This method returns an error if the setter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    pub fn remove_feature(&self, id: &Id<FeatureId>) -> Result<(), Error> {
        self.back_end.write().unwrap().remove_feature(id)
    }
}

/// A handle to the public API.
impl AdapterManager {
    /// Get the metadata on services matching some conditions.
    ///
    /// A call to `API::get_services(vec![req1, req2, ...])` will return
    /// the metadata on all services matching _either_ `req1` or `req2`
    /// or ...
    pub fn get_services(&self, selectors: Vec<ServiceSelector>) -> Vec<ServiceDescription> {
        self.back_end.read().unwrap().get_services(selectors)
    }

    /// Label a set of services with a set of tags.
    ///
    /// A call to `API::put_service_tag(vec![req1, req2, ...], vec![tag1,
    /// ...])` will label all the services matching _either_ `req1` or
    /// `req2` or ... with `tag1`, ... and return the number of services
    /// matching any of the selectors.
    ///
    /// Some of the services may already be labelled with `tag1`, or
    /// `tag2`, ... They will not change state. They are counted in
    /// the resulting `usize` nevertheless.
    ///
    /// Note that this call is _not live_. In other words, if services
    /// are added after the call, they will not be affected.
    pub fn add_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize
    {
        let (request, result) = {
            // Acquire and release the write lock.
            self.back_end.write().unwrap().add_service_tags(selectors, tags)
        };
        // Background registration
        self.register_watches(request);
        result
    }

    /// Remove a set of tags from a set of services.
    ///
    /// A call to `API::delete_service_tag(vec![req1, req2, ...], vec![tag1,
    /// ...])` will remove from all the services matching _either_ `req1` or
    /// `req2` or ... all of the tags `tag1`, ... and return the number of services
    /// matching any of the selectors.
    ///
    /// Some of the services may not be labelled with `tag1`, or `tag2`,
    /// ... They will not change state. They are counted in the
    /// resulting `usize` nevertheless.
    ///
    /// Note that this call is _not live_. In okther words, if services
    /// are added after the call, they will not be affected.
    pub fn remove_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize {
        self.back_end.write().unwrap().remove_service_tags(selectors, tags)
    }

    /// Get a list of channels matching some conditions
    pub fn get_features(&self, selectors: Vec<FeatureSelector>) -> Vec<FeatureDescription> {
        self.back_end.read().unwrap().get_features(&selectors)
    }

    /// Label a set of channels with a set of tags.
    ///
    /// A call to `API::put_{setter, setter}_tag(vec![req1, req2, ...], vec![tag1,
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
    pub fn add_feature_tags(&self, selectors: Vec<FeatureSelector>, tags: Vec<Id<TagId>>) -> usize
    {
        let (request, result) = {
            // Acquire and release the write lock.
            self.back_end.write().unwrap().add_feature_tags(selectors, tags)
        };
        self.register_watches(request);
        result
    }

    /// Remove a set of tags from a set of channels.
    ///
    /// A call to `API::delete_{setter, setter}_tag(vec![req1, req2, ...], vec![tag1,
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
    pub fn remove_feature_tags(&self, selectors: Vec<FeatureSelector>, tags: Vec<Id<TagId>>) -> usize {
        self.back_end.write().unwrap().remove_feature_tags(selectors, tags)
    }

    pub fn place_method_call<T, U, Decoder, Encoder>(&self, method: MethodCall, request: TargetMap<FeatureSelector, Option<T>>, user: User,
            decode: Decoder, encode: Encoder) ->
        PerFeatureResult<Option<U>>
        where Decoder: Fn(&Arc<Format>, T) -> Result<Value, Error>,
              Encoder: Fn(&Arc<Format>, Value) -> Result<U, Error>,
              T: Clone + Debug,
              U: Clone
    {
        // Prepare the request, classified by adapter and feature.
        let mut prepared;
        {
            // Make sure that the lock is released asap.
            prepared = self.back_end.read().unwrap().prepare_method_request(method, request);
        }

        let mut results = Vec::new();
        for (_, (adapter, mut batch)) in prepared.drain() {

            // Convert request to use Value instead of T
            let mut proceed = vec![];
            let mut failed = vec![];
            let mut formats = HashMap::new();
            for RequestDetail { feature: id, signature, payload } in batch.drain(..) {
                match (signature.accepts, payload) {
                    (Expects::Nothing, None)
                        | (Expects::Optional(_), None)
                     => {
                        proceed.push((id.clone(), None));
                    }
                    (Expects::Requires(format), Some(value))
                        | (Expects::Optional(format), Some(value))
                    => {
                        match decode(&format, value) {
                            Err(err) => {
                                failed.push((id, Err(err)));
                                continue;
                            }
                            Ok(value) => { proceed.push((id.clone(), Some(value))); }
                        }
                    }
                    (Expects::Nothing, Some(_)) => {
                        failed.push((id, unimplemented!()));
                        continue;
                    }
                    (Expects::Requires(_), None) => {
                        failed.push((id, unimplemented!()));
                        continue;
                    }
                }
                formats.insert(id, signature.returns);
            }
            let mut got = match method {
                MethodCall::Fetch => adapter.fetch_values(proceed, user.clone()),
                MethodCall::Send => adapter.send_values(proceed, user.clone()),
                MethodCall::Delete => adapter.delete_values(proceed, user.clone()),
            };

            // Convert result to use U instead of Value
            let encoded : Vec<_> = got.drain(..)
                .map(|(id, result)| {
                    let result = match result {
                        Err(err) => Err(err),
                        Ok(returned) => {
                            match formats.get(&id) {
                                None => unimplemented!(), // FIXME: Bug in the adapter
                                Some(ref returns) => {
                                    match ((*returns).clone(), returned) {
                                        (Expects::Nothing, Some(_)) => Err(Error::TypeError("Expected no return value, but the adapter provided one".to_owned())),
                                        (Expects::Nothing, None) |
                                            (Expects::Optional(_), None) => Ok(None),

                                        (Expects::Requires(_), None) => Err(Error::TypeError("Expected a return value, the adapter didn't provide one".to_owned())),
                                        (Expects::Requires(format), Some(value)) |
                                            (Expects::Optional(format), Some(value)) =>
                                                encode(&format, value)
                                                    .map(Some),
                                    }
                                }
                            }
                        }
                    };
                    (id, result)
                })
                .collect();
            results.extend(failed);
            results.extend(encoded);
        }

        results
    }

    /// Watch for any change
    pub fn register_watch(&self, watch: TargetMap<FeatureSelector, Exactly<Arc<AsValue>>>,
        on_event: Box<ExtSender<GenericWatchEvent<WatchEventDetails>>>,
        deserialization: Arc<DeserializeSupport>) -> WatchGuard
  {
        let (request, watch_key, is_dropped) =
        {
            // Acquire and release write lock.
            self.back_end.write()
                .unwrap()
                .prepare_watch_features(watch, on_event, deserialization)
        };

        self.register_watches(request);
        WatchGuard::new(self.tx_watch.lock().unwrap().internal_clone(), watch_key, is_dropped)
    }
}



/// Operations related to watching.
///
/// As the adapter side of operations can be slow, we want to keep them out of the `MainLock`. On the
/// other hand, we want to make sure that they take place in a predictable order, to avoid race
/// conditions. So we delegate them to a specialized background thread.
enum WatchOp {
    /// Start watching a bunch of channels, then register them as being watched.
    Start(WatchRequest, RawSender<()>),

    /// Release a watch, after the corresponding WatchGuard has been dropped.
    Release(WatchKey)
}

impl AdapterManager {
    /// Register watches on the dedicated background thread. This must be done outside of any
    /// lock!
    fn register_watches(&self, request: WatchRequest)
    {
        if request.is_empty() {
            return;
        }
        let (tx, rx) = channel();
        let _ = self.tx_watch.lock().unwrap().send(WatchOp::Start(request, tx));
        let _ = rx.recv();
    }

    /// Start the background thread .
    fn handle_watches(state: Weak<MainLock<State>>) -> RawSender<WatchOp> {
        let (tx, rx) = channel();
        let state = state.clone();
        thread::spawn(move || {
            for msg in rx {
                match state.upgrade() {
                    None => return, // The manager has been dropped.
                    Some(backend) =>
                        match msg {
                            WatchOp::Start(request, tx) => {
                                let add = State::start_watch(request);
                                backend.write().unwrap().register_ongoing_watch(add);
                                let _ = tx.send(());
                            }
                            WatchOp::Release(request) => {
                                backend.write().unwrap().stop_watch(request)
                            }
                        }
                }
            }
        });
        tx
    }
}


/// A data structure that causes cancellation of a watch when dropped.
pub struct WatchGuard {
    tx_owner: Box<ExtSender<WatchOp>>,

    /// The cancellation key.
    key: WatchKey,

    /// Once dropped, the watch callbacks will stopped being called. Note
    /// that dropping this value is not sufficient to cancel the watch, as
    /// the adapters will continue sending updates.
    is_dropped: Arc<AtomicBool>,
}
impl WatchGuard {
    fn new(tx_owner: Box<ExtSender<WatchOp>>, key: WatchKey, is_dropped: Arc<AtomicBool>) -> Self
    {
        WatchGuard {
            tx_owner: tx_owner,
            key: key,
            is_dropped: is_dropped
        }
    }
}
impl Drop for WatchGuard {
    fn drop(&mut self) {
        self.is_dropped.store(true, Ordering::Relaxed);

        // Attempt to release the watch. In the unlikely case that we can't (perhaps if we're
        // dropping during shutdown), don't insist.
        // Note that we background this to avoid any risk of deadlock during the drop.
        let _ = self.tx_owner.send(WatchOp::Release(self.key));
    }
}


impl AdapterManager {
    pub fn stop(&self) {
        self.back_end.write().unwrap().stop()
    }
}



/// An event during watching.
#[derive(Clone, Debug)]
pub enum GenericWatchEvent<T> {
    /// If a range was specified when we registered for watching, `EnterRange` is fired whenever
    /// we enter this range. If `Always` was specified, `EnterRange` is fired whenever a new value
    /// is available. Otherwise, never fired.
    EnterRange {
        /// The channel that sent the value.
        id: Id<FeatureId>,

        /// The actual value.
        value: T
    },

    /// If a range was specified when we registered for watching, `ExitRange` is fired whenever
    /// we exit this range. Otherwise, never fired.
    ExitRange {
        /// The channel that sent the value.
        id: Id<FeatureId>,

        /// The actual value.
        value: T
    },

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was
    /// removed.
    FeatureRemoved {
        id: Id<FeatureId>,
        connection: bool
    },

    /// The set of devices being watched has changed, typically either
    /// because a tag was edited or because a device was
    /// added. Payload is the id of the device that was added.
    FeatureAdded {
        id: Id<FeatureId>,
        connection: bool
    },

    Error {
        id: Id<FeatureId>,
        error: Error
    },
}
impl<T> GenericWatchEvent<T> {
    pub fn convert<F, U>(self, f: F) -> GenericWatchEvent<U>
            where F: Fn(T) -> Result<U, Error>
    {
        use self::GenericWatchEvent::*;
        match self {
            FeatureRemoved {id, connection } => FeatureRemoved { id: id, connection: connection },
            FeatureAdded {id, connection } => FeatureAdded { id: id, connection: connection },
            Error { id, error } => Error { id: id, error: error },
            EnterRange { id, value } => {
                match f(value) {
                    Ok(ok) => EnterRange { id: id, value: ok },
                    Err(err) => Error { id: id, error: err },
                }
            }
            ExitRange { id, value } => {
                match f(value) {
                    Ok(ok) => ExitRange { id: id, value: ok },
                    Err(err) => Error { id: id, error: err },
                }
            }
        }
    }
}