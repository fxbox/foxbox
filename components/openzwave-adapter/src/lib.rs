extern crate openzwave_stateful as openzwave;
extern crate foxbox_taxonomy as taxonomy;
extern crate transformable_channels;

use taxonomy::util::Id as TaxId;
use taxonomy::services::{ Setter, Getter, AdapterId, ServiceId, Service, Channel, ChannelKind };
use taxonomy::values::{ Value, Range };
use taxonomy::api::{ ResultMap, Error as TaxError };
use taxonomy::adapter::{ AdapterManagerHandle, AdapterWatchGuard, WatchEvent };
use transformable_channels::mpsc::ExtSender;

use openzwave::{ InitOptions, ZWaveManager, ZWaveNotification };
use openzwave::{ CommandClass, ValueGenre, ValueID };
use openzwave::{ Controller };

use std::error;
use std::fmt;
use std::thread;
use std::sync::mpsc;
use std::sync::RwLock;
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

struct IdMap<Kind, Type> {
    map: Vec<(TaxId<Kind>, Type)>
}

impl<Kind, Type> IdMap<Kind, Type> where Type: Eq {
    fn new() -> Self {
        IdMap {
            map: Vec::new()
        }
    }

    fn push(&mut self, id: TaxId<Kind>, ozw_object: Type) {
        self.map.push((id, ozw_object))
    }

    fn find_tax_id(&mut self, ozw_object: Type) -> Option<&TaxId<Kind>> {
        let find_result = self.map.iter().find(|&&(_, ref controller)| controller == &ozw_object);
        find_result.map(|&(ref id, _)| id)
    }
}

fn kind_from_value(value: ValueID) -> Option<ChannelKind> {
    value.get_command_class().map(|cc| match cc {
        CommandClass::SensorBinary => ChannelKind::OpenClosed,
        _ => ChannelKind::Ready // TODO
    })
}

pub struct OpenzwaveAdapter {
    id: TaxId<AdapterId>,
    name: String,
    vendor: String,
    version: [u32; 4],
    ozw: ZWaveManager,
}

impl OpenzwaveAdapter {
    pub fn init<T: AdapterManagerHandle + Clone + Send + 'static> (box_manager: &T) -> Result<(), OpenzwaveError> {
        let options = InitOptions {
            device: None // TODO we should expose this as a Value
        };

        let (ozw, rx) = try!(openzwave::init(&options));

        let name = String::from("OpenZwave Adapter");
        let adapter = Box::new(OpenzwaveAdapter {
            id: TaxId::new(&name),
            name: name,
            vendor: String::from("Mozilla"),
            version: [1, 0, 0, 0],
            ozw: ozw,
        });

        adapter.spawn_notification_thread(rx, box_manager);
        try!(box_manager.add_adapter(adapter));

        Ok(())
    }

    fn spawn_notification_thread<T: AdapterManagerHandle + Clone + Send + 'static>(&self, rx: mpsc::Receiver<ZWaveNotification>, box_manager: &T) {
        let adapter_id = self.id.clone();
        let box_manager = box_manager.clone();

        thread::spawn(move || {
            let mut controller_map: IdMap<ServiceId, Controller> = IdMap::new();
            let mut getter_map: IdMap<Getter, ValueID> = IdMap::new();
            let mut setter_map: IdMap<Setter, ValueID> = IdMap::new();

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

                        let controller_id = controller_map.find_tax_id(value.get_controller());
                        if controller_id.is_none() { continue }
                        let controller_id = controller_id.unwrap();

                        let has_getter = !value.is_write_only();
                        let has_setter = !value.is_read_only();

                        let kind = kind_from_value(value);
                        if kind.is_none() { continue }
                        let kind = kind.unwrap();

                        if has_getter {
                            let getter_id = TaxId::new(&value_id);
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
        let state = self.ozw.get_state();

        unimplemented!()
    }

    fn send_values(&self, values: HashMap<TaxId<Setter>, Value>) -> ResultMap<TaxId<Setter>, (), TaxError> {
        unimplemented!()
    }

    fn register_watch(&self, values: Vec<(TaxId<Getter>, Option<Range>)>, cb: Box<ExtSender<WatchEvent>>) -> ResultMap<TaxId<Getter>, Box<AdapterWatchGuard>, TaxError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
