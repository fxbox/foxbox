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
use openzwave::{ ValueGenre, ValueID };
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
            let mut controller_map: Vec<(TaxId<ServiceId>, Controller)> = Vec::new();

            for notification in rx {
                match notification {
                    ZWaveNotification::ControllerReady(controller) => {
                        let service = format!("OpenZWave/{}", controller.get_home_id());
                        let service_id = TaxId::new(&service);
                        controller_map.push((service_id.clone(), controller));

                        box_manager.add_service(Service::empty(service_id.clone(), adapter_id.clone()));
                    }
                    ZWaveNotification::NodeNew(node)               => {}
                    ZWaveNotification::NodeAdded(node)             => {}
                    ZWaveNotification::NodeRemoved(node)           => {}
                    ZWaveNotification::ValueAdded(value)           => {
                        let value_id = format!("OpenZWave/{}", value.get_id());
                        let controller_pair = controller_map.iter().find(|&&(_, controller)| controller == value.get_controller());
                        if controller_pair.is_none() { continue; }
                        let ref controller_id = controller_pair.unwrap().0;

                        if value.is_read_only() {
                            box_manager.add_getter(Channel {
                                id: TaxId::new(&value_id),
                                service: controller_id.clone(),
                                adapter: adapter_id.clone(),
                                last_seen: None,
                                tags: HashSet::new(),
                                mechanism: Getter {
                                    kind: ChannelKind::Ready,
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
