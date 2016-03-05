//! An API for plugging in adapters.

#![allow(dead_code)] // Implementation in progress, code isn't called yet.

use transact::InsertInMap;

use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::api::{ API, WatchOptions, Error as APIError, WatchEvent };
use foxbox_taxonomy::util::*;
use foxbox_taxonomy::values::*;

use std::boxed::FnBox;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::rc::Rc;
use std::sync::mpsc::{ channel, Sender };
use std::thread;

/// An error that took place while communicating with either an adapter or the mechanism that
/// handles registeration of adapters.
#[derive(Debug)]
pub enum Error {
    DuplicateGetter(Id<Getter>),
    NoSuchGetter(Id<Getter>),
    GetterDoesNotSupportPolling(Id<Getter>),
    GetterDoesNotSupportWatching(Id<Getter>),
    GetterRequiresThresholdForWatching(Id<Getter>),

    DuplicateSetter(Id<Setter>),
    NoSuchSetter(Id<Setter>),

    DuplicateService(Id<ServiceId>),
    NoSuchService(Id<ServiceId>),
    TypeError(TypeError),

    DuplicateAdapter(Id<AdapterId>),
    NoSuchAdapter(Id<AdapterId>),
    InvalidValue
}

/// A witness that we are currently watching for a value.
/// Watching stops when the guard is dropped.
pub trait WatchGuard {
}

/// API that adapters must implement
pub trait Adapter: Send {
    /// An id unique to this adapter. This id must persist between
    /// reboots/reconnections.
    fn id(&self) -> Id<AdapterId>;

    /// The name of the adapter.
    fn name(&self) -> &str;
    fn vendor(&self) -> &str;
    fn version(&self) -> &[u32;4];
    // ... more metadata

    /// Request a value from a channel. The FoxBox (not the adapter)
    /// is in charge of keeping track of the age of values.
    fn get_values(&self, set: Vec<Id<Getter>>) -> Vec<Result<Option<Value>, Error>>;

    /// Request that a value be sent to a channel.
    fn set_values(&self, values: Vec<(Id<Setter>, Value)>) -> Result<(), Error>;

    fn register_watch(&self, id: Id<Getter>, threshold: Option<Value>, cb: Box<Fn(Value) + Send>) -> Result<Box<WatchGuard>, Error>;
}

/// Data and metadata on an adapter.
struct AdapterData {
    /// The implementation of the adapter.
    adapter: Box<Adapter>,

    /// The services for this adapter.
    services: HashMap<Id<ServiceId>, Rc<RefCell<Service>>>,
}

impl AdapterData {
    fn new(adapter: Box<Adapter>) -> Self {
        AdapterData {
            adapter: adapter,
            services: HashMap::new(),
        }
    }
}

pub struct AdapterControlState {
    /// Adapters, indexed by their id.
    adapter_by_id: HashMap<Id<AdapterId>, AdapterData>,

    /// Services, indexed by their id.
    service_by_id: HashMap<Id<ServiceId>, Rc<RefCell<Service>>>,

    /// Getters, indexed by their id.
    getter_by_id: HashMap<Id<Getter>, Channel<Getter>>,

    /// Setters, indexed by their id.
    setter_by_id: HashMap<Id<Setter>, Channel<Setter>>,
}

impl AdapterControlState {
    /// Auxiliary function to remove a service, once the mutex has been acquired.
    /// Clients should rather use AdapterControl::remove_service.
    fn aux_remove_service(&mut self, id: &Id<ServiceId>) -> Result<(), Error> {
        let service = match self.service_by_id.remove(&id) {
            None => return Err(Error::NoSuchService(id.clone())),
            Some(service) => service,
        };
        for id in service.borrow().getters.keys() {
            let _ignored = self.getter_by_id.remove(id);
        }
        for id in service.borrow().setters.keys() {
            let _ignored = self.setter_by_id.remove(id);
        }
        Ok(())
    }

