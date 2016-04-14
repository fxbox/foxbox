extern crate openzwave_stateful as openzwave;
extern crate foxbox_taxonomy as taxonomy;
extern crate transformable_channels;
#[macro_use]
extern crate log;

use taxonomy::util::Id as TaxId;
use taxonomy::services::{ Setter, Getter, AdapterId, ServiceId, Service, Channel, ChannelKind };
use taxonomy::values::*;
use taxonomy::api::{ ResultMap, Error as TaxError, InternalError, User };
use taxonomy::adapter::{ AdapterManagerHandle, AdapterWatchGuard, WatchEvent };
use transformable_channels::mpsc::ExtSender;

use openzwave::{ ConfigPath, InitOptions, ZWaveManager, ZWaveNotification };
use openzwave::{ CommandClass, ValueGenre, ValueType, ValueID };
use openzwave::{ Node };

use std::error;
use std::fmt;
use std::{ fs, io };
use std::path::Path;
use std::thread;
use std::sync::mpsc;
use std::sync::{ Arc, Mutex, RwLock, Weak };
use std::collections::{ HashMap, HashSet };

pub use self::OpenzwaveAdapter as Adapter;

#[derive(Debug)]
pub enum Error {
    TaxonomyError(TaxError),
    IOError(io::Error),
    OpenzwaveError(openzwave::Error),
    UnknownError
}

impl From<TaxError> for Error {
    fn from(err: TaxError) -> Self {
        Error::TaxonomyError(err)
    }
}

impl From<()> for Error {
    fn from(_: ()) -> Self {
        Error::UnknownError
    }
}

