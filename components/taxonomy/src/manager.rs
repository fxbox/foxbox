//! The Adapter manager
//!
//! This structure serves two roles:
//! - adapters use it to (un)register themselves, as well as services and channels;
//! - it exposes an implementation of the taxonomy API.

pub use backend::WatchGuard;

use backend::{ Op, AdapterManagerState };
use adapter::{ Adapter, AdapterManagerHandle };

use api::{ API, Error, ResultMap, TargetMap, WatchEvent };
use selector::*;
use services::*;
use values::{ Range, Value };

use transformable_channels::mpsc::*;

use std::thread;

/// An implementation of the AdapterManager.
#[derive(Clone)]
pub struct AdapterManager {
    tx: RawSender<Op>
}

impl AdapterManager {
    /// Create an empty AdapterManager.
    /// This function does not attempt to load any state from the disk.
    pub fn new() -> Self {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let mut back_end = AdapterManagerState::new();
            for msg in rx {
                back_end.execute(msg);
            }
            // The loop will exit once self.tx is dropped.
        });
        AdapterManager {
            tx: tx
        }
    }

    fn dispatch<T>(&self, op: Op, rx_back: Receiver<T>) -> T {
        // Send to the back-end thread. This will panic iff the back-end thread has
        // already panicked, i.e. probably if one of the adapters has panicked. At
        // this stage, we probably want to relaunch the foxbox.
        self.tx.send(op).unwrap();
        // Again, this will panic iff the back-end thread has already panicked.
        rx_back.recv().unwrap()
    }
}

impl Default for AdapterManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AdapterManagerHandle for AdapterManager {
    /// Add an adapter to the system.
    ///
    /// # Errors
    ///
    /// Returns an error if an adapter with the same id is already present.
    fn add_adapter(&self, adapter: Box<Adapter>) -> Result<(), Error> {
        let (tx, rx) = channel();
        self.dispatch(Op::AddAdapter {
            adapter: adapter,
            tx: tx
        }, rx)
    }