    /// Add an adapter to the system.
    ///
    /// # Errors
    ///
    /// Returns an error if an adapter with the same id is already present.
    fn add_adapter(&mut self, adapter: Box<Adapter>, services: Vec<Service>) -> Result<(), Error> { // FIXME: Add the services
        let id = adapter.id();
        match self.adapter_by_id.entry(id.clone()) {
            Entry::Occupied(_) => return Err(Error::DuplicateAdapter(id)),
            Entry::Vacant(entry) => {
                entry.insert(AdapterData::new(adapter));
            }
        }
        let mut added = Vec::with_capacity(services.len());
        for service in services {
            let service_id = service.id.clone();
            match self.add_service(id.clone(), service) {
                Ok(_) => added.push(service_id),
                Err(err) => {
                    // Rollback everything
                    for service in added {
                        let _ignored = self.remove_service(id.clone(), service);
                    }
                    let _ignored = self.adapter_by_id.remove(&id);
                    return Err(err)
                }
            }
        }
        Ok(())
    }

    /// Remove an adapter from the system, including all its services and channels.
    ///
    /// # Errors
    ///
    /// Returns an error if no adapter with this identifier exists. Otherwise, attempts
    /// to cleanup as much as possible, even if for some reason the system is in an
    /// inconsistent state.
    fn remove_adapter(&mut self, id: Id<AdapterId>) -> Result<(), Error> {
        let mut services = match self.adapter_by_id.remove(&id) {
            Some(AdapterData {services: adapter_services, ..}) => {
                adapter_services
            }
            None => return Err(Error::NoSuchAdapter(id)),
        };
        for (service_id, _) in services.drain() {
            let _ignored = self.aux_remove_service(&service_id);
        }
        Ok(())
    }

    /// Add a service to the system. Called by the adapter when a new
    /// service (typically a new device) has been detected/configured.
    ///
    /// # Requirements
    ///
    /// The adapter is in charge of making sure that identifiers persist across reboots.
    ///
    /// # Errors
    ///
    /// Returns an error if the adapter does not exist or a service with the same identifier
    /// already exists, or if the identifier introduces a channel that would overwrite another
    /// channel with the same identifier. In either cases, this method reverts all its changes.
    fn add_service(&mut self, adapter: Id<AdapterId>, service: Service) -> Result<(), Error> {
        // Insert all setters of this service in `setters`.
        // Note that they already appear in `service`, by construction.
        let getters_to_insert = service.getters.iter().map(|(id, getter)| {
            (id.clone(), getter.clone())
        }).collect();
        let insert_getters =
            match InsertInMap::start(&mut self.getter_by_id, getters_to_insert) {
                Err(k) => return Err(Error::DuplicateGetter(k)),
                Ok(transaction) => transaction
            };

        // Insert all setters of this service in `setters`.
        // Note that they already appear in `service`, by construction.
        let setters_to_insert = service.setters.iter().map(|(id, setter)| {
            (id.clone(), setter.clone())
        }).collect();
        let insert_setters =
            match InsertInMap::start(&mut self.setter_by_id, setters_to_insert) {
                Err(k) => return Err(Error::DuplicateSetter(k)),
                Ok(transaction) => transaction
            };

        // Insert in `adapters`.
        let mut services_for_this_adapter =
            match self.adapter_by_id.get_mut(&adapter) {
                None => return Err(Error::NoSuchAdapter(adapter.clone())),
                Some(&mut AdapterData {ref mut services, ..}) => {
                    services
                }
            };
        let id = service.id.clone();
        let service = Rc::new(RefCell::new(service));
        let insert_in_adapters =
            match InsertInMap::start(&mut services_for_this_adapter, vec![(id.clone(), service.clone())]) {
                Err(k) => return Err(Error::DuplicateService(k)),
                Ok(transaction) => transaction
            };

        let insert_in_services =
            match InsertInMap::start(&mut self.service_by_id, vec![(id.clone(), service)]) {
                Err(k) => return Err(Error::DuplicateService(k)),
                Ok(transaction) => transaction
            };

        // If we haven't bailed out yet, leave all this stuff in the maps and sets.
        insert_in_adapters.commit();
        insert_getters.commit();
        insert_setters.commit();
        insert_in_services.commit();
        Ok(())
    }