impl From<openzwave::Error> for Error {
    fn from(error: openzwave::Error) -> Self {
        Error::OpenzwaveError(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IOError(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::TaxonomyError(ref err)  => write!(f, "{}: {}", error::Error::description(self), err),
            Error::OpenzwaveError(ref err) => write!(f, "{}: {}", error::Error::description(self), err),
            Error::IOError(ref err) => write!(f, "{}: {}", error::Error::description(self), err),
            Error::UnknownError => write!(f, "{}", error::Error::description(self)),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::TaxonomyError(_) => "Taxonomy Error",
            Error::OpenzwaveError(_) => "Openzwave Error",
            Error::IOError(_) => "I/O Error",
            Error::UnknownError => "Unknown error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::TaxonomyError(ref err) => Some(err),
            Error::OpenzwaveError(ref err) => Some(err),
            Error::IOError(ref err) => Some(err),
            Error::UnknownError => None,
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
        let find_result = guard.iter().find(|&&(_, ref item)| item == needle);
        Ok(find_result.map(|&(ref id, _)| id.clone()))
    }

    fn find_ozw_from_tax_id(&self, needle: &TaxId<Kind>) -> Result<Option<Type>, ()> {
        let guard = try!(self.map.read().or(Err(())));
        let find_result = guard.iter().find(|&&(ref id, _)| id == needle);
        Ok(find_result.map(|&(_, ref ozw_object)| ozw_object.clone()))
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum SendDirection {
    Enter,
    Exit,
}

trait RangeChecker {
    fn should_send(&self, &Value, SendDirection) -> bool;
}

impl RangeChecker for Option<Range> {
    fn should_send(&self, value: &Value, direction: SendDirection) -> bool {
        match *self {
            None => direction == SendDirection::Enter, // no range means we send only Enter events
            Some(ref range) => range.contains(value)
        }
    }
}

impl RangeChecker for Range {
    fn should_send(&self, value: &Value, _: SendDirection) -> bool {
        self.contains(value)
    }
}

type SyncSender = Mutex<Box<ExtSender<WatchEvent>>>;
type WatchersMap = HashMap<usize, Arc<SyncSender>>;
type RangedWeakSender = (Option<Range>, Weak<SyncSender>);
type RangedSyncSender = (Option<Range>, Arc<SyncSender>);
struct Watchers {
    current_index: usize,
    map: Arc<Mutex<WatchersMap>>,
    getter_map: HashMap<TaxId<Getter>, Vec<RangedWeakSender>>,
}

impl Watchers {
    fn new() -> Self {
        Watchers {
            current_index: 0,
            map: Arc::new(Mutex::new(HashMap::new())),
            getter_map: HashMap::new(),
        }
    }

    fn push(&mut self, tax_id: TaxId<Getter>, range: Option<Range>, watcher: Arc<SyncSender>) -> WatcherGuard {
        let index = self.current_index;
        self.current_index += 1;
        {
            let mut map = self.map.lock().unwrap();
            map.insert(index, watcher.clone());
        }

        let entry = self.getter_map.entry(tax_id).or_insert(Vec::new());
        entry.push((range, Arc::downgrade(&watcher)));

        WatcherGuard {
            key: index,
            map: self.map.clone()
        }
    }

    fn get(&self, index: usize) -> Option<Arc<SyncSender>> {
        let map = self.map.lock().unwrap();
        map.get(&index).cloned()
    }

    fn get_from_tax_id(&self, tax_id: &TaxId<Getter>) -> Option<Vec<RangedSyncSender>> {
        self.getter_map.get(tax_id).and_then(|vec| {
            let vec: Vec<_> = vec.iter().filter_map(|&(ref range, ref weak_sender)| {
                let range = range.clone();
                weak_sender.upgrade().map(|sender| (range, sender))
            }).collect();
            if vec.len() == 0 { None } else { Some(vec) }
        })
    }
}

fn tax_kind_from_ozw_vid(vid: &ValueID) -> Option<ChannelKind> {
    match (vid.get_type(), vid.get_command_class(), vid.get_index()) {
        (ValueType::ValueType_Bool, Some(CommandClass::DoorLock),     0) => Some(ChannelKind::DoorLocked),
        (ValueType::ValueType_Bool, Some(CommandClass::SensorBinary), _) => Some(ChannelKind::OpenClosed),
        // (ValueType::ValueType_Bool, Some(_)) => Some(ChannelKind::OnOff), TODO Find a proper type
        // Unrecognized command class or type - we don't know what to do with it.
        _ => None
    }
}

fn ozw_vid_as_tax_value(vid: &ValueID) -> Option<Value> {
    if vid.get_command_class().is_none() {
        return None;
    }

    match vid.get_type() {
        ValueType::ValueType_Bool => {
            if let Ok(value) = vid.as_bool() {
                match tax_kind_from_ozw_vid(vid) {
                    // Some(ChannelKind::OnOff)  => Some(Value::OnOff(if value {OnOff::On} else {OnOff::Off})), // TODO support switches
                    Some(ChannelKind::OpenClosed)  => Some(Value::OpenClosed(if value {OpenClosed::Open} else {OpenClosed::Closed})),
                    Some(ChannelKind::DoorLocked)  => Some(Value::DoorLocked(if value {DoorLocked::Locked} else {DoorLocked::Unlocked})),
                    _ => None,
                }
            } else {
                None
            }
        },
        _ => None,   // TODO: Support more ValueType's
    }
}

fn set_ozw_vid_from_tax_value(vid: &ValueID, value: Value) -> Result<(), TaxError> {
    if vid.get_command_class().is_none() {
        return Err(TaxError::InternalError(InternalError::GenericError(format!("Unknown command class: {}", vid.get_command_class_id()))));
    }

    match vid.get_type() {
        ValueType::ValueType_Bool => {
            match value {
                //Value::OnOff(onOff) => { vid.set_bool(onOff == OnOff::On); } // TODO support switches
                Value::OpenClosed(open_closed) => { vid.set_bool(open_closed == OpenClosed::Open); }
                Value::DoorLocked(locked_unlocked) => { vid.set_bool(locked_unlocked == DoorLocked::Locked); }
                _ => { return Err(TaxError::InternalError(InternalError::GenericError(format!("Unsupported value type: {:?}", value)))); }
            }
        }
        _ => { return Err(TaxError::InternalError(InternalError::GenericError(format!("Unsupported OZW type: {:?}", vid.get_type())))); }
    };
    Ok(())
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

type ValueCache = HashMap<TaxId<Getter>, Value>;

pub struct OpenzwaveAdapter {
    id: TaxId<AdapterId>,
    name: String,
    vendor: String,
    version: [u32; 4],
    ozw: ZWaveManager,
    node_map: IdMap<ServiceId, Node>,
    getter_map: IdMap<Getter, ValueID>,
    setter_map: IdMap<Setter, ValueID>,
    watchers: Arc<Mutex<Watchers>>,
    value_cache: Arc<Mutex<ValueCache>>,
}

fn ensure_directory<T: AsRef<Path> + ?Sized>(directory: &T) -> Result<(), Error> {
    let path = directory.as_ref();
    if path.exists() && !path.is_dir() {
        return Err(
            Error::IOError(io::Error::new(io::ErrorKind::AlreadyExists, format!("The file {} already exists and isn't a directory.", path.display())))
        );
    }

    if !path.exists() {
        try!(fs::create_dir(path));
    }

    Ok(())
}

impl OpenzwaveAdapter {
    pub fn init<T: AdapterManagerHandle + Send + Sync + 'static> (box_manager: &Arc<T>, user_path: &str) -> Result<(), Error> {

        try!(ensure_directory(user_path));

        let options = InitOptions {
            device: None, // TODO we should expose this as a Value
            config_path: ConfigPath::Default, // This is where the default system configuraton is, usually contains the device information.
            user_path: user_path, // This is where we can override the system configuration, and where the network layout and logs are stored.
        };

        let (ozw, rx) = try!(match openzwave::init(&options) {
            Err(openzwave::Error::NoDeviceFound) => {
                // early return: we should not impair foxbox startup for this error.
                // TODO concept of FatalError vs IgnoreableError
                error!("No ZWave device has been found.");
                return Ok(());
            }
            result => result
        });

        let name = String::from("OpenZwave Adapter");
        let adapter = Arc::new(OpenzwaveAdapter {
            id: TaxId::new(&name),
            name: name,
            vendor: String::from("Mozilla"),
            version: [1, 0, 0, 0],
            ozw: ozw,
            node_map: IdMap::new(),
            getter_map: IdMap::new(),
            setter_map: IdMap::new(),
            watchers: Arc::new(Mutex::new(Watchers::new())),
            value_cache: Arc::new(Mutex::new(HashMap::new())),
        });

        adapter.spawn_notification_thread(rx, box_manager);
        try!(box_manager.add_adapter(adapter));

        info!("Started Openzwave adapter.");

        Ok(())
    }

    fn spawn_notification_thread<T: AdapterManagerHandle + Send + Sync + 'static>(&self, rx: mpsc::Receiver<ZWaveNotification>, box_manager: &Arc<T>) {
        let adapter_id = self.id.clone();
        let box_manager = box_manager.clone();
        let mut node_map = self.node_map.clone();
        let mut getter_map = self.getter_map.clone();
        let mut setter_map = self.setter_map.clone();
        let watchers = self.watchers.clone();
        let value_cache = self.value_cache.clone();

        thread::spawn(move || {
            for notification in rx {
                //debug!("Received notification {:?}", notification);
                match notification {
                    ZWaveNotification::ControllerReady(_controller) => {}
                    ZWaveNotification::NodeNew(_node)               => {}
                    ZWaveNotification::NodeAdded(node)              => {
                        let service = format!("OpenZWave-{:08x}-{:02x}", node.get_home_id(), node.get_id());
                        let service_id = TaxId::new(&service);
                        node_map.push(service_id.clone(), node);

                        let mut service = Service::empty(service_id.clone(), adapter_id.clone());
                        service.properties.insert(String::from("name"), node.get_name());
                        service.properties.insert(String::from("product_name"), node.get_product_name());
                        service.properties.insert(String::from("manufacturer_name"), node.get_manufacturer_name());
                        service.properties.insert(String::from("location"), node.get_location());

                        box_manager.add_service(service);
                    }
                    ZWaveNotification::NodeNaming(_node)             => {
                        // unfortunately we can't change a service' properties :(
                        // https://github.com/fxbox/taxonomy/issues/97
                        // When it's done we can move the properties change from above to here.
                    }
                    ZWaveNotification::NodeRemoved(_node)           => {}
                    ZWaveNotification::ValueAdded(vid)              => {
                        if vid.get_genre() != ValueGenre::ValueGenre_User { continue }

                        let value_id = format!("OpenZWave-{:08x}-{:016x} ({})", vid.get_home_id(), vid.get_id(), vid.get_label());

                        let node_id = node_map.find_tax_id_from_ozw(&vid.get_node()).unwrap();
                        if node_id.is_none() { continue }
                        let node_id = node_id.unwrap();

                        let has_getter = !vid.is_write_only();
                        let has_setter = !vid.is_read_only();

                        let kind = tax_kind_from_ozw_vid(&vid);
                        if kind.is_none() { continue }
                        let kind = kind.unwrap();

                        if has_getter {
                            let getter_id = TaxId::new(&value_id);
                            getter_map.push(getter_id.clone(), vid);
                            box_manager.add_getter(Channel {
                                id: getter_id.clone(),
                                service: node_id.clone(),
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
                            setter_map.push(setter_id.clone(), vid);
                            box_manager.add_setter(Channel {
                                id: setter_id.clone(),
                                service: node_id.clone(),
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
                    ZWaveNotification::ValueChanged(vid)          => {
                        match vid.get_type() {
                            ValueType::ValueType_Bool => {},
                            _ => continue // ignore non-bool vals for now
                        };

                        let tax_id = match getter_map.find_tax_id_from_ozw(&vid) {
                            Ok(Some(tax_id)) => tax_id,
                            _ => continue
                        };

                        let tax_value = match ozw_vid_as_tax_value(&vid) {
                            Some(value) => value,
                            _ => continue
                        };

                        let watchers = watchers.lock().unwrap();

                        let watchers = match watchers.get_from_tax_id(&tax_id) {
                            Some(watchers) => watchers,
                            _ => continue
                        };

                        let previous_value = {
                            let mut cache = value_cache.lock().unwrap();
                            let previous = cache.get(&tax_id).cloned();
                            cache.insert(tax_id.clone(), tax_value.clone());
                            previous
                        };

                        for &(ref range, ref sender) in &watchers {
                            debug!("Openzwave::Adapter::ValueChanged iterate over watcher {:?} {:?}", tax_id, range);

                            let should_send_value = range.should_send(&tax_value, SendDirection::Enter);

                            if let Some(ref previous_value) = previous_value {
                                let should_send_previous = range.should_send(previous_value, SendDirection::Exit);
                                // If the new and the old values are both in the same range, we
                                // need to send nothing.
                                if should_send_value && should_send_previous { continue }

                                if should_send_previous {
                                    debug!("Openzwave::Adapter::ValueChanged Sending event Exit {:?} {:?}", tax_id, tax_value);
                                    let sender = sender.lock().unwrap();
                                    sender.send(
                                        WatchEvent::Exit { id: tax_id.clone(), value: tax_value.clone() }
                                    );
                                }
                            }

                            if should_send_value {
                                debug!("Openzwave::Adapter::ValueChanged Sending event Enter {:?} {:?}", tax_id, tax_value);
                                let sender = sender.lock().unwrap();
                                sender.send(
                                    WatchEvent::Enter { id: tax_id.clone(), value: tax_value.clone() }
                                );
                            }
                        }
                    }
                    ZWaveNotification::ValueRemoved(_value)         => {}
                    ZWaveNotification::Generic(_string)             => {}
                    _ => {}
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

    fn fetch_values(&self, mut set: Vec<TaxId<Getter>>, _: User) -> ResultMap<TaxId<Getter>, Option<Value>, TaxError> {
        set.drain(..).map(|id| {
            let ozw_vid = self.getter_map.find_ozw_from_tax_id(&id).unwrap();

            let tax_value: Option<Option<Value>> = ozw_vid.map(|ozw_vid: ValueID| {
                if !ozw_vid.is_set() { return None }

                ozw_vid_as_tax_value(&ozw_vid)
            });
            let value_result: Result<Option<Value>, TaxError> = tax_value.ok_or(TaxError::InternalError(InternalError::NoSuchGetter(id.clone())));
            (id, value_result)
        }).collect()
    }

    fn send_values(&self, mut values: HashMap<TaxId<Setter>, Value>, _: User) -> ResultMap<TaxId<Setter>, (), TaxError> {
        values.drain().map(|(id, value)| {
            if let Some(ozw_vid) = self.setter_map.find_ozw_from_tax_id(&id).unwrap() {
                (id, set_ozw_vid_from_tax_value(&ozw_vid, value))
            } else {
                (id.clone(), Err(TaxError::InternalError(InternalError::NoSuchSetter(id))))
            }
        }).collect()
    }

    fn register_watch(&self, mut values: Vec<(TaxId<Getter>, Option<Range>, Box<ExtSender<WatchEvent>>)>) -> Vec<(TaxId<Getter>, Result<Box<AdapterWatchGuard>, TaxError>)> {
        debug!("Openzwave::Adapter::register_watch Should register some watchers");
        values.drain(..).map(|(id, range, sender)| {
            let sender = Arc::new(Mutex::new(sender)); // Mutex is necessary because cb is not Sync.
            debug!("Openzwave::Adapter::register_watch Should register a watcher for {:?} {:?}", id, range);
            let watch_guard = {
                let mut watchers = self.watchers.lock().unwrap();
                watchers.push(id.clone(), range.clone(), sender.clone())
            };
            let value_result: Result<Box<AdapterWatchGuard>, TaxError> = Ok(Box::new(watch_guard));

            // if there is a set value already, let's send it.
            let ozw_value: Option<ValueID> = self.getter_map.find_ozw_from_tax_id(&id).unwrap(); // FIXME no unwrap
            if let Some(value) = ozw_value {
                if value.is_set() && value.get_type() == ValueType::ValueType_Bool {
                    if let Some(value) = ozw_vid_as_tax_value(&value) {
                        self.value_cache.lock().unwrap().insert(id.clone(), value.clone());
                        if range.should_send(&value, SendDirection::Enter) {
                            debug!("Openzwave::Adapter::register_watch Sending event Enter {:?} {:?}", id, value);
                            let sender = sender.lock().unwrap();
                            sender.send(
                                WatchEvent::Enter { id: id.clone(), value: value }
                            );
                        }
                    }
                }
            }

            (id, value_result)
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}

