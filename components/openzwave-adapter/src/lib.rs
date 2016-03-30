extern crate openzwave_stateful as openzwave;
extern crate foxbox_taxonomy as taxonomy;
extern crate transformable_channels;

use taxonomy::util::Id as TaxId;
use taxonomy::services::{ Setter, Getter, AdapterId, ServiceId, Service, Channel, ChannelKind };
use taxonomy::values::*;
use taxonomy::api::{ ResultMap, Error as TaxError, InternalError };
use taxonomy::adapter::{ AdapterManagerHandle, AdapterWatchGuard, WatchEvent };
use transformable_channels::mpsc::ExtSender;

use openzwave::{ InitOptions, ZWaveManager, ZWaveNotification };
use openzwave::{ CommandClass, ValueGenre, ValueType, ValueID };
use openzwave::{ Controller };

use std::error;
use std::fmt;
use std::thread;
use std::sync::mpsc;
use std::sync::{ Arc, Mutex, RwLock, Weak };
use std::collections::{ HashMap, HashSet };

#[derive(Debug)]
pub enum OpenzwaveError {
    RegisteringError(TaxError),
    UnknownError
}

impl From<TaxError> for OpenzwaveError {
    fn from(err: TaxError) -> Self {
        OpenzwaveError::RegisteringError(err)
    }
}

impl From<()> for OpenzwaveError {
    fn from(_: ()) -> Self {
        OpenzwaveError::UnknownError
    }
}

impl fmt::Display for OpenzwaveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OpenzwaveError::RegisteringError(ref err) => write!(f, "IO error: {}", err),
            OpenzwaveError::UnknownError => write!(f, "Unknown error"),
        }
    }
}

impl error::Error for OpenzwaveError {
    fn description(&self) -> &str {
        match *self {
            OpenzwaveError::RegisteringError(ref err) => err.description(),
            OpenzwaveError::UnknownError => "Unknown error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            OpenzwaveError::RegisteringError(ref err) => Some(err),
            OpenzwaveError::UnknownError => None,
        }
    }
}

#[derive(Debug, Clone)]
struct IdMap<Kind, Type> {
    map: Arc<RwLock<Vec<(TaxId<Kind>, Type)>>>
}

impl<Kind, Type> IdMap<Kind, Type> where Type: Eq + Clone, Kind: Clone {
    fn new() -> Self {
        IdMap {
            map: Arc::new(RwLock::new(Vec::new()))
        }
    }

    fn push(&mut self, id: TaxId<Kind>, ozw_object: Type) -> Result<(), ()> {
        let mut guard = try!(self.map.write().or(Err(())));
        guard.push((id, ozw_object));
        Ok(())
    }

    fn find_tax_id_from_ozw(&self, needle: &Type) -> Result<Option<TaxId<Kind>>, ()> {
        let guard = try!(self.map.read().or(Err(())));
        let find_result = guard.iter().find(|&&(_, ref controller)| controller == needle);
        Ok(find_result.map(|&(ref id, _)| id.clone()))
    }

    fn find_ozw_from_tax_id(&self, needle: &TaxId<Kind>) -> Result<Option<Type>, ()> {
        let guard = try!(self.map.read().or(Err(())));
        let find_result = guard.iter().find(|&&(ref id, _)| id == needle);
        Ok(find_result.map(|&(_, ref ozw_object)| ozw_object.clone()))
    }
}

type SyncExtSender = Mutex<Box<ExtSender<WatchEvent>>>;
type WatchersMap = HashMap<usize, Arc<SyncExtSender>>;
struct Watchers {
    current_index: usize,
    map: Arc<Mutex<WatchersMap>>,
    getter_map: HashMap<TaxId<Getter>, Vec<Weak<SyncExtSender>>>,
}

impl Watchers {
    fn new() -> Self {
        Watchers {
            current_index: 0,
            map: Arc::new(Mutex::new(HashMap::new())),
            getter_map: HashMap::new(),
        }
    }