    /// Remove a service previously registered on the system. Typically, called by
    /// an adapter when a service (e.g. a device) is disconnected.
    ///
    /// # Error
    ///
    /// This method returns an error if the adapter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    fn remove_service(&mut self, adapter: Id<AdapterId>, service_id: Id<ServiceId>) -> Result<(), Error> {
        let _ignored = self.aux_remove_service(&service_id);
        match self.adapter_by_id.get_mut(&adapter) {
            None => Err(Error::NoSuchAdapter(adapter)),
            Some(mut data) => {
                if data.services.remove(&service_id).is_none() {
                    Err(Error::NoSuchService(service_id))
                } else {
                    Ok(())
                }
            }
        }
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
    fn add_getter(&mut self, getter: Channel<Getter>) -> Result<(), Error> {
        let service = match self.service_by_id.get_mut(&getter.service) {
            None => return Err(Error::NoSuchService(getter.service.clone())),
            Some(service) => service
        };
        let getters = &mut service.borrow_mut().getters;
        let insert_in_service = match InsertInMap::start(getters, vec![(getter.id.clone(), getter.clone())]) {
            Ok(transaction) => transaction,
            Err(id) => return Err(Error::DuplicateGetter(id))
        };
        let insert_in_getters = match InsertInMap::start(&mut self.getter_by_id, vec![(getter.id.clone(), getter)]) {
            Ok(transaction) => transaction,
            Err(id) => return Err(Error::DuplicateGetter(id))
        };
        insert_in_service.commit();
        insert_in_getters.commit();
        Ok(())
    }

    /// Remove a setter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its setters.
    ///
    /// # Error
    ///
    /// This method returns an error if the setter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    fn remove_getter(&mut self, id: Id<Getter>) -> Result<(), Error> {
        let setter = match self.getter_by_id.remove(&id) {
            None => return Err(Error::NoSuchGetter(id)),
            Some(setter) => setter
        };
        match self.service_by_id.get_mut(&setter.service) {
            None => Err(Error::NoSuchService(setter.service)),
            Some(service) => {
                if service.borrow_mut().getters.remove(&id).is_none() {
                    Err(Error::NoSuchGetter(id))
                } else {
                    Ok(())
                }
            }
        }
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
    fn add_setter(&mut self, setter: Channel<Setter>) -> Result<(), Error> {
        let service = match self.service_by_id.get_mut(&setter.service) {
            None => return Err(Error::NoSuchService(setter.service.clone())),
            Some(service) => service
        };
        let setters = &mut service.borrow_mut().setters;
        let insert_in_service = match InsertInMap::start(setters, vec![(setter.id.clone(), setter.clone())]) {
            Ok(transaction) => transaction,
            Err(id) => return Err(Error::DuplicateSetter(id))
        };
        let insert_in_setters = match InsertInMap::start(&mut self.setter_by_id, vec![(setter.id.clone(), setter)]) {
            Ok(transaction) => transaction,
            Err(id) => return Err(Error::DuplicateSetter(id))
        };
        insert_in_service.commit();
        insert_in_setters.commit();
        Ok(())
    }

    /// Remove a setter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its setters.
    ///
    /// # Error
    ///
    /// This method returns an error if the setter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    fn remove_setter(&mut self, id: Id<Setter>) -> Result<(), Error> {
        let setter = match self.setter_by_id.remove(&id) {
            None => return Err(Error::NoSuchSetter(id.clone())),
            Some(setter) => setter
        };
        match self.service_by_id.get_mut(&setter.service) {
            None => Err(Error::NoSuchService(setter.service)),
            Some(service) => {
                if service.borrow_mut().setters.remove(&id).is_none() {
                    Err(Error::NoSuchSetter(id))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Dispatch instructions received from the front-end thread.
    fn execute(&mut self, msg: Execute) {
        use self::Execute::*;
        match msg {
            AddAdapter { adapter, services, cb } => cb(self.add_adapter(adapter, services)),
            RemoveAdapter { id, cb } => cb(self.remove_adapter(id)),
            AddService { adapter, service, cb } => cb(self.add_service(adapter, service)),
            RemoveService { adapter, id, cb } => cb(self.remove_service(adapter, id)),
            AddSetter { setter, cb } => cb(self.add_setter(setter)),
            RemoveSetter { id, cb } => cb(self.remove_setter(id)),
            AddGetter { getter, cb } => cb(self.add_getter(getter)),
            RemoveGetter { id, cb } => cb(self.remove_getter(id)),
        }
    }
}

pub type Callback<T, E> = Box<FnBox(Result<T, E>) + Send>;

enum Execute {
    AddAdapter {
        adapter: Box<Adapter>,
        services: Vec<Service>,
        cb: Callback<(), Error>
    },
    RemoveAdapter {
        id: Id<AdapterId>,
        cb: Callback<(), Error>
    },
    AddService {
        adapter: Id<AdapterId>,
        service: Service,
        cb: Callback<(), Error>,
    },
    RemoveService {
        adapter: Id<AdapterId>,
        id: Id<ServiceId>,
        cb: Callback<(), Error>,
    },
    AddGetter {
        getter: Channel<Getter>,
        cb: Callback<(), Error>,
    },
    RemoveGetter {
        id: Id<Getter>,
        cb: Callback<(), Error>,
    },
    AddSetter {
        setter: Channel<Setter>,
        cb: Callback<(), Error>,
    },
    RemoveSetter {
        id: Id<Setter>,
        cb: Callback<(), Error>,
    }
}
enum Op {
    Stop(Sender<()>),
    Execute(Execute)
}
pub struct AdapterControl {
    tx: Sender<Op>,
}

impl AdapterControl {
    /// Create an empty AdapterControl.
    /// This function does not attempt to load any state from the disk.
    pub fn new() -> Self {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let mut state = AdapterControlState {
                adapter_by_id: HashMap::new(),
                service_by_id: HashMap::new(),
                getter_by_id: HashMap::new(),
                setter_by_id: HashMap::new(),
            };
            for msg in rx.iter() {
                match msg {
                    Op::Stop(tx) => {
                        let _ignored = tx.send(());
                        return;
                    },
                    Op::Execute(e) => state.execute(e)
                }
            }
        });
        AdapterControl {
            tx: tx
        }
    }

    fn dispatch(&self, execute: Execute) {
        let _ignore = self.tx.send(Op::Execute(execute));
    }
}
impl Drop for AdapterControl {
    fn drop(&mut self) {
        let (tx, rx) = channel();
        {
            let _ignored = self.tx.send(Op::Stop(tx));
        }
        // At this stage, if `self.tx` is dead, so is `tx`.
        let _ignored = rx.recv();
        // At this stage, we are sure that the dispatch thread is not executing anymore.
        // We don't know how it stopped its execution, but we don't really care either.
    }
}

impl AdapterControl {
    /// Add an adapter to the system.
    ///
    /// # Errors
    ///
    /// Returns an error if an adapter with the same id is already present.
    pub fn add_adapter<T>(&self, adapter: T, services: Vec<Service>, cb: Callback<(), Error>)
	   where T: Adapter + 'static {
       self.dispatch(Execute::AddAdapter {
           adapter: Box::new(adapter),
           services: services,
           cb: cb
       });
   }

    /// Remove an adapter from the system, including all its services and channels.
    ///
    /// # Errors
    ///
    /// Returns an error if no adapter with this identifier exists. Otherwise, attempts
    /// to cleanup as much as possible, even if for some reason the system is in an
    /// inconsistent state.
    pub fn remove_adapter(&self, id: &Id<AdapterId>, cb: Callback<(), Error>) {
        self.dispatch(Execute::RemoveAdapter {
            id: id.clone(),
            cb: cb
        });
    }

    /// Add a service to the system. Called by the adapter when a new
    /// service (typically a new device) has been detected/configured.
    ///
    /// # Requirements
    ///
    /// The adapter is in charge of making sure that identifiers persist across reboots.
    ///
    /// # Errors
    ///
    /// Returns an error if the adapter does not exist or a service with the same identifier
    /// already exists, or if the identifier introduces a channel that would overwrite another
    /// channel with the same identifier. In either cases, this method reverts all its changes.
    pub fn add_service(&self, adapter: &Id<AdapterId>, service: Service, cb: Callback<(), Error>) {
        self.dispatch(Execute::AddService {
            adapter: adapter.clone(),
            service: service,
            cb: cb
        });
    }

    /// Remove a service previously registered on the system. Typically, called by
    /// an adapter when a service (e.g. a device) is disconnected.
    ///
    /// # Error
    ///
    /// This method returns an error if the adapter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    pub fn remove_service(&self, adapter: &Id<AdapterId>, service_id: &Id<ServiceId>, cb: Callback<(), Error>) {
        self.dispatch(Execute::RemoveService {
            adapter: adapter.clone(),
            id: service_id.clone(),
            cb: cb
        });
    }

    /// Add a getter to the system. Typically, this is called by the adapter when a new
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
    pub fn add_getter(&self, getter: Channel<Getter>, cb: Callback<(), Error>) {
        self.dispatch(Execute::AddGetter {
            getter: getter,
            cb: cb
        });
    }

    /// Remove a getter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its getters.
    ///
    /// # Error
    ///
    /// This method returns an error if the getter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    pub fn remove_getter(&self, id: &Id<Getter>, cb: Callback<(), Error>) {
        self.dispatch(Execute::RemoveGetter {
            id: id.clone(),
            cb: cb
        });
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
    pub fn add_setter(&self, setter: Channel<Setter>, cb: Callback<(), Error>) {
        self.dispatch(Execute::AddSetter {
            setter: setter,
            cb: cb
        });
    }

    /// Remove a setter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its setters.
    ///
    /// # Error
    ///
    /// This method returns an error if the setter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    pub fn remove_setter(&self, id: &Id<Getter>, cb: Callback<(), Error>) {
        self.dispatch(Execute::RemoveGetter {
            id: id.clone(),
            cb: cb
        });
    }
}

/// A handle to the public API.
impl API for AdapterControl {
    /// Get the metadata on services matching some conditions.
    ///
    /// A call to `API::get_services(vec![req1, req2, ...])` will return
    /// the metadata on all services matching _either_ `req1` or `req2`
    /// or ...
    fn get_services(&self, _selector: &[ServiceSelector], _cb: Box<FnBox(Vec<Service>)>) {
        unimplemented!()
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
    fn add_service_tag(&self, _set: &[ServiceSelector], _tags: &[String], _cb: Box<FnBox(usize)>) {
        unimplemented!()
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
    fn remove_service_tag(&self, _set: &[ServiceSelector], _tags: &[String], _cb: Box<FnBox(usize)>) {
        unimplemented!()
    }

    /// Get a list of setters matching some conditions
    fn get_getter_channels(&self, _selector: &[GetterSelector], _cb: Box<FnBox(Vec<Channel<Getter>>)>) {
        unimplemented!()
    }
    fn get_setter_channels(&self, _selector: &[SetterSelector], _cb: Box<FnBox(Vec<Channel<Setter>>)>) {
        unimplemented!()
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
    fn add_getter_tag(&self, _selector: &[GetterSelector], _tags: &[String], _cb: Box<FnBox(usize)>) {
        unimplemented!()
    }
    fn add_setter_tag(&self, _selector: &[SetterSelector], _tags: &[String], _cb: Box<FnBox(usize)>) {
        unimplemented!()
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
    fn remove_getter_tag(&self, _getter: &[GetterSelector], _tags: &[String], _cb: Box<FnBox(usize)>) {
        unimplemented!()
    }
    fn remove_setter_tag(&self, _getter: &[SetterSelector], _tags: &[String], _cb: Box<FnBox(usize)>) {
        unimplemented!()
    }

    /// Read the latest value from a set of channels
    fn get_channel_value(&self, _selector: &[GetterSelector], _cb: Box<FnBox(ResultSet<Id<Getter>, Value, APIError>)>) {
        unimplemented!()
    }

    /// Send one value to a set of channels
    fn set_channel_value(&self, _selector: &[Vec<SetterSelector>], _values: Vec<Value>, _cb: Box<FnBox(ResultSet<Id<Setter>, (), APIError>)>) {
        unimplemented!()
    }

    /// Watch for any change
    fn register_channel_watch(&self, _options: Vec<WatchOptions>, _cb: Box<Fn(WatchEvent) + Send + 'static>) -> Self::WatchGuard {
        unimplemented!()
    }

    /// A value that causes a disconnection once it is dropped.
    type WatchGuard = ();
}
