//! A simple adapter designe solely to print messages on the console.
//!
//! Useful for logging.

use foxbox_taxonomy::api::{ Error, InternalError, User };
use foxbox_taxonomy::manager::*;
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::{ Range, Value };

use transformable_channels::mpsc::*;

use std::collections::{ HashMap, HashSet };
use std::sync::Arc;


static ADAPTER_NAME: &'static str = "Console adapter (built-in)";
static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
static ADAPTER_VERSION: [u32;4] = [0, 0, 0, 0];

pub struct Console {
    setter_stdout_id: Id<Setter>
}

impl Console {
    pub fn id() -> Id<AdapterId> {
        Id::new("console@link.mozilla.org")
    }
    pub fn service_console_id() -> Id<ServiceId> {
        Id::new("service:console@link.mozilla.org")
    }
    pub fn setter_stdout_id() -> Id<Setter> {
        Id::new("setter:stdout@link.mozilla.org")
    }
}
impl Adapter for Console {
    fn id(&self) -> Id<AdapterId> {
        Self::id()
    }

    fn name(&self) -> &str {
        ADAPTER_NAME
    }

    fn vendor(&self) -> &str {
        ADAPTER_VENDOR
    }

    fn version(&self) -> &[u32;4] {
        &ADAPTER_VERSION
    }

    fn fetch_values(&self, mut set: Vec<Id<Getter>>, _: User) -> ResultMap<Id<Getter>, Option<Value>, Error> {
        set.drain(..).map(|id| {
            (id.clone(), Err(Error::InternalError(InternalError::NoSuchGetter(id))))
        }).collect()
    }

    fn send_values(&self, mut values: HashMap<Id<Setter>, Value>, user: User) -> ResultMap<Id<Setter>, (), Error> {
        values.drain()
            .map(|(id, value)| {
                let result = {
                    if id == self.setter_stdout_id {
                        if let Value::String(s) = value {
                            info!("[{:?}] {}", user, s);
                        } else {
                            info!("[{:?}] {:?}", user, value);
                        }
                        Ok(())
                    } else {
                        Err(Error::InternalError(InternalError::NoSuchSetter(id.clone())))
                    }
                };
                (id, result)
            })
            .collect()
    }

    fn register_watch(&self, mut watch: Vec<(Id<Getter>, Option<Range>)>,
        _: Box<ExtSender<WatchEvent>>) ->
            ResultMap<Id<Getter>, Box<AdapterWatchGuard>, Error>
    {
        watch.drain(..).map(|(id, _)| {
            (id.clone(), Err(Error::InternalError(InternalError::NoSuchGetter(id))))
        }).collect()
    }
}


impl Console {
    pub fn init(adapt: &Arc<AdapterManager>) -> Result<(), Error> {
        let service_console_id = Console::service_console_id();
        let setter_stdout_id = Console::setter_stdout_id();
        let console = Arc::new(Console {
            setter_stdout_id: setter_stdout_id.clone()
        });
        try!(adapt.add_adapter(console));
        let mut service = Service::empty(service_console_id.clone(), Console::id());
        service.properties.insert("model".to_owned(), "Mozilla console v1".to_owned());
        try!(adapt.add_service(service));
        try!(adapt.add_setter(Channel {
                tags: HashSet::new(),
                adapter: Console::id(),
                id: setter_stdout_id.clone(),
                last_seen: None,
                service: service_console_id.clone(),
                mechanism: Setter {
                    kind: ChannelKind::Log,
                    updated: None
                }
        }));
        Ok(())
    }
}