    fn push(&mut self, tax_id: TaxId<Getter>, watcher: Arc<SyncExtSender>) -> WatcherGuard {
        let index = self.current_index;
        self.current_index += 1;
        {
            let mut map = self.map.lock().unwrap();
            map.insert(index, watcher.clone());
        }

        let entry = self.getter_map.entry(tax_id).or_insert(Vec::new());
        entry.push(Arc::downgrade(&watcher));

        WatcherGuard {
            key: index,
            map: self.map.clone()
        }
    }

    fn get(&self, index: usize) -> Option<Arc<SyncExtSender>> {
        let map = self.map.lock().unwrap();
        map.get(&index).cloned()
    }

    fn get_from_tax_id(&self, tax_id: &TaxId<Getter>) -> Option<Vec<Arc<SyncExtSender>>> {
        self.getter_map.get(tax_id).and_then(|vec| {
            let vec: Vec<_> = vec.iter().filter_map(|weak_sender| weak_sender.upgrade()).collect();
            if vec.len() == 0 { None } else { Some(vec) }
        })
    }
}

fn kind_from_value(value: ValueID) -> Option<ChannelKind> {
    value.get_command_class().map(|cc| match cc {
        CommandClass::SensorBinary => ChannelKind::OpenClosed,
        _ => ChannelKind::Ready // TODO
    })
}

struct WatcherGuard {
    key: usize,
    map: Arc<Mutex<WatchersMap>>,
}

impl Drop for WatcherGuard {
    fn drop(&mut self) {
        let mut map = self.map.lock().unwrap();
        map.remove(&self.key);
    }
}

impl AdapterWatchGuard for WatcherGuard {}

pub struct OpenzwaveAdapter {
    id: TaxId<AdapterId>,
    name: String,
    vendor: String,
    version: [u32; 4],
    ozw: ZWaveManager,
    controller_map: IdMap<ServiceId, Controller>,
    getter_map: IdMap<Getter, ValueID>,
    setter_map: IdMap<Setter, ValueID>,
    watchers: Arc<Mutex<Watchers>>,
}

impl OpenzwaveAdapter {
    pub fn init<T: AdapterManagerHandle + Clone + Send + 'static> (box_manager: &T) -> Result<(), OpenzwaveError> {
        let options = InitOptions {
            device: None // TODO we should expose this as a Value
        };

        let (ozw, rx) = try!(openzwave::init(&options));

        let name = String::from("OpenZwave Adapter");
        let adapter = Arc::new(OpenzwaveAdapter {
            id: TaxId::new(&name),
            name: name,
            vendor: String::from("Mozilla"),
            version: [1, 0, 0, 0],
            ozw: ozw,
            controller_map: IdMap::new(),
            getter_map: IdMap::new(),
            setter_map: IdMap::new(),
            watchers: Arc::new(Mutex::new(Watchers::new())),
        });

        adapter.spawn_notification_thread(rx, box_manager);
        try!(box_manager.add_adapter(adapter));

        Ok(())
    }

    fn spawn_notification_thread<T: AdapterManagerHandle + Clone + Send + 'static>(&self, rx: mpsc::Receiver<ZWaveNotification>, box_manager: &T) {
        let adapter_id = self.id.clone();
        let box_manager = box_manager.clone();
        let mut controller_map = self.controller_map.clone();
        let mut getter_map = self.getter_map.clone();
        let mut setter_map = self.setter_map.clone();

        thread::spawn(move || {
            for notification in rx {
                match notification {
                    ZWaveNotification::ControllerReady(controller) => {
                        let service = format!("OpenZWave/{}", controller.get_home_id());
                        let service_id = TaxId::new(&service);
                        controller_map.push(service_id.clone(), controller);

                        box_manager.add_service(Service::empty(service_id.clone(), adapter_id.clone()));
                    }
                    ZWaveNotification::NodeNew(node)               => {}
                    ZWaveNotification::NodeAdded(node)             => {}
                    ZWaveNotification::NodeRemoved(node)           => {}
                    ZWaveNotification::ValueAdded(value)           => {
                        if value.get_genre() != ValueGenre::ValueGenre_User { continue }

                        let value_id = format!("OpenZWave/{}", value.get_id());

                        let controller_id = controller_map.find_tax_id_from_ozw(&value.get_controller()).unwrap();
                        if controller_id.is_none() { continue }
                        let controller_id = controller_id.unwrap();

                        let has_getter = !value.is_write_only();
                        let has_setter = !value.is_read_only();

                        let kind = kind_from_value(value);
                        if kind.is_none() { continue }
                        let kind = kind.unwrap();

                        if has_getter {
                            let getter_id = TaxId::new(&value_id);
                            getter_map.push(getter_id.clone(), value);
                            box_manager.add_getter(Channel {
                                id: getter_id.clone(),
                                service: controller_id.clone(),
                                adapter: adapter_id.clone(),
                                last_seen: None,
                                tags: HashSet::new(),
                                mechanism: Getter {
                                    kind: kind.clone(),
                                    updated: None
                                }
                            });
                        }

                        if has_setter {
                            let setter_id = TaxId::new(&value_id);
                            setter_map.push(setter_id.clone(), value);
                            box_manager.add_setter(Channel {
                                id: setter_id.clone(),
                                service: controller_id.clone(),
                                adapter: adapter_id.clone(),
                                last_seen: None,
                                tags: HashSet::new(),
                                mechanism: Setter {
                                    kind: kind,
                                    updated: None
                                }
                            });
                        }
                    }
                    ZWaveNotification::ValueChanged(value)         => {}
                    ZWaveNotification::ValueRemoved(value)         => {}
                    ZWaveNotification::Generic(string)             => {}
                }
            }
        });
    }
}

impl taxonomy::adapter::Adapter for OpenzwaveAdapter {
    fn id(&self) -> TaxId<AdapterId> {
        self.id.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn vendor(&self) -> &str {
        &self.vendor
    }

    fn version(&self) -> &[u32; 4] {
        &self.version
    }

    fn fetch_values(&self, set: Vec<TaxId<Getter>>) -> ResultMap<TaxId<Getter>, Option<Value>, TaxError> {
        set.iter().map(|id| {
            let ozw_value: Option<ValueID> = self.getter_map.find_ozw_from_tax_id(id).unwrap();

            let ozw_value: Option<Option<Value>> = ozw_value.map(|ozw_value: ValueID| {
                let result: Option<Value> = match ozw_value.get_type() {
                    ValueType::ValueType_Bool => ozw_value.as_bool().map(
                        |bool| Value::OpenClosed(
                            if bool { OpenClosed::Open } else { OpenClosed::Closed }
                        )
                    ).ok(),
                    _ => Some(Value::Unit)
                };
                result
            });
            let value_result: Result<Option<Value>, TaxError> = ozw_value.ok_or(TaxError::InternalError(InternalError::NoSuchGetter(id.clone())));
            (id.clone(), value_result)
        }).collect()
    }

    fn send_values(&self, values: HashMap<TaxId<Setter>, Value>) -> ResultMap<TaxId<Setter>, (), TaxError> {
        unimplemented!()
    }

    fn register_watch(&self, values: Vec<(TaxId<Getter>, Option<Range>)>, cb: Box<ExtSender<WatchEvent>>) -> ResultMap<TaxId<Getter>, Box<AdapterWatchGuard>, TaxError> {
        let cb = Arc::new(Mutex::new(cb)); // Mutex is necessary because cb is not Sync.
        values.iter().map(|&(ref id, _)| {
            let watch_guard = {
                let mut watchers = self.watchers.lock().unwrap();
                watchers.push(id.clone(), cb.clone())
            };
            let value_result: Result<Box<AdapterWatchGuard>, TaxError> = Ok(Box::new(watch_guard));
            (id.clone(), value_result)
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