    /// Remove an adapter from the system, including all its services and channels.
    ///
    /// # Errors
    ///
    /// Returns an error if no adapter with this identifier exists. Otherwise, attempts
    /// to cleanup as much as possible, even if for some reason the system is in an
    /// inconsistent state.
    fn remove_adapter(&self, id: &Id<AdapterId>) -> Result<(), Error> {
        let (tx, rx) = channel();
        self.dispatch(Op::RemoveAdapter {
            id: id.clone(),
            tx: tx
        }, rx)
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
    fn add_service(&self, service: Service) -> Result<(), Error> {
        let (tx, rx) = channel();
        self.dispatch(Op::AddService {
            service: service,
            tx: tx
        }, rx)
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
    fn remove_service(&self, id: &Id<ServiceId>) -> Result<(), Error> {
        let (tx, rx) = channel();
        self.dispatch(Op::RemoveService {
            id: id.clone(),
            tx: tx
        }, rx)
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
    fn add_getter(&self, getter: Channel<Getter>) -> Result<(), Error> {
        let (tx, rx) = channel();
        self.dispatch(Op::AddGetter {
            getter: getter,
            tx: tx,
        }, rx)
    }

    /// Remove a setter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its getters.
    ///
    /// # Error
    ///
    /// This method returns an error if the setter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    fn remove_getter(&self, id: &Id<Getter>) -> Result<(), Error> {
        let (tx, rx) = channel();
        self.dispatch(Op::RemoveGetter {
            id: id.clone(),
            tx: tx,
        }, rx)
    }

    /// Add a setter to the system. Typically, this is called by the adapter when a new
    /// service has been detected/configured. Some services may gain/lose setters at
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
    fn add_setter(&self, setter: Channel<Setter>) -> Result<(), Error> {
        let (tx, rx) = channel();
        self.dispatch(Op::AddSetter {
            setter: setter,
            tx: tx,
        }, rx)
    }

    /// Remove a setter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its setters.
    ///
    /// # Error
    ///
    /// This method returns an error if the setter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    fn remove_setter(&self, id: &Id<Setter>) -> Result<(), Error> {
        let (tx, rx) = channel();
        self.dispatch(Op::RemoveSetter {
            id: id.clone(),
            tx: tx
        }, rx)
    }
}

/// A handle to the public API.
impl API for AdapterManager {
    /// Get the metadata on services matching some conditions.
    ///
    /// A call to `API::get_services(vec![req1, req2, ...])` will return
    /// the metadata on all services matching _either_ `req1` or `req2`
    /// or ...
    fn get_services(&self, selectors: Vec<ServiceSelector>) -> Vec<Service> {
        let (tx, rx) = channel();
        self.dispatch(Op::GetServices {
            selectors: selectors,
            tx: tx
        }, rx)
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
    fn add_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize {
        let (tx, rx) = channel();
        self.dispatch(Op::AddServiceTags {
            selectors: selectors,
            tags: tags,
            tx: tx
        }, rx)
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
    /// Note that this call is _not live_. In other words, if services
    /// are added after the call, they will not be affected.
    fn remove_service_tags(&self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize {
        let (tx, rx) = channel();
        self.dispatch(Op::RemoveServiceTags {
            selectors: selectors,
            tags: tags,
            tx: tx,
        }, rx)
    }

    /// Get a list of channels matching some conditions
    fn get_getter_channels(&self, selectors: Vec<GetterSelector>) -> Vec<Channel<Getter>> {
        let (tx, rx) = channel();
        self.dispatch(Op::GetGetterChannels {
            selectors: selectors,
            tx: tx,
        }, rx)
    }
    fn get_setter_channels(&self, selectors: Vec<SetterSelector>) -> Vec<Channel<Setter>> {
        let (tx, rx) = channel();
        self.dispatch(Op::GetSetterChannels {
            selectors: selectors,
            tx: tx,
        }, rx)
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
    fn add_getter_tags(&self, selectors: Vec<GetterSelector>, tags: Vec<Id<TagId>>) -> usize {
        let (tx, rx) = channel();
        self.dispatch(Op::AddGetterTags {
            selectors: selectors,
            tags: tags,
            tx: tx,
        }, rx)

    }
    fn add_setter_tags(&self, selectors: Vec<SetterSelector>, tags: Vec<Id<TagId>>) -> usize {
        let (tx, rx) = channel();
        self.dispatch(Op::AddSetterTags {
            selectors: selectors,
            tags: tags,
            tx: tx,
        }, rx)
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
    fn remove_getter_tags(&self, selectors: Vec<GetterSelector>, tags: Vec<Id<TagId>>) -> usize {
        let (tx, rx) = channel();
        self.dispatch(Op::RemoveGetterTags {
            selectors: selectors,
            tags: tags,
            tx: tx,
        }, rx)
    }
    fn remove_setter_tags(&self, selectors: Vec<SetterSelector>, tags: Vec<Id<TagId>>) -> usize {
        let (tx, rx) = channel();
        self.dispatch(Op::RemoveSetterTags {
            selectors: selectors,
            tags: tags,
            tx: tx,
        }, rx)
    }

    /// Read the latest value from a set of channels
    fn fetch_values(&self, selectors: Vec<GetterSelector>) ->
        ResultMap<Id<Getter>, Option<Value>, Error>
    {
        let (tx, rx) = channel();
        self.dispatch(Op::FetchValues {
            selectors: selectors,
            tx: tx,
        }, rx)
    }

    /// Send a bunch of values to a set of channels
    fn send_values(&self, keyvalues: TargetMap<SetterSelector, Value>) ->
        ResultMap<Id<Setter>, (), Error>
    {
        let (tx, rx) = channel();
        self.dispatch(Op::SendValues {
            keyvalues: keyvalues,
            tx: tx,
        }, rx)
    }

    /// Watch for any change
    fn watch_values(&self, watch: TargetMap<GetterSelector, Exactly<Range>>,
        on_event: Box<ExtSender<WatchEvent>>) -> Self::WatchGuard
    {
        let (tx, rx) = channel();
        let (key, is_dropped) = self.dispatch(Op::RegisterChannelWatch {
            watch: watch,
            on_event: on_event,
            tx: tx
        }, rx);
        WatchGuard::new(Box::new(self.tx.clone()), key, is_dropped)
    }

    /// A value that causes a disconnection once it is dropped.
    type WatchGuard = WatchGuard;
}