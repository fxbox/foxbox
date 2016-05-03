//! This module contains the intelligence of the Taxonomy API.
//!
//! If we describe foxbox as an Operating System, this is the _Driver Manager_. It lets third-party
//! modules add or remove `Adapter`s dynamically, turns high-level API calls that know nothing of
//! individual `Adapter`s into low-level per-adapter calls, it monitors the topology to adapt
//! high-level watch requests, it handles persistence of tags to disk, etc.

use adapters::adapter::{ Adapter, AdapterWatchGuard, Feature, Service, Signature, WatchEvent as AdapterWatchEvent };
use adapters::manager::{ MethodCall, GenericWatchEvent as WatchEvent };
use adapters::tag_storage::TagStorage;
use api::error::*;
use api::native::{ TargetMap, Targetted };
use api::selector::*;
use api::services::*;
use io::parse::*;
use io::types::*;
use misc::transact::InsertInMap;
use misc::util::Description;

use std::collections::{ HashMap, HashSet };
use std::collections::hash_map::Entry;
use std::hash::{ Hash, Hasher };
use std::ops::{ Deref };
use std::path::PathBuf;
use std::fmt::Debug;
use std::sync::{ Arc, Mutex, Weak };
use std::sync::atomic::{ AtomicBool, Ordering };

use sublock::atomlock::*;
use transformable_channels::mpsc::*;

use serde::de::Deserialize;

// In release build, log an error and continue.
// In debug build, log an error and panic.
#[macro_export]
macro_rules! log_debug_assert {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            error!($($arg)*);
            panic!($($arg)*);
        }
    };
}

/// A bunch of requests to individual features, grouped by adapter.
pub type AdapterRequest<T> = HashMap<Id<AdapterId>, (Arc<Adapter>, Vec<RequestDetail<T>>)>;

/// Individual request to a feature, as present in an `AdapterRequest`.
pub struct RequestDetail<T> {
    /// The feature targeted.
    pub feature: Id<FeatureId>,

    /// The signature of the method for `feature`.
    pub signature: Signature,

    /// Actual payload (typically, the data sent to `feature`).
    pub payload: T,
}

pub type WatchRequest = AdapterRequest<(Option<Arc<AsValue>> /* The condition */, Weak<WatcherData>)>;
pub struct WatchEventDetails {
    pub value: Value,
    pub format: Arc<Format>,
}
pub type MethodRequest<T> = AdapterRequest<Option<T>>;

pub type WatchGuardCommit = Vec<(Weak<WatcherData>, Vec<(Id<FeatureId>, Box<AdapterWatchGuard>)>)>;


struct FeatureData {
    /// The tags, as in a Service.
    tags: Arc<SubCell<HashSet<Id<TagId>>>>,
    id: Id<FeatureId>,
    service: Id<ServiceId>,
    weak_service: Weak<SubCell<ServiceData>>,
    adapter: Id<AdapterId>,
    implements: HashSet<Id<ImplementId>>,

    /// If `Some(sig)`, the feature supports sending values to the device. For instance, a
    /// heater supports sending desired temperatures, a light supports sending "on" or "off".
    /// On the other hand, most sensors do not support sending.
    pub send: Option<Signature>,

    /// If `Some(sig)`, the feature supports fetching values to the device. For instance, a
    /// temperature sensors supports fetching the temperature, but a text-to-speech device
    /// typically does not support fetching.
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

    watchers: HashMap<WatchKey, Weak<WatcherData>>,
}

impl FeatureData {
    fn new(liveness: &Arc<Liveness>,
            weak_service: Weak<SubCell<ServiceData>>,
            adapter: &Id<AdapterId>,
            mut feature: Feature) -> Self
    {
        FeatureData {
            implements: feature.implements.drain(..).map(|str| Id::new(str)).collect(),
            id: feature.id,
            service: feature.service,
            tags: Arc::new(SubCell::new(liveness, feature.tags.drain(..).map(|str| Id::new(str)).collect())),
            send: feature.send,
            fetch: feature.fetch,
            watch: feature.watch,
            delete: feature.delete,
            watchers: HashMap::new(),
            weak_service: weak_service,
            adapter: adapter.clone(),
        }
    }

    fn get_method(&self, method: MethodCall) -> &Option<Signature> {
        match method {
            MethodCall::Send => &self.send,
            MethodCall::Delete => &self.delete,
            MethodCall::Fetch => &self.fetch,
        }
    }

    fn matches(&self, service: &ServiceData, selector: &FeatureSelector) -> bool {
        match selector.services {
            Exactly::Never => false,
            Exactly::Always => self.matches_without_service(selector),
            Exactly::Exactly(ref services) => {
                let service_fails = services.iter().all(|service_selector| {
                    !service.matches(service_selector)
                });
                if service_fails {
                    false
                } else {
                    self.matches_without_service(selector)
                }
            }
        }
    }

    // Note: This assumes that we have already matched the service.
    fn sub_matches(&self, selector: &SimpleFeatureSelector) -> bool {
        self.matches_without_service(selector)
    }

    fn matches_without_service<T>(&self, selector: &BaseFeatureSelector<T>) -> bool
        where T: Clone + Debug + Deserialize + Default
    {
        if !selector.id.matches(&self.id) {
            return false;
        }
        if !(TagMatch {needs: &selector.tags, has: &*self.tags.borrow()}).matches() {
            return false;
        }

        match selector.implements {
            Exactly::Never => return false,
            Exactly::Exactly(ref implements) if !self.implements.contains(implements) => return false,
            _ => {}
        }
        true
    }
    fn get_description(&self) -> FeatureDescription {
        FeatureDescription {
            id: self.id.clone(),
            service: self.service.clone(),
            adapter: self.adapter.clone(),
            implements: self.implements.iter().cloned().collect(),
            tags: self.tags.borrow().clone(),
            fetch: self.fetch.description(),
            send: self.send.description(),
            watch: self.watch.description(),
            delete: self.delete.description(),
        }
    }
}

/// Information on a service.
///
/// Used to build `Service` values.
struct ServiceData {
    /// The tags, as in a Service.
    tags: Arc<SubCell<HashSet<Id<TagId>>>>,

    /// The id, as in a `Service`.
    id: Id<ServiceId>,

    /// Creation time properties.
    properties: HashMap<String, String>,

    features: HashMap<Id<FeatureId>, Arc<SubCell<FeatureData>>>,

    /// The adapter, as in `Service`.
    adapter: Id<AdapterId>,
}
impl ServiceData {
    /// Instantiate a `ServiceData` from a `Service`.
    ///
    /// # Warning
    ///
    /// Any `getters` or `setters` will be ignored!
    fn new(liveness: &Arc<Liveness>, mut service: Service) -> Self {
        ServiceData {
            tags: Arc::new(SubCell::new(liveness, service.tags.drain(..).collect())),
            id: service.id,
            adapter: service.adapter,
            properties: service.properties.drain(..).collect(),
            features: HashMap::new(),
        }
    }
    fn get_description(&self) -> ServiceDescription {
        ServiceDescription {
            tags: self.tags.borrow().clone(),
            id: self.id.clone(),
            properties: self.properties.clone(),
            adapter: self.adapter.clone(),
            features: self.features.iter().map(|(key, value)| {
                (key.clone(), value.borrow().get_description())
            }).collect(),
        }
    }

    pub fn matches(&self, selector: &ServiceSelector) -> bool
    {
        if !selector.id.matches(&self.id) {
            return false;
        }
        if !(TagMatch {needs: &selector.tags, has: &*self.tags.borrow()}).matches() {
            return false;
        }
        // If any of the feature selectors doesn't find a feature,
        // we don't match.
        let features_found = selector.features.iter().all(|selector| {
            for feature in self.features.values() {
                if feature.borrow().sub_matches(selector) {
                    return true;
                }
            }
            false
        });
        if !features_found {
            return false;
        }

        true
    }
}

/// Data and metadata on an adapter.
struct AdapterData {
    /// The implementation of the adapter.
    adapter: Arc<Adapter>,

    /// The services for this adapter.
    services: HashMap<Id<ServiceId>, Arc<SubCell<ServiceData>>>,
}

impl AdapterData {
    fn new(adapter: Arc<Adapter>) -> Self {
        AdapterData {
            adapter: adapter,
            services: HashMap::new(),
        }
    }
}
impl Deref for AdapterData {
    type Target = Arc<Adapter>;
    fn deref(&self) -> &Self::Target {
        &self.adapter
    }
}


/// A key used to uniquely represent a watcher.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct WatchKey(usize);


/// All the information needed to apply a watch to a newly registered (or
/// newly mapping) `Feature`. Instances of `WatcherData` are created by
/// calls to `register_watch` and deallocated upon drop of a `WatchGuard`.
pub struct WatcherData {
    /// The criteria for watching.
    watch: TargetMap<FeatureSelector, Exactly<Arc<AsValue>>>,

    /// The data needed to deserialize instances of `AsValue`.
    deserialization: Arc<DeserializeSupport>,

    /// The listener for this watch.
    on_event: Mutex<Box<ExtSender<WatchEvent<WatchEventDetails>>>>,

    /// A unique key used to locate the `WatcherData` in the
    /// `WatchMap`.
    key: WatchKey,

    /// The individual guard for each getter currently watched.
    guards: SubCell<HashMap<Id<FeatureId>, Vec<Box<AdapterWatchGuard>>>>,

    /// `true` once the `WatchGuard` has dropped. In this
    /// case, the `WatcherData` will shortly be removed
    /// from the `WatchMap`.
    is_dropped: Arc<AtomicBool>,
}
impl WatcherData {
    fn new(liveness: &Arc<Liveness>,
        key: WatchKey,
        watch: TargetMap<FeatureSelector, Exactly<Arc<AsValue>>>,
        on_event: Box<ExtSender<WatchEvent<WatchEventDetails>>>,
        deserialization: Arc<DeserializeSupport>) -> Self
    {
        WatcherData {
            key: key,
            watch: watch,
            deserialization: deserialization,
            on_event: Mutex::new(on_event),
            guards: SubCell::new(liveness, HashMap::new()),
            is_dropped: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl Hash for WatcherData {
     fn hash<H>(&self, state: &mut H) where H: Hasher {
         self.key.hash(state)
     }
}
impl Eq for WatcherData {}
impl PartialEq for WatcherData {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl WatcherData {
    fn push_guard(&self, id: Id<FeatureId>, guard: Box<AdapterWatchGuard>) {
        match self.guards.borrow_mut().entry(id) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(guard);
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![guard]);
            }
        }
    }
}

pub struct WatchMap {
    /// A counter of all watchers that have been added to the system.
    /// Used to generate unique keys.
    counter: usize,
    watchers: HashMap<WatchKey, Arc<WatcherData>>,
    liveness: Arc<Liveness>,
}
impl WatchMap {
    fn new(liveness: &Arc<Liveness>) -> Self {
        WatchMap {
            counter: 0,
            watchers: HashMap::new(),
            liveness: liveness.clone()
        }
    }
    fn create(&mut self,
        watch: TargetMap<FeatureSelector, Exactly<Arc<AsValue>>>,
        on_event: Box<ExtSender<WatchEvent<WatchEventDetails>>>,
        deserialization: Arc<DeserializeSupport>) -> Arc<WatcherData> {
        let id = WatchKey(self.counter);
        self.counter += 1;
        let watcher = Arc::new(WatcherData::new(&self.liveness, id, watch, on_event, deserialization));
        self.watchers.insert(id, watcher.clone());
        watcher
    }
    fn remove(&mut self, key: WatchKey) -> Option<Arc<WatcherData>> {
        self.watchers.remove(&key)
    }
}




pub struct State {
    /// Adapters, indexed by their id.
    adapter_by_id: HashMap<Id<AdapterId>, AdapterData>,

    /// Services, indexed by their id.
    service_by_id: HashMap<Id<ServiceId>, Arc<SubCell<ServiceData>>>,

    /// Features, indexed by their id.
    feature_by_id: HashMap<Id<FeatureId>, Arc<SubCell<FeatureData>>>,

    /// The set of watchers registered. Used both when we add/remove channels
    /// and a when a new value is available from a getter channel.
    watchers: Arc<Mutex<WatchMap>>,

    /// Information on whether the lock holding the state is open/closed,
    /// mutable/immutable.
    liveness: Arc<Liveness>,

    /// The path to the database used to persist tags.
    /// We don't keep track on the database itself since it won't see high load:
    /// - We read all tags once per lifetime of the manager.
    /// - We write occasionaly when adding or removing tags.
    db_path: Option<PathBuf>,
}

impl State {
    /// Auxiliary function to remove a service, once the mutex has been acquired.
    /// Clients should rather use AdapterManager::remove_service.
    fn aux_remove_service(&mut self, id: &Id<ServiceId>) -> Result<Id<AdapterId>, Error> {
        let (adapter, service) = match self.service_by_id.remove(&id) {
            None => return Err(Error::InternalError(InternalError::NoSuchService(id.clone()))),
            Some(service) => {
                let adapter = service.borrow().adapter.clone();
                (adapter, service)
            }
        };
        for id in service.borrow().features.keys() {
            let _ignored = self.feature_by_id.remove(id);
        }
        Ok(adapter)
    }

    fn get_services_map(&self, selectors: &[ServiceSelector]) -> HashMap<Id<ServiceId>, Arc<SubCell<ServiceData>>> {
        let mut result = HashMap::new();
        for (id, service) in &self.service_by_id {
            // Ensure that we release the borrow before calling `cb`.
            let fails;
            {
                if selectors.is_empty() {
                    fails = false;
                } else {
                    let borrow = &*service.borrow();
                    fails = selectors.iter().all(|selector| {
                        !borrow.matches(selector)
                    });
                }
            }
            if !fails {
                result.insert(id.clone(), service.clone());
            }
        };
        result
    }

    fn get_features_map(&self, selectors: &[FeatureSelector]) -> HashMap<Id<FeatureId>, Arc<SubCell<FeatureData>>> {
        // FIXME: This is very suboptimal.
        let mut result : HashMap<Id<_>, _> = HashMap::new();
        for service in self.service_by_id.values() {
            let service = service.borrow();
            for (id, feature) in &service.features {
                let fail = !selectors.is_empty() && selectors.iter().all(|feature_selector| {
                    !feature.borrow().matches(&service, feature_selector)
                });
                if !fail {
                    result.insert(id.clone(), feature.clone());
                }
            }
        }
        result
    }


    pub fn get_features(&self, selectors: &[FeatureSelector]) -> Vec<FeatureDescription>
    {
        self.get_features_map(selectors)
            .values()
            .map(|feature| feature.borrow().get_description())
            .collect()
    }

    fn aux_feature_may_need_unregistration(data: &mut FeatureData, is_being_removed: bool) {
        let mut keys_to_drop = vec![];
        {
            for (key, ref watcher) in &data.watchers {
                let watcher = match watcher.upgrade() {
                    Some(watcher) => watcher,
                    None => {
                        // The watcher has already been removed.
                        keys_to_drop.push(*key);
                        continue;
                    }
                };

                let service = match data.weak_service.upgrade() {
                    None => {
                        log_debug_assert!(false, "[backend@taxonomy] Cannot find the service for a feature {:?}", data.id);
                        continue
                    }
                    Some(service) => service
                };
                let service = service.borrow();

                // We need to disconnect the watcher if either the channel is being removed
                // or it doesn't match anymore any of the selectors for the watchers
                // that were watching it.
                let should_disconnect = is_being_removed
                    || watcher.watch.iter().any(|targetted| {
                        targetted.select.iter().any(|selector| {
                            !data.matches(&*service, selector)
                        })
                    });
                if !should_disconnect {
                    // The channel hasn't stopped matching this watcher.
                    continue;
                }

                // Inform of topology change
                let on_event = &watcher.on_event;
                let _ = on_event.lock()
                    .unwrap()
                    .send(WatchEvent::FeatureRemoved {
                        id: data.id.clone(),
                        connection: is_being_removed
                    });

                // Drop individual guard.
                watcher.guards.borrow_mut().remove(&data.id);
                keys_to_drop.push(*key);
            }
        }
        for key in keys_to_drop {
            data.watchers.remove(&key);
        }
    }

    fn aux_features_may_need_registration(&mut self, features: Vec<Id<FeatureId>>, is_being_added: bool) -> WatchRequest {
        let adapter_by_id = &self.adapter_by_id;
        let mut per_adapter = HashMap::new();
        for id in features {
            let mut data = match self.feature_by_id.get_mut(&id) {
                None => {
                    log_debug_assert!(false, "[backend@taxonomy] I have just added/modified feature {:?} but I can't \
                                            find it anymore", id);
                    continue
                },
                Some(data) => data
            }.borrow_mut();

            let service = match data.weak_service.upgrade() {
                None => {
                    log_debug_assert!(false, "[backend@taxonomy] Cannot find the service for a feature {:?}", id);
                    continue
                }
                Some(service) => service
            };
            let service = service.borrow();

            // Determine if the feature matches an already registered watcher.
            for watcher in &mut self.watchers.lock().unwrap().watchers.values() {
                if watcher.guards.borrow().contains_key(&id) {
                    // The watcher already matches this getter.
                    continue;
                }
                for targetted in &watcher.watch {
                    let matches = targetted.select.iter().any(|selector| {
                        data.matches(&*service, selector)
                    });
                    if !matches {
                        // The feature doesn't match this watcher.
                        continue;
                    }

                    // From now on, inform of topology changes.
                    let on_event = &watcher.on_event;
                    let _ = on_event.lock().unwrap().send(WatchEvent::FeatureAdded {
                        id: id.clone(),
                        connection: is_being_added,
                    });

                    // Register to be informed of future changes.
                    Self::aux_prepare_watch_features(&mut watcher.clone(),
                        &mut *data, &targetted.payload, adapter_by_id, &mut per_adapter)
                }
            }
        }

        per_adapter
    }
}

impl State {
    pub fn new(liveness: &Arc<Liveness>, db_path: Option<PathBuf>) -> Self {
        State {
            liveness: liveness.clone(),
            adapter_by_id: HashMap::new(),
            service_by_id: HashMap::new(),
            feature_by_id: HashMap::new(),
            watchers: Arc::new(Mutex::new(WatchMap::new(liveness))),
            db_path: db_path,
       }
    }

    /// Add an adapter to the system.
    ///
    /// # Errors
    ///
    /// Returns an error if an adapter with the same id is already present.
    pub fn add_adapter(&mut self, adapter: Arc<Adapter>) -> Result<(), Error> {
        match self.adapter_by_id.entry(adapter.id()) {
            Entry::Occupied(_) => return Err(Error::InternalError(InternalError::DuplicateAdapter(adapter.id()))),
            Entry::Vacant(entry) => {
                entry.insert(AdapterData::new(adapter));
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
    pub fn remove_adapter(&mut self, id: &Id<AdapterId>) -> Result<(), Error> {
        let mut services = match self.adapter_by_id.remove(id) {
            Some(AdapterData {services: adapter_services, ..}) => {
                adapter_services
            }
            None => return Err(Error::InternalError(InternalError::NoSuchAdapter(id.clone()))),
        };
        for (service_id, _) in services.drain() {
            let _ignored = self.aux_remove_service(&service_id);
        }
        Ok(())
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
    pub fn add_service(&mut self, service: Service) -> Result<(), Error> {
        // Make sure that there are no channels.
        let service = ServiceData::new(&self.liveness, service);
        let mut services_for_this_adapter =
            match self.adapter_by_id.get_mut(&service.adapter) {
                None => return Err(Error::InternalError(InternalError::NoSuchAdapter(service.adapter.clone()))),
                Some(&mut AdapterData {ref mut services, ..}) => {
                    services
                }
            };
        let id = service.id.clone();


        // Synchronize the tags with the database.
        {
            if let Some(ref path) = self.db_path {
                // Update the service's tag set with the full set from the database.
                let mut store = TagStorage::new(&path);
                let tags = match store.get_tags_for(&id) {
                    Err(err) => return Err(Error::InternalError(InternalError::GenericError(format!("{}", err)))),
                    Ok(tags) => tags
                };

                let mut tag_set = service.tags.borrow_mut();
                for tag in &tags {
                    let _ = tag_set.insert(tag.clone());
                }
            }
        }

        let service = Arc::new(SubCell::new(&self.liveness, service));
        let insert_in_adapters =
            match InsertInMap::start(&mut services_for_this_adapter, vec![(id.clone(), service.clone())]) {
                Err(k) => return Err(Error::InternalError(InternalError::DuplicateService(k))),
                Ok(transaction) => transaction
            };

        let insert_in_services =
            match InsertInMap::start(&mut self.service_by_id, vec![(id.clone(), service)]) {
                Err(k) => return Err(Error::InternalError(InternalError::DuplicateService(k))),
                Ok(transaction) => transaction
            };

        // If we haven't bailed out yet, leave all this stuff in the maps and sets.
        insert_in_adapters.commit();
        insert_in_services.commit();
        Ok(())
    }

    /// Remove a service previously registered on the system. Typically, called by
    /// an adapter when a service (e.g. a device) is disconnected.
    ///
    /// # Errors
    ///
    /// Returns an error if any of:
    /// - there is no such service;
    /// - there is an internal inconsistency, in which case this method will still attempt to
    /// cleanup before returning an error.
    pub fn remove_service(&mut self, service_id: &Id<ServiceId>) -> Result<(), Error> {
        let adapter = try!(self.aux_remove_service(service_id));
        match self.adapter_by_id.get_mut(&adapter) {
            None => Err(Error::InternalError(InternalError::NoSuchAdapter(adapter.clone()))),
            Some(mut data) => {
                if data.services.remove(service_id).is_none() {
                    Err(Error::InternalError(InternalError::NoSuchService(service_id.clone())))
                } else {
                    Ok(())
                }
            }
        }
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
    /// registered, or a `Feature` with the same identifier is already registered.
    /// In either cases, this method reverts all its changes.
    pub fn add_feature(&mut self, feature: Feature) -> Result<WatchRequest, Error>
    {
        let id = feature.id.clone();
        {
            let feature_by_id = &mut self.feature_by_id;
            let service = match self.service_by_id.get_mut(&feature.service) {
                None => return Err(Error::InternalError(InternalError::NoSuchService(feature.service.clone()))),
                Some(service) => service
            };
            let weak_service = Arc::downgrade(&service);
            let mut service = &mut *service.borrow_mut();
            let features = &mut service.features;
            let feature = FeatureData::new(&self.liveness, weak_service, &service.adapter, feature);

            {
                // Add the database tags to this feature.
                let mut tags = feature.tags.borrow_mut();
                if let Some(ref path) = self.db_path {
                    let mut store = TagStorage::new(&path);

                    // Add all the tags for this feature.
                    if let Ok(all_tags) = store.get_tags_for(&id) {
                        for tag in all_tags {
                            tags.insert(tag);
                        }
                    }
                }
            }

            let data = Arc::new(SubCell::new(&self.liveness, feature));

            let insert_in_service = match InsertInMap::start(features, vec![(id.clone(), data.clone())]) {
                Ok(transaction) => transaction,
                Err(id) => return Err(Error::InternalError(InternalError::DuplicateFeature(id)))
            };

            let insert_in_features = match InsertInMap::start(feature_by_id, vec![(id.clone(), data)]) {
                Ok(transaction) => transaction,
                Err(id) => return Err(Error::InternalError(InternalError::DuplicateFeature(id)))
            };

            insert_in_service.commit();
            insert_in_features.commit();
        }

        Ok(self.aux_features_may_need_registration(vec![id], true))
    }



    /// Remove a setter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its setters.
    ///
    /// # Error
    ///
    /// This method returns an error if the setter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    pub fn remove_feature(&mut self, id: &Id<FeatureId>) -> Result<(), Error> {
        let feature = match self.feature_by_id.remove(id) {
            None => return Err(Error::InternalError(InternalError::NoSuchFeature(id.clone()))),
            Some(feature) => feature
        };
        Self::aux_feature_may_need_unregistration(&mut *feature.borrow_mut(), true);

        let service_id = &feature.borrow().service;
        match self.service_by_id.get_mut(&service_id) {
            None => Err(Error::InternalError(InternalError::NoSuchService(service_id.clone()))),
            Some(service) => {
                if service.borrow_mut().features.remove(id).is_none() {
                    Err(Error::InternalError(InternalError::NoSuchFeature(id.clone())))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn get_services(&self, selectors: Vec<ServiceSelector>) -> Vec<ServiceDescription> {
        // This implementation is not nearly optimal, but it should be sufficient in a system
        // with relatively few services.
        self.get_services_map(&selectors)
            .values()
            .map(|service_data| service_data.borrow().get_description())
            .collect()
    }

    pub fn add_service_tags(&mut self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> (WatchRequest, usize) {
        let mut store = match self.db_path {
            // Even if we have a path, TagStorage opens the underlying database lazily,
            // so this is cheap.
            Some(ref path) => Some(TagStorage::new(&path)),
            None => None
        };

        let mut size = 0;
        let mut request : WatchRequest = HashMap::new();

        for service in self.get_services_map(&selectors).values() {
            size += 1;

            let mut changed = false;
            let features;
            {
                // Make sure that we release the borrows before proceeding.
                let service = service.borrow_mut();
                let mut tag_set = service.tags.borrow_mut();
                for tag in &tags {
                    if tag_set.insert(tag.clone()) {
                        changed = true;
                    }
                }

                if changed {
                    features = service.features.keys().cloned().collect();
                    if let Some(ref mut storage) = store {
                        storage.add_tags(&service.id, &tags)
                            .unwrap_or_else(|err| { error!("Storage add_tags error: {}", err); });
                    }
                } else {
                    continue;
                }
            }

            // At this stage, changed is true.
            let mut aux_request = self.aux_features_may_need_registration(features, false);
            for (id, payload) in aux_request.drain() {
                request.insert(id, payload);
            }
        };
        (request, size)
    }

    pub fn remove_service_tags(&mut self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize {
        let mut store = match self.db_path {
            // Even if we have a path, TagStorage opens the underlying database lazily,
            // so this is cheap.
            Some(ref path) => Some(TagStorage::new(&path)),
            None => None
        };

        let mut size = 0;
        for service in self.get_services_map(&selectors).values() {
            size += 1;
            let mut changed = false;
            let features : Vec<_>;
            {
                // Make sure that we release the borrow as soon as we can.
                let service = service.borrow_mut();
                let mut tag_set = service.tags.borrow_mut();
                for tag in &tags {
                    if tag_set.remove(&tag) {
                        changed = true;
                    }
                }

                if changed {
                    features = service.features.values().cloned().collect();
                    if let Some(ref mut storage) = store {
                        storage.remove_tags(&service.id, &tags)
                            .unwrap_or_else(|err| { error!("Storage add_tags error: {}", err); });
                    }
                } else {
                    continue;
                }
            }


            // At this stage, changed is true
            for feature in features {
                Self::aux_feature_may_need_unregistration(&mut*feature.borrow_mut(), false);
            }
        };
        size
    }

    pub fn add_feature_tags(&mut self, selectors: Vec<FeatureSelector>, tags: Vec<Id<TagId>>) -> (WatchRequest, usize) {
        let mut store = match self.db_path {
            // Even if we have a path, TagStorage opens the underlying database lazily,
            // so this is cheap.
            Some(ref path) => Some(TagStorage::new(&path)),
            None => None
        };

        let mut size = 0;
        let mut features = vec![];
        {
            for (id, data) in self.get_features_map(&selectors) {
                size += 1;
                let mut changed = false;
                let data = data.borrow();
                let mut tag_set = data.tags.borrow_mut();
                for tag in &tags {
                    if tag_set.insert(tag.clone()) {
                        changed = true;
                    }
                }
                if changed {
                    if let Some(ref mut storage) = store {
                        storage.add_tags(&id, &tags)
                            .unwrap_or_else(|err| { error!("Storage add_tags error: {}", err); });
                    }
                    features.push(id);
                }
            }
        }
        (self.aux_features_may_need_registration(features, false), size)
    }

    pub fn remove_feature_tags(&mut self, selectors: Vec<FeatureSelector>, tags: Vec<Id<TagId>>) -> usize {
        let mut store = match self.db_path {
            // Even if we have a path, TagStorage opens the underlying database lazily,
            // so this is cheap.
            Some(ref path) => Some(TagStorage::new(&path)),
            None => None
        };

        let mut result = 0;
        for (id, data) in self.get_features_map(&selectors) {
            let mut changed = false;
            {
                let data = data.borrow();
                let mut tag_set = data.tags.borrow_mut();
                for tag in &tags {
                    if tag_set.remove(tag) {
                        changed = true;
                    }
                }
            }
            if changed {
                Self::aux_feature_may_need_unregistration(&mut*data.borrow_mut(), false);
                if let Some(ref mut storage) = store {
                    storage.remove_tags(&id, &tags)
                        .unwrap_or_else(|err| { error!("Storage add_tags error: {}", err); });
                }
            }
            result += 1;
        }
        result
    }

    pub fn prepare_method_request<T>(&self, method: MethodCall, mut target: TargetMap<FeatureSelector, Option<T>>) -> MethodRequest<T>
        where T: Clone
    {
        // First determine the channels and group them by adapter.
        let mut per_adapter = HashMap::new();
        for Targetted {select: selectors, payload: maybe_value} in target.drain(..) {
            for (id, data) in self.get_features_map(&selectors) {
                use std::collections::hash_map::Entry::*;
                let data = data.borrow();
                if let Some(ref signature) = *data.get_method(method) {
                    let details = RequestDetail {
                        feature: id,
                        signature: signature.clone(),
                        payload: maybe_value.clone()
                    };
                    match per_adapter.entry(data.adapter.clone()) {
                        Vacant(entry) => {
                            let adapter = match self.adapter_by_id.get(&data.adapter) {
                                None => {
                                    log_debug_assert!(false, "[backend@taxonomy] Internal inconsistency: could not find adapter {}", data.adapter);
                                    continue
                                }
                                Some(adapter) => adapter
                            };
                            entry.insert((adapter.adapter.clone(), vec![details]));
                        }
                        Occupied(mut entry) => {
                            let &mut (_, ref mut items) = entry.get_mut();
                            items.push(details);
                        }
                    }
                }
            }
        }
        per_adapter
    }

    fn aux_prepare_watch_features(watcher: &mut Arc<WatcherData>,
        data: &mut FeatureData,
        filter: &Exactly<Arc<AsValue>>,
        adapter_by_id: &HashMap<Id<AdapterId>, AdapterData>,
        per_adapter: &mut WatchRequest)
    {
        use std::collections::hash_map::Entry::*;

        let id = data.id.clone();
        let adapter = data.adapter.clone();

        let insert_in_getter =
            match InsertInMap::start(&mut data.watchers, vec![ ( watcher.key, Arc::downgrade(watcher) )] ) {
            Err(_) => {
                log_debug_assert!(false, "[backend@taxonomy] Internal inconsistency: This watcher is already watching this getter.");
                return
            }
            Ok(transaction) => transaction
        };

        let condition = match *filter {
            Exactly::Exactly(ref value) => Some(value.clone()),
            Exactly::Always => None,
            _ => {
                insert_in_getter.commit();
                return // Don't watch data, just topology.
            }
        };

        let signature = match data.watch {
            None => {
                insert_in_getter.commit();
                return // Don't watch data, just topology.
            },
            Some(ref signature) => signature.clone()
        };

        let detail = RequestDetail {
            feature: id,
            signature: signature,
            payload: (condition, Arc::downgrade(watcher))
        };

        match per_adapter.entry(adapter) {
            Vacant(entry) => {
                let adapter = match adapter_by_id.get(&data.adapter) {
                    None => {
                        log_debug_assert!(false, "[backend@taxonomy] Internal inconsistency: Could not find adapter {:?}",
                            data.adapter);
                        return;
                    },
                    Some(ref adapter_data) => {
                        adapter_data.adapter.clone()
                    }
                };
                entry.insert((adapter, vec![detail]));
            },
            Occupied(mut entry) => {
                let (_, ref mut batch) = *entry.get_mut();
                batch.push(detail)
            }
        }

        insert_in_getter.commit();
    }

    pub fn prepare_watch_features(&mut self,
        mut watch: TargetMap<FeatureSelector, Exactly<Arc<AsValue>>>,
        on_event: Box<ExtSender<WatchEvent<WatchEventDetails>>>,
        deserialization: Arc<DeserializeSupport>) -> (WatchRequest, WatchKey, Arc<AtomicBool>)
    {
        // Prepare the watcher and store it. Once we leave the lock, every time a channel is
        // added/removed/updated, this will cause us to reexamine whether the channel should
        // be visible to a watcher.
        let mut watcher = self.watchers.lock().unwrap().create(watch.clone(), on_event.clone(), deserialization);
        let is_dropped = watcher.is_dropped.clone();

        // Regroup per adapter.
        let mut per_adapter = HashMap::new();
        let adapter_by_id = &self.adapter_by_id;
        for Targetted { select: selectors, payload: filter } in watch.drain(..) {
            // Find out which channels already match the selectors and attach
            // the watcher immediately.
            let filter = &filter;
            for data in self.get_features_map(&selectors).values() {
                Self::aux_prepare_watch_features(&mut watcher, &mut *data.borrow_mut(), filter,
                    adapter_by_id, &mut per_adapter)
            }
        }

        // Upon drop, this data structure will immediately drop `is_dropped` and then dispatch
        // `unregister_channel_watch` to unregister everything else.
        (per_adapter, watcher.key, is_dropped)
    }

    /// Unregister a watch previously registered with `register_channel_watch`.
    ///
    /// This method is dispatched from `WatchGuard::drop()`.
    pub fn stop_watch(&mut self, key: WatchKey) {
        // Note: no matter when we arrive here, `is_dropped` is already set to `true`.

        // Remove `key` from `watchers`. This will prevent the watcher from being registered
        // automatically with any new getter.
        let watcher_data = match self.watchers.lock().unwrap().remove(key) {
            None => {
                // Attempting to unregister a watcher that has not been added yet.
                // This can happen in case of race if `stop_watch` is executed before
                // `start_watch`. Since `is_dropped` is `true`, `start_watch` will be
                // a noop for this watcher, so we're good.
                return
            }
            Some(watcher_data) => watcher_data
        };

        log_debug_assert!(watcher_data.is_dropped.load(Ordering::Relaxed), "The watcher should have been dropped by now.");

        // Remove the watcher from all getters.
        for feature_id in watcher_data.guards.borrow().keys() {
            let getter = match self.feature_by_id.get_mut(feature_id) {
                None => continue, // Race condition between removing the getter and dropping the watcher.
                Some(getter) => getter
            };
            if getter.borrow_mut().watchers.remove(&watcher_data.key).is_none() {
                debug_assert!(false, "Attempting to unregister a watcher that has already been removed from its getter {:?}, {:?}", key, feature_id);
            }
        }

        // At this stage, one getter may still have a strong reference to watcher_data, if it has
        // just been upgraded in `start_watch`. However, this reference will disappear soon. Once
        // the last reference has disappeared, all `guards` will be dropped.
    }

    /// Start watching a set of channels.
    pub fn start_watch(mut per_adapter: WatchRequest) -> WatchGuardCommit {
        // In most cases, stop_watch will take place long after start_watch. It is, however,
        // possible that the WatchGuard is dropped before start_watch is processed for this
        // channel. In this case, three events take place:
        // 1. The WatchGuard sets `is_dropped` to `true`, atomically.
        // 2a. The WatchGuard dispatches `stop_watch`, which is serialized.
        // 2b. Someone dispatches `start_watch`, which is serialized.
        //
        // Since `start_watch` and `stop_watch` are serialized, either 2a or 2b will win.
        //
        // A/ If 2a takes place before 2b.
        //  - The call to `stop_watch` is a noop, as there is nothing to remove.
        //  - The call to `start_watch` is a noop, as `is_dropped` is `true.`
        //
        // B/ If 2b takes place before 2a.
        //  - The call to `start_watch` is a noop, as `is_dropped` is `true`.
        //  - The call to `stop_watch` is a noop, as there is nothing to remove.
        //   by checking whether `is_dropped` is true.

        let mut to_add = vec![];
        for (_, (adapter, mut batch)) in per_adapter.drain() {
            for RequestDetail {feature: id, payload: (condition, weak_watch_data), signature } in batch.drain(..) {
                let watch_data = match weak_watch_data.upgrade() {
                    None => {
                        // The `weak_watch_data` has already been dropped, nothing to do.
                        continue
                    }
                    Some(watch_data) => watch_data
                };
                let is_dropped = watch_data.is_dropped.clone();
                if is_dropped.load(Ordering::Relaxed) {
                    // The `WatchGuard` has already been dropped.
                    continue
                }
                let returns = match signature.returns {
                    Expects::Nothing => {
                        // FIXME: Does this really need a warning?
                        warn!("[backend@taxonomy] Attempting to start_watch on a service where `returns` is Nothing {}", id);
                        continue
                    },
                    Expects::Optional(format) | Expects::Requires(format) => format
                };
                let condition = match signature.accepts {
                    Expects::Nothing => {
                        // FIXME: Does this really need a warning?
                        warn!("[backend@taxonomy] Attempting to start_watch on a service where `accepts` is Nothing {}", id);
                        continue
                    }
                    Expects::Optional(format) | Expects::Requires(format) => {
                        match condition {
                            None => None,
                            Some(as_value) => {
                                match as_value.decode(&*format, &*watch_data.deserialization) {
                                    Ok(condition) => Some(condition),
                                    Err(err) => {
                                        warn!("[backend@taxonomy] While watching, feature {} attempted to return a value that does not match its format: {:?}", id, err);
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                };
                let on_ok = Box::new(watch_data.on_event.lock().unwrap().filter_map(move |event| {
                    if is_dropped.load(Ordering::Relaxed) {
                        // The WatchGuard has already been dropped.
                        // We want to stop propagating messages immediately, even if unregistration
                        // is not necessarily complete yet. Unregistration will be completed after
                        // the call to `stop_watch`.
                        return None;
                    }
                    Some(match event {
                        AdapterWatchEvent::Enter { id, value } => {
                            WatchEvent::EnterRange {
                                id: id,
                                value: WatchEventDetails {
                                    value: value,
                                    format: returns.clone()
                                }
                            }
                        },
                        AdapterWatchEvent::Exit { id, value } => {
                            WatchEvent::ExitRange {
                                id: id,
                                value: WatchEventDetails {
                                    value: value,
                                    format: returns.clone()
                                }
                            }
                        }
                    })
                }));

                let mut guards = vec![];
                for (id, result) in adapter.register_watch(vec![(id.clone(), (condition, on_ok))]) {
                    match result {
                        Err(err) => {
                            let event = WatchEvent::Error {
                                id: id.clone(),
                                error: err
                            };
                            let _ = watch_data.on_event.lock().unwrap().send(event);
                        },
                        // Calling `watch_data.push((id, guard))` requires .write(), so we delay
                        // this until we have grabbed the lock again.
                        Ok(guard) => guards.push((id, guard))
                    }
                }
                to_add.push((weak_watch_data, guards));
            }
        }
        to_add
    }

    /// Register a bunch of ongoing watches previously started by `start_watch`.
    pub fn register_ongoing_watch(&mut self, mut ongoing: WatchGuardCommit)
    {
        for (watch_data, mut guards) in ongoing.drain(..) {
            if let Some(ref watch_data) = watch_data.upgrade() {
                for (id, guard) in guards.drain(..) {
                    watch_data.push_guard(id, guard)
                }
            }
        }
    }
}


impl State {
    // Clear all state, removing any remaining cycle or lingering thread.
    pub fn stop(&mut self) {
        for adapter in self.adapter_by_id.values() {
            adapter.stop();
        }
        self.adapter_by_id.clear();
        self.service_by_id.clear();
        self.feature_by_id.clear();
        self.watchers.lock().unwrap().watchers.clear();
    }
}

struct TagMatch<'a> {
    needs: &'a HashSet<Id<TagId>>,
    has: &'a HashSet<Id<TagId>>
}
impl<'a> TagMatch<'a> {
    fn matches(&self) -> bool {
        for tag in &*self.needs {
            if !self.has.contains(tag) {
                return false;
            }
        }
        true
    }
}
