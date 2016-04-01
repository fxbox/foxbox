//! An API for plugging in adapters.

use adapter::{ Adapter, AdapterWatchGuard, ResultMap, WatchEvent as AdapterWatchEvent };
use transact::InsertInMap;

use api::{ Error, InternalError, TargetMap, Targetted, WatchEvent };
use selector::*;
use services::*;
use values::*;

use sublock::atomlock::*;
use transformable_channels::mpsc::*;

use std::collections::{ HashMap, HashSet };
use std::collections::hash_map::Entry;
use std::hash::{ Hash, Hasher };
use std::ops::{ Deref };
use std::sync::{ Arc, Mutex, Weak };
use std::sync::atomic::{ AtomicBool, Ordering };

/// A request to a bunch of adapters.
///
/// Whenever possible, the AdapterManager attempts to place calls to the Adapters
/// after it has released its locks. An `AdapterRequest` represents stuff that has
/// been extracted from the maps while they were locked for use after unlocking.
pub type AdapterRequest<T> = HashMap<Id<AdapterId>, (Arc<Adapter>, T)>;

/// A request to an adapter, for performing a `fetch` operation.
pub type FetchRequest = AdapterRequest<HashMap<Id<Getter>, Type>>;

/// A request to an adapter, for performing a `send` operation.
pub type SendRequest = AdapterRequest<(HashMap<Id<Setter>, Value>, ResultMap<Id<Setter>, (), Error>)>;

/// A request to an adapter, for performing a `watch` operation.
pub type WatchRequest = AdapterRequest<(Vec<(Id<Getter>, Option<Range>)>, Weak<WatcherData>)>;

pub type WatchGuardCommit = Vec<(Arc<WatcherData>, Vec<(Id<Getter>, Box<AdapterWatchGuard>)>)>;

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

    /// Information on the getters. Used to build field `getters` of service.
    getters: HashMap<Id<Getter>, Arc<SubCell<GetterData>>>,

    /// Information on the setters. Used to build field `getters` of service.
    setters: HashMap<Id<Setter>, Arc<SubCell<SetterData>>>,

    /// The adapter, as in `Service`.
    adapter: Id<AdapterId>,
}
impl ServiceData {
    /// Instantiate a `ServiceData` from a `Service`.
    ///
    /// # Warning
    ///
    /// Any `getters` or `setters` will be ignored!
    fn new(liveness: &Arc<Liveness>, service: Service) -> Self {
        ServiceData {
            tags: Arc::new(SubCell::new(liveness, service.tags)),
            id: service.id,
            adapter: service.adapter,
            properties: service.properties,
            getters: HashMap::new(),
            setters: HashMap::new(),
        }
    }
    fn as_service(&self) -> Service {
        Service {
            tags: self.tags.borrow().clone(),
            id: self.id.clone(),
            properties: self.properties.clone(),
            adapter: self.adapter.clone(),
            getters: self.getters.iter().map(|(key, value)| {
                (key.clone(), (**value).borrow().channel.clone())
            }).collect(),
            setters: self.setters.iter().map(|(key, value)| {
                (key.clone(), (**value).borrow().channel.clone())
            }).collect()
        }
    }
}

struct ServiceView<'a> where 'a {
    data: &'a ServiceData,
}
impl<'a> ServiceView<'a> {
    fn new(data: &'a ServiceData) -> Self {
        ServiceView {
            data: data,
        }
    }
}
impl<'a> ServiceLike for ServiceView<'a> {
    fn id(&self) -> &Id<ServiceId> {
        &self.data.id
    }
    fn adapter(&self) -> &Id<AdapterId> {
        &self.data.adapter
    }
    fn with_tags<F>(&self, f: F) -> bool where F: Fn(&HashSet<Id<TagId>>) -> bool {
        f(&*self.data.tags.borrow())
    }
    fn has_getters<F>(&self, f: F) -> bool where F: Fn(&Channel<Getter>) -> bool {
        for chan in self.data.getters.values() {
            if f(&*chan.borrow()) {
                return true;
            }
        }
        false
    }
    fn has_setters<F>(&self, f: F) -> bool where F: Fn(&Channel<Setter>) -> bool {
        for chan in self.data.setters.values() {
            if f(&*chan.borrow()) {
                return true;
            }
        }
        false
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

trait Tagged {
    fn insert_tags(&mut self, tags: &[Id<TagId>]) -> bool;
    fn remove_tags(&mut self, tags: &[Id<TagId>]) -> bool;
}

impl<T> Tagged for Channel<T> where T: IOMechanism {
    fn insert_tags(&mut self, tags: &[Id<TagId>]) -> bool {
        let mut has_changed = false;
        for tag in tags {
            if self.tags.insert(tag.clone()) {
                has_changed = true;
            }
        }
        has_changed
    }
    fn remove_tags(&mut self, tags: &[Id<TagId>]) -> bool {
        let mut has_changed = false;
        for tag in tags {
            if self.tags.remove(&tag) {
                has_changed = true;
            }
        }
        has_changed
    }
}

/// A key used to uniquely represent a watcher.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct WatchKey(usize);

/// Data and metadata on a getter.
struct GetterData {
    /// The getter itself.
    channel: Channel<Getter>,

    /// The tags of the service.
    service_tags: Arc<SubCell<HashSet<Id<TagId>>>>,

    /// Watchers that currently watch this channel.
    watchers: HashMap<WatchKey, Weak<WatcherData>>,
}
impl SelectedBy<GetterSelector> for GetterData {
    fn matches(&self, selector: &GetterSelector) -> bool {
        selector.matches(&*self.service_tags.borrow(), &self.channel)
    }
}

impl GetterData {
    fn new(channel: Channel<Getter>, service_tags: Arc<SubCell<HashSet<Id<TagId>>>>) -> Self {
        GetterData {
            channel: channel,
            service_tags: service_tags.clone(),
            watchers: HashMap::new(),
        }
    }
}

impl Deref for GetterData {
    type Target = Channel<Getter>;
    fn deref(&self) -> &Self::Target {
        &self.channel
    }
}
impl Tagged for GetterData {
    fn insert_tags(&mut self, tags: &[Id<TagId>]) -> bool {
        self.channel.insert_tags(tags)
    }
    fn remove_tags(&mut self, tags: &[Id<TagId>]) -> bool {
        self.channel.remove_tags(tags)
    }
}

/// Data and metadata on a setter.
struct SetterData {
    channel: Channel<Setter>,
    service_tags: Arc<SubCell<HashSet<Id<TagId>>>>,
}

impl SelectedBy<SetterSelector> for SetterData {
    fn matches(&self, selector: &SetterSelector) -> bool {
        selector.matches(&*self.service_tags.borrow(), &self.channel)
    }
}

impl SetterData {
    fn new(channel: Channel<Setter>, service_tags: Arc<SubCell<HashSet<Id<TagId>>>>) -> Self {
        SetterData {
            channel: channel,
            service_tags: service_tags.clone(),
        }
    }
}
impl Tagged for SetterData {
    fn insert_tags(&mut self, tags: &[Id<TagId>]) -> bool {
        self.channel.insert_tags(tags)
    }
    fn remove_tags(&mut self, tags: &[Id<TagId>]) -> bool {
        self.channel.remove_tags(tags)
    }
}
impl Deref for SetterData {
    type Target = Channel<Setter>;
    fn deref(&self) -> &Self::Target {
        &self.channel
    }
}

/// All the information on a currently registered watch.
///
/// A single watch may concern any number of getter channels, including channels not registered
/// yet. The WatcherData is materialized as a WatchGuard in userland.
pub struct WatcherData {
    /// The criteria for watching.
    watch: TargetMap<GetterSelector, Exactly<Range>>,

    /// The listener for this watch.
    on_event: Mutex<Box<ExtSender<WatchEvent>>>,

    /// A unique key used to locate the WatcherData in the
    /// WatchMap.
    key: WatchKey,

    /// The individual guard for each getter currently watched.
    guards: SubCell<HashMap<Id<Getter>, Vec<Box<AdapterWatchGuard>>>>,

    /// `true` once the WatchGuard has dropped. In this
    /// case, the `WatcherData` will shortly be removed
    /// from the WatchMap.
    is_dropped: Arc<AtomicBool>,
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
    fn new(liveness: &Arc<Liveness>, key: WatchKey, watch:TargetMap<GetterSelector, Exactly<Range>>, on_event: Box<ExtSender<WatchEvent>>) -> Self {
        WatcherData {
            key: key,
            on_event: Mutex::new(on_event),
            watch: watch,
            is_dropped: Arc::new(AtomicBool::new(false)),
            guards: SubCell::new(liveness, HashMap::new()),
        }
    }

    fn push_guard(&self, id: Id<Getter>, guard: Box<AdapterWatchGuard>) {
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
    fn create(&mut self, watch:TargetMap<GetterSelector, Exactly<Range>>, on_event: Box<ExtSender<WatchEvent>>) -> Arc<WatcherData> {
        let id = WatchKey(self.counter);
        self.counter += 1;
        let watcher = Arc::new(WatcherData::new(&self.liveness, id, watch, on_event));
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

    /// Getters, indexed by their id.
    getter_by_id: HashMap<Id<Getter>, Arc<SubCell<GetterData>>>,

    /// Setters, indexed by their id
    setter_by_id: HashMap<Id<Setter>, Arc<SubCell<SetterData>>>,

    /// The set of watchers registered. Used both when we add/remove channels
    /// and a when a new value is available from a getter channel.
    watchers: Arc<Mutex<WatchMap>>,

    /// Information on whether the lock holding the state is open/closed,
    /// mutable/immutable.
    liveness: Arc<Liveness>,
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
        for id in service.borrow().getters.keys() {
            let _ignored = self.getter_by_id.remove(id);
        }
        for id in service.borrow().setters.keys() {
            let _ignored = self.setter_by_id.remove(id);
        }
        Ok(adapter)
    }

    fn with_services<F>(&self, selectors: Vec<ServiceSelector>, mut cb: F) where F: FnMut(&Arc<SubCell<ServiceData>>) {
        for service in self.service_by_id.values() {
            // All services match when we have no selectors.
            if selectors.is_empty() {
                cb(service);
                continue;
            }
            let matches;
            {
                // Ensure that we release the borrow before calling `cb`.
                let borrow = &*service.borrow();
                let view = ServiceView::new(borrow);
                matches = selectors.iter().find(|selector| {
                    selector.matches(&view)
                }).is_some();
            }
            if matches {
                cb(service);
            }
        };
    }

    /// Iterate over all channels that match any selector in a slice.
    fn with_channels<S, K, V, F>(selectors: Vec<S>, map: &HashMap<Id<K>, Arc<SubCell<V>>>, mut cb: F)
        where F: FnMut(&V),
              V: SelectedBy<S>,
    {
        for (_, data) in map.iter() {
            let matches = selectors.iter().find(|selector| {
                data.borrow().matches(selector)
            }).is_some();
            if matches {
                cb(&*data.borrow());
            }
        }
    }

    /// Iterate mutably over all channels that match any selector in a slice.
    fn with_channels_mut<S, K, V, F>(selectors: Vec<S>, map: &mut HashMap<Id<K>, Arc<SubCell<V>>>, mut cb: F)
        where F: FnMut(&mut V),
              V: SelectedBy<S>,
    {
        for (_, data) in map.iter_mut() {
            let matches = selectors.iter().find(|selector| {
                data.borrow().matches(selector)
            }).is_some();
            if matches {
                cb(&mut *data.borrow_mut());
            }
        }
    }

     /// Iterate over all channels that match any selector in a slice.
    fn aux_get_channels<S, K, V, T>(selectors: Vec<S>, map: &HashMap<Id<K>, Arc<SubCell<V>>>) -> Vec<Channel<T>>
        where V: SelectedBy<S> + Deref<Target = Channel<T>>,
              T: IOMechanism,
              Channel<T>: Clone
    {
        let mut result = Vec::new();
        Self::with_channels(selectors, map, |data| {
            result.push((*data.deref()).clone());
        });
        result
    }

    fn aux_getter_may_need_unregistration(getter_data: &mut GetterData, is_being_removed: bool) {
        let mut keys_to_drop = vec![];
        {
            for (key, ref watcher) in &getter_data.watchers {
                let watcher = match watcher.upgrade() {
                    Some(watcher) => watcher,
                    None => {
                        // The watcher has already been removed.
                        keys_to_drop.push(*key);
                        continue;
                    }
                };

                // We need to disconnect the watcher if either the channel is being removed
                // or it doesn't match anymore any of the selectors for the watchers
                // that were watching it.
                let should_disconnect = is_being_removed
                    || watcher.watch.iter().find(|&targetted| {
                        targetted.select.iter().find(|selector| {
                            !getter_data.matches(selector)
                        }).is_some()
                    }).is_some();
                if !should_disconnect {
                    // The channel hasn't stopped matching this watcher.
                    continue;
                }

                // Inform of topology change
                let on_event = &watcher.on_event;
                let _ = on_event.lock()
                    .unwrap()
                    .send(WatchEvent::GetterRemoved(getter_data.id.clone()));

                // Drop individual guard.
                watcher.guards.borrow_mut().remove(&getter_data.id);
                keys_to_drop.push(*key);
            }
        }
        for key in keys_to_drop {
            getter_data.watchers.remove(&key);
        }
    }

    fn aux_getters_may_need_registration(&mut self, getters: Vec<Id<Getter>>) -> WatchRequest {
        let adapter_by_id = &self.adapter_by_id;
        let mut per_adapter = HashMap::new();
        for id in getters {
            match self.getter_by_id.get_mut(&id) {
                None => {
                    debug_assert!(false, "I have just added/modified getter {:?} but I can't \
                                            find it anymore", id);
                    // FIXME: Logging would be nice.
                },
                Some(getter_data) => {
                    let mut getter_data = getter_data.borrow_mut();

                    // Determine if the channel matches an ongoing watcher.
                    for watcher in &mut self.watchers.lock().unwrap().watchers.values() {
                        if watcher.guards.borrow().contains_key(&id) {
                            // The watcher already matches this getter.
                            continue;
                        }
                        for targetted in &watcher.watch {
                            let matches = targetted.select.iter().find(|selector| {
                                getter_data.matches(selector)
                            }).is_some();
                            if !matches {
                                // The channel doesn't match this watcher.
                                continue;
                            }

                            // Inform of topology change.
                            let on_event = &watcher.on_event;
                            let _ = on_event.lock().unwrap().send(WatchEvent::GetterAdded(id.clone()));

                            // Register to be informed of future changes.
                            Self::aux_start_channel_watch(&mut watcher.clone(),
                                &mut *getter_data, &targetted.payload, adapter_by_id, &mut per_adapter)
                        }
                    }
                }
            }
        }

        per_adapter
    }

    /*
        fn iter_channels<S, K, V>(selectors: Vec<S>, map: &HashMap<Id<K>, V>) ->
            Filter<Values<Id<K>, V>, &(Fn(&V) -> bool)>
            where V: SelectedBy<S>
        {
            let cb : &Fn(&V) -> bool + 'state = |data: &V| {
                selectors.iter().find(|selector| {
                    data.matches(selector)
                }).is_some()
            };
            map.values()
                .filter(cb)
        }
    */

}

impl State {
    pub fn new(liveness: &Arc<Liveness>) -> Self {
        State {
            liveness: liveness.clone(),
            adapter_by_id: HashMap::new(),
            service_by_id: HashMap::new(),
            getter_by_id: HashMap::new(),
            setter_by_id: HashMap::new(),
            watchers: Arc::new(Mutex::new(WatchMap::new(liveness))),
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
        if !service.getters.is_empty() || !service.setters.is_empty() {
            return Err(Error::InternalError(InternalError::InvalidInitialService));
        }
        let service = ServiceData::new(&self.liveness, service);
        let mut services_for_this_adapter =
            match self.adapter_by_id.get_mut(&service.adapter) {
                None => return Err(Error::InternalError(InternalError::NoSuchAdapter(service.adapter.clone()))),
                Some(&mut AdapterData {ref mut services, ..}) => {
                    services
                }
            };
        let id = service.id.clone();
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
    /// registered, or a channel with the same identifier is already registered.
    /// In either cases, this method reverts all its changes.
    pub fn add_getter(&mut self, getter: Channel<Getter>) -> Result<WatchRequest, Error> {
        let id = getter.id.clone();
        {
            let getter_by_id = &mut self.getter_by_id;
            let service = match self.service_by_id.get_mut(&getter.service) {
                None => return Err(Error::InternalError(InternalError::NoSuchService(getter.service.clone()))),
                Some(service) => service
            };
            let mut service = &mut *service.borrow_mut();
            if service.adapter != getter.adapter {
                return Err(Error::InternalError(InternalError::ConflictingAdapter(service.adapter.clone(), getter.adapter.clone())));
            }
            let getters = &mut service.getters;
            let getter_data = Arc::new(SubCell::new(&self.liveness, GetterData::new(getter, service.tags.clone())));

            let insert_in_service = match InsertInMap::start(getters, vec![(id.clone(), getter_data.clone())]) {
                Ok(transaction) => transaction,
                Err(id) => return Err(Error::InternalError(InternalError::DuplicateGetter(id)))
            };

            let insert_in_getters = match InsertInMap::start(getter_by_id, vec![(id.clone(), getter_data)]) {
                Ok(transaction) => transaction,
                Err(id) => return Err(Error::InternalError(InternalError::DuplicateGetter(id)))
            };

            insert_in_service.commit();
            insert_in_getters.commit();
        }

        Ok(self.aux_getters_may_need_registration(vec![id]))
    }



    /// Remove a setter previously registered on the system. Typically, called by
    /// an adapter when a service is reconfigured to remove one of its setters.
    ///
    /// # Error
    ///
    /// This method returns an error if the setter is not registered or if the service
    /// is not registered. In either case, it attemps to clean as much as possible, even
    /// if the state is inconsistent.
    pub fn remove_getter(&mut self, id: &Id<Getter>) -> Result<(), Error> {
        let getter = match self.getter_by_id.remove(id) {
            None => return Err(Error::InternalError(InternalError::NoSuchGetter(id.clone()))),
            Some(getter) => getter
        };
        Self::aux_getter_may_need_unregistration(&mut *getter.borrow_mut(), true);

        let service_id = &getter.borrow().channel.service;
        match self.service_by_id.get_mut(&service_id) {
            None => Err(Error::InternalError(InternalError::NoSuchService(service_id.clone()))),
            Some(service) => {
                if service.borrow_mut().getters.remove(id).is_none() {
                    Err(Error::InternalError(InternalError::NoSuchGetter(id.clone())))
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
    pub fn add_setter(&mut self, setter: Channel<Setter>) -> Result<(), Error> {
        let service = match self.service_by_id.get_mut(&setter.service) {
            None => return Err(Error::InternalError(InternalError::NoSuchService(setter.service.clone()))),
            Some(service) => service
        };
        let mut service = &mut *service.borrow_mut();
        if service.adapter != setter.adapter {
            return Err(Error::InternalError(InternalError::ConflictingAdapter(service.adapter.clone(), setter.adapter)));
        }

        let id = setter.id.clone();
        let setters = &mut service.setters;
        let setter_data = Arc::new(SubCell::new(&self.liveness, SetterData::new(setter, service.tags.clone())));

        let insert_in_service = match InsertInMap::start(setters, vec![(id.clone(), setter_data.clone())]) {
            Ok(transaction) => transaction,
            Err(id) => return Err(Error::InternalError(InternalError::DuplicateSetter(id)))
        };
        let insert_in_setters = match InsertInMap::start(&mut self.setter_by_id, vec![(id, setter_data)]) {
            Ok(transaction) => transaction,
            Err(id) => return Err(Error::InternalError(InternalError::DuplicateSetter(id)))
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
    pub fn remove_setter(&mut self, id: &Id<Setter>) -> Result<(), Error> {
        let setter = match self.setter_by_id.remove(id) {
            None => return Err(Error::InternalError(InternalError::NoSuchSetter(id.clone()))),
            Some(setter) => setter
        };

        let service_id = &setter.borrow().channel.service;
        match self.service_by_id.get_mut(&service_id) {
            None => Err(Error::InternalError(InternalError::NoSuchService(service_id.clone()))),
            Some(service) => {
                if service.borrow_mut().setters.remove(id).is_none() {
                    Err(Error::InternalError(InternalError::NoSuchSetter(id.clone())))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn get_services(&self, selectors: Vec<ServiceSelector>) -> Vec<Service> {
        // This implementation is not nearly optimal, but it should be sufficient in a system
        // with relatively few services.
        let mut result = Vec::new();
        self.with_services(selectors, |service| {
            result.push(service.borrow().as_service())
        });
        result
    }

    pub fn add_service_tags(&mut self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize {
        let mut result = 0;
        self.with_services(selectors, |service| {
            let service = service.borrow_mut();
            let mut tag_set = service.tags.borrow_mut();
            for tag in &tags {
                let _ = tag_set.insert(tag.clone());
            }
            result += 1;
        });
        result
    }

    pub fn remove_service_tags(&mut self, selectors: Vec<ServiceSelector>, tags: Vec<Id<TagId>>) -> usize {
        let mut result = 0;
        self.with_services(selectors, |service| {
            let service = service.borrow_mut();
            let mut tag_set = service.tags.borrow_mut();
            for tag in &tags {
                let _ = tag_set.remove(&tag);
            }
            result += 1;
        });
        result
    }

    pub fn get_getter_channels(&self, selectors: Vec<GetterSelector>) -> Vec<Channel<Getter>>
    {
        Self::aux_get_channels(selectors, &self.getter_by_id)
    }
    pub fn get_setter_channels(&self, selectors: Vec<SetterSelector>) -> Vec<Channel<Setter>>
    {
        Self::aux_get_channels(selectors, &self.setter_by_id)
    }

    /// Add tags to a getter.
    /// As our in-memory representation stores the same getter both in the Service
    /// and in `self.getters`, we need to update both.
    pub fn add_getter_tags(&mut self, selectors: Vec<GetterSelector>, tags: Vec<Id<TagId>>) -> (WatchRequest, usize) {
        let mut size = 0;
        let mut channels = vec![];
        {
            Self::with_channels_mut(selectors, &mut self.getter_by_id, |mut data| {
                // This channel has changed, we may need to update watches.
                if data.insert_tags(&tags) {
                    channels.push(data.id.clone());
                }
                size += 1;
            });
        }
        (self.aux_getters_may_need_registration(channels), size)
    }

    pub fn add_setter_tags(&mut self, selectors: Vec<SetterSelector>, tags: Vec<Id<TagId>>) -> usize {
        let mut result = 0;
        Self::with_channels_mut(selectors, &mut self.setter_by_id, |mut data| {
            data.insert_tags(&tags);
            result += 1;
        });
        result
    }

    pub fn remove_getter_tags(&mut self, selectors: Vec<GetterSelector>, tags: Vec<Id<TagId>>) -> usize {
        let mut result = 0;
        Self::with_channels_mut(selectors, &mut self.getter_by_id, |mut data| {
            data.remove_tags(&tags);
            Self::aux_getter_may_need_unregistration(&mut data, false);
            result += 1;
        });
        result
    }
    pub fn remove_setter_tags(&mut self, selectors: Vec<SetterSelector>, tags: Vec<Id<TagId>>) -> usize {
        let mut result = 0;
        Self::with_channels_mut(selectors, &mut self.setter_by_id, |mut data| {
            data.remove_tags(&tags);
            result += 1;
        });
        result
    }

    /// Read the latest value from a set of channels
    pub fn prepare_fetch_values(&self, selectors: Vec<GetterSelector>) -> FetchRequest {
        // First, prepare the list of actual getters and group it by adapter.
        // Once we have done this, we can release the lock.
        let mut per_adapter : FetchRequest = HashMap::new();
        let adapter_by_id = &self.adapter_by_id;
        Self::with_channels(selectors, &self.getter_by_id, |data| {
            use std::collections::hash_map::Entry::*;
            let id = data.channel.id.clone();
            let typ = data.channel.mechanism.kind.get_type();
            match per_adapter.entry(data.adapter.clone()) {
                Vacant(entry) => {
                    let adapter = match adapter_by_id.get(&data.channel.adapter) {
                        None => {
                            debug_assert!(false, "Internal inconsistency: Could not find adapter {:?}", id);
                            // FIXME: Logging would be nice.
                            return;
                        },
                        Some(ref adapter_data) => {
                            adapter_data.adapter.clone()
                        }
                    };
                    let mut source = vec![(id, typ)];
                    entry.insert((adapter, source.drain(..).collect()));
                }
                Occupied(mut entry) => {
                    entry.get_mut().1.insert(id, typ);
                }
            };
        });
        per_adapter
    }


    /// Send values to a set of channels
    pub fn prepare_send_values(&self, mut keyvalues: TargetMap<SetterSelector, Value>) -> SendRequest {
        // First determine the channels and group them by adapter.
        let mut per_adapter = HashMap::new();
        for Targetted {select: selectors, payload: value} in keyvalues.drain(..) {
            Self::with_channels(selectors, &self.setter_by_id, |data| {
                use std::collections::hash_map::Entry::*;
                let id = data.channel.id.clone();

                // Check that the values we are about to send have the correct type. If they
                // don't, no need to even send them to the Adapter.
                let typ = data.channel.mechanism.kind.get_type();
                let checked = if value.get_type() == typ {
                    Ok(value.clone())
                } else {
                    Err(Error::TypeError(TypeError {
                        got: value.get_type(),
                        expected: typ
                    }))
                };
                match per_adapter.entry(data.channel.adapter.clone()) {
                    Vacant(entry) => {
                        let mut request = HashMap::new();
                        let mut failures = HashMap::new();
                        match checked {
                            Ok(value) => {
                                request.insert(id, value);
                            }
                            Err(error) => {
                                failures.insert(id, Err(error));
                            }
                        }
                        let adapter = match self.adapter_by_id.get(&data.channel.adapter) {
                            None => {
                                debug_assert!(false, "Internal inconsistency: could not find adapter {}", data.channel.adapter);
                                // FIXME: Logging would be nice.
                                return
                            }
                            Some(adapter) => adapter
                        };
                        entry.insert((adapter.adapter.clone(), (request, failures)));
                    }
                    Occupied(mut entry) => {
                        let &mut(_, (ref mut request, ref mut failures)) = entry.get_mut();
                        match checked {
                            Ok(value) => {
                                request.insert(id, value);
                            }
                            Err(error) => {
                                failures.insert(id, Err(error));
                            }
                        }
                    }
                }
            })
        }
        per_adapter
    }

    fn aux_start_channel_watch(watcher: &mut Arc<WatcherData>,
        getter_data: &mut GetterData,
        filter: &Exactly<Range>,
        adapter_by_id: &HashMap<Id<AdapterId>, AdapterData>,
        per_adapter: &mut WatchRequest)
    {
        use std::collections::hash_map::Entry::*;

        let id = getter_data.id.clone();
        let adapter = getter_data.adapter.clone();

        let insert_in_getter =
            match InsertInMap::start(&mut getter_data.watchers, vec![ ( watcher.key, Arc::downgrade(watcher) )] ) {
            Err(_) => {
                debug_assert!(false, "Internal inconsistency: This watcher is already watching this getter.");
                return
            }
            Ok(transaction) => transaction
        };

        let range = match *filter {
            Exactly::Exactly(ref range) => Some(range.clone()),
            Exactly::Always => None,
            _ => {
                insert_in_getter.commit();
                return // Don't watch data, just topology.
            }
        };

        match per_adapter.entry(adapter) {
            Vacant(entry) => {
                let adapter = match adapter_by_id.get(&getter_data.channel.adapter) {
                    None => {
                        debug_assert!(false, "Internal inconsistency: Could not find adapter {:?}",
                            getter_data.channel.adapter);
                        // FIXME: Logging would be nice.
                        return;
                    },
                    Some(ref adapter_data) => {
                        adapter_data.adapter.clone()
                    }
                };
                entry.insert((adapter, (vec![(id, range)], Arc::downgrade(watcher))));
            },
            Occupied(mut entry) => {
                (entry.get_mut().1).0.push((id, range));
            }
        }

        insert_in_getter.commit();
    }

    pub fn prepare_channel_watch(&mut self, mut watch: TargetMap<GetterSelector, Exactly<Range>>,
        on_event: Box<ExtSender<WatchEvent>>) -> (WatchRequest, WatchKey, Arc<AtomicBool>)
    {
        // Prepare the watcher and store it. Once we leave the lock, every time a channel is
        // added/removed/updated, this will cause us to reexamine whether the channel should
        // be visible to a watcher.
        let mut watcher = self.watchers.lock().unwrap().create(watch.clone(), on_event.clone());
        let is_dropped = watcher.is_dropped.clone();

        // Regroup per adapter.
        let mut per_adapter = HashMap::new();
        let adapter_by_id = &self.adapter_by_id;
        for Targetted { select: selectors, payload: filter } in watch.drain(..) {
            // Find out which channels already match the selectors and attach
            // the watcher immediately.
            let filter = &filter;
            Self::with_channels_mut(selectors, &mut self.getter_by_id, |mut getter_data| {
                Self::aux_start_channel_watch(&mut watcher, &mut getter_data, filter,
                    adapter_by_id, &mut per_adapter)
            });
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
        let mut watcher_data = match self.watchers.lock().unwrap().remove(key) {
            None => {
                // Attempting to unregister a watcher that has not been added yet.
                // This can happen in case of race if `stop_watch` is executed before
                // `start_watch`. Since `is_dropped` is `true`, `start_watch` will be
                // a noop for this watcher, so we're good.
                return
            }
            Some(watcher_data) => watcher_data
        };

        debug_assert!(watcher_data.is_dropped.load(Ordering::Relaxed));

        // Remove the watcher from all getters.
        for getter_id in watcher_data.guards.borrow().keys() {
            let getter = match self.getter_by_id.get_mut(getter_id) {
                None => continue, // Race condition between removing the getter and dropping the watcher.
                Some(getter) => getter
            };
            if getter.borrow_mut().watchers.remove(&watcher_data.key).is_none() {
                debug_assert!(false, "Attempting to unregister a watcher that has already been removed from its getter {:?}, {:?}", key, getter_id);
            }
        }

        // At this stage, theoretically, no getters have a strong reference to watcher_data.

        debug_assert!(Arc::get_mut(&mut watcher_data).is_some(),
            "This watcher is being unregistered but we still have strong references to it. That's not good.");

        // At this stage, `watcher_data` has no strong reference left. All its `guards` will be dropped.
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
        for (_, (adapter, (request, watch_data))) in per_adapter.drain() {
            let watch_data = match watch_data.upgrade() {
                None => {
                    // The watch_data has already been dropped, nothing to do.
                    continue
                }
                Some(watch_data) => watch_data
            };
            let is_dropped = watch_data.is_dropped.clone();
            if is_dropped.load(Ordering::Relaxed) {
                // The WatchGuard has already been dropped.
                continue
            }
            let on_ok = watch_data.on_event.lock().unwrap().filter_map(move |event| {
                if is_dropped.load(Ordering::Relaxed) {
                    // The WatchGuard has already been dropped.
                    // We want to stop propagating messages immediately, even if unregistration
                    // is not necessarily complete yet. Unregistration will be completed after
                    // the call to `stop_watch`.
                    return None;
                }
                Some(match event {
                    AdapterWatchEvent::Enter { id, value } =>
                        WatchEvent::EnterRange {
                            from: id,
                            value: value
                        },
                    AdapterWatchEvent::Exit { id, value } =>
                        WatchEvent::ExitRange {
                            from: id,
                            value: value
                        },
                })
            });
            let mut guards = vec![];
            for (id, result) in adapter.register_watch(request, Box::new(on_ok)) {
                match result {
                    Err(err) => {
                        let event = WatchEvent::InitializationError {
                            channel: id.clone(),
                            error: err
                        };
                        let _ = watch_data.on_event.lock().unwrap().send(event);
                    },
                    // Calling `watch_data.push((id, guard))` requires .write(), so we delay
                    // this until we have grabbed the lock again.
                    Ok(guard) => guards.push((id, guard))
                }
            }
            to_add.push((watch_data, guards));
        }
        to_add
    }

    /// Register a bunch of ongoing watches previously started by `start_watch`.
    pub fn register_ongoing_watch(&mut self, mut ongoing: WatchGuardCommit)
    {
        for (watch_data, mut guards) in ongoing.drain(..) {
            for (id, guard) in guards.drain(..) {
                watch_data.push_guard(id, guard)
            }
        }
    }
}


