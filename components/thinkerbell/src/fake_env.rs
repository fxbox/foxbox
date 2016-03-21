use compile::ExecutableDevEnv;

use foxbox_adapters::adapter::*;
use foxbox_adapters::manager::*;

use foxbox_taxonomy::api::{ API, Error };
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::*;
use foxbox_taxonomy::util::Id;

use std::collections::HashMap;
use std::thread;

use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer};

use transformable_channels::mpsc::*;


static VERSION : [u32;4] = [0, 0, 0, 0];

/// A back-end holding values and watchers, shared by all the virtual adapters of this simulator.
struct TestSharedAdapterBackend {
    /// The latest known value for each getter.
    getter_values: HashMap<Id<Getter>, Value>,

    /// A channel to inform when things happen.
    on_event: Box<ExtSender<FakeEnvEvent>>,

    /// All watchers. 100% counter-optimized
    counter: usize,
    watchers: HashMap<usize, (Id<Getter>, Option<Range>, Box<ExtSender<WatchEvent>>)>,
}

impl TestSharedAdapterBackend {
    fn new(on_event: Box<ExtSender<FakeEnvEvent>>) -> Self {
        TestSharedAdapterBackend {
            getter_values: HashMap::new(),
            on_event: on_event,
            counter: 0,
            watchers: HashMap::new()
        }
    }

    fn fetch_values(&self, mut getters: Vec<Id<Getter>>) -> ResultMap<Id<Getter>, Option<Value>, Error> {
        getters.drain(..).map(|id| {
            let got = self.getter_values.get(&id).cloned();
            (id, Ok(got))
        }).collect()
    }

    fn send_values(&self, mut values: HashMap<Id<Setter>, Value>) -> ResultMap<Id<Setter>, (), Error> {
        values.drain().map(|(id, value)| {
            let event = FakeEnvEvent::Send {
                id: id.clone(),
                value: value
            };
            let _ = self.on_event.send(event);
            (id, Ok(()))
        }).collect()
    }

    fn register_watch(&mut self, mut source: Vec<(Id<Getter>, Option<Range>)>,
        cb: Box<ExtSender<WatchEvent>>) ->
            ResultMap<Id<Getter>, usize, Error>
    {
        let results = source.drain(..).map(|(id, range)| {
            let result = (id.clone(), Ok(self.counter));
            self.watchers.insert(self.counter, (id, range, cb.clone()));
            self.counter += 1;
            result
        }).collect();
        results
    }

    fn remove_watch(&mut self, key: usize) {
        let _ = self.watchers.remove(&key);
    }

    fn inject_getter_values(&mut self, mut values: Vec<(Id<Getter>, Value)>)
    {
        for (id, value) in values.drain(..) {
            let old = self.getter_values.insert(id.clone(), value.clone());
            for watcher in self.watchers.values() {
                let (ref watched_id, ref range, ref cb) = *watcher;
                if *watched_id != id {
                    continue;
                }
                if let Some(ref range) = *range {
                    let was_met = match old {
                        None => false,
                        Some(ref value) => range.contains(value)
                    };
                    match (was_met, range.contains(&value)) {
                        (false, true) => {
                            let _ = cb.send(WatchEvent::Enter {
                                id: id.clone(),
                                value: value.clone()
                            });
                        }
                        (true, false) => {
                            let _ = cb.send(WatchEvent::Exit {
                                id: id.clone(),
                                value: value.clone()
                            });
                        }
                        _ => continue
                    }
                } else {
                    let _ = cb.send(WatchEvent::Enter {
                        id: id.clone(),
                        value: value.clone()
                    });
                }
            }
        }
    }

    fn execute(&mut self, op: AdapterOp) {
        use self::AdapterOp::*;
        match op {
            FetchValues { getters, tx } => {
                let _ = tx.send(self.fetch_values(getters));
            }
            SendValues { values, tx } => {
                let _ = tx.send(self.send_values(values));
            }
            Watch { source, cb, tx } => {
                let _ = tx.send(self.register_watch(source, cb));
            }
            Unwatch(key) => {
                let _ = self.remove_watch(key);
            }
            InjectGetterValues(values) => {
                let _ = self.inject_getter_values(values);
            }
        }
    }
}

/// Events of interest, that should be displayed to the user of the simulator.
#[derive(Debug)]
pub enum FakeEnvEvent {
    /// Some value was sent to a setter channel.
    Send {
        id: Id<Setter>,
        value: Value
    },

    /// Some error took place.
    Error(Error),

    /// Handling of an instruction is complete.
    Done,
}

/// An adapter for the simulator.
struct TestAdapter {
    id: Id<AdapterId>,

    /// The back-end holding the state of this adapter. Shared between all adapters.
    back_end: Box<ExtSender<AdapterOp>>,
}
impl TestAdapter {
    fn new(id: Id<AdapterId>, back_end: Box<ExtSender<AdapterOp>>) -> Self {
        TestAdapter {
            id: id,
            back_end: back_end,
        }
    }
}

impl Adapter for TestAdapter {
    /// An id unique to this adapter. This id must persist between
    /// reboots/reconnections.
    fn id(&self) -> Id<AdapterId> {
        self.id.clone()
    }

    /// The name of the adapter.
    fn name(&self) -> &str {
        "Test Adapter"
    }
    fn vendor(&self) -> &str {
        "test@foxlink"
    }
    fn version(&self) -> &[u32;4] {
        &VERSION
    }
    // ... more metadata

    /// Request values from a group of channels.
    ///
    /// The AdapterManager always attempts to group calls to `fetch_values` by `Adapter`, and then
    /// expects the adapter to attempt to minimize the connections with the actual devices.
    ///
    /// The AdapterManager is in charge of keeping track of the age of values.
    fn fetch_values(&self, getters: Vec<Id<Getter>>) -> ResultMap<Id<Getter>, Option<Value>, Error> {
        let (tx, rx) = channel();
        self.back_end.send(AdapterOp::FetchValues {
            getters: getters,
            tx: Box::new(tx),
        }).unwrap();
        rx.recv().unwrap()
    }

    /// Request that values be sent to channels.
    ///
    /// The AdapterManager always attempts to group calls to `send_values` by `Adapter`, and then
    /// expects the adapter to attempt to minimize the connections with the actual devices.
    fn send_values(&self, values: HashMap<Id<Setter>, Value>) -> ResultMap<Id<Setter>, (), Error> {
        let (tx, rx) = channel();
        self.back_end.send(AdapterOp::SendValues {
            values: values,
            tx: Box::new(tx),
        }).unwrap();
        rx.recv().unwrap()
    }

    /// Watch a bunch of getters as they change.
    ///
    /// The `AdapterManager` always attempts to group calls to `fetch_values` by `Adapter`, and
    /// then expects the adapter to attempt to minimize the connections with the actual devices.
    /// The Adapter should however be ready to handle concurrent `register_watch` on the same
    /// devices, possibly with distinct `Option<Range>` options.
    ///
    /// If a `Range` option is set, the watcher expects to receive `EnterRange`/`ExitRange` events
    /// whenever the value available on the device enters/exits the range. If the `Range` is
    /// a `Range::Eq(x)`, the adapter may decide to reject the request or to interpret it as
    /// a `Range::BetweenEq { min: x, max: x }`.
    ///
    /// If no `Range` option is set, the watcher expects to receive `EnterRange` events whenever
    /// a new value is available on the device. The adapter may decide to reject the request if
    /// this is clearly not the expected usage for a device, or to throttle it.
    fn register_watch(&self, source: Vec<(Id<Getter>, Option<Range>)>,
        cb: Box<ExtSender<WatchEvent>>) ->
            ResultMap<Id<Getter>, Box<AdapterWatchGuard>, Error>
    {
        let (tx, rx) = channel();
        self.back_end.send(AdapterOp::Watch {
            source: source,
            cb: cb,
            tx: Box::new(tx),
        }).unwrap();
        let received = rx.recv().unwrap();
        let tx_unregister = self.back_end.clone();
        received.iter().map(|(id, result)| {
            let tx_unregister = tx_unregister.clone();
            (id.clone(), match *result {
                Err(ref err) => Err(err.clone()),
                Ok(ref key) => {
                    let guard = TestAdapterWatchGuard {
                        tx: Box::new(tx_unregister),
                        key: key.clone()
                    };
                    Ok(Box::new(guard) as Box<AdapterWatchGuard>)
                }
            })
        }).collect()
    }
}

/// A watchguard for a TestAdapter
#[derive(Clone)]
struct TestAdapterWatchGuard {
    tx: Box<ExtSender<AdapterOp>>,
    key: usize,
}
impl AdapterWatchGuard for TestAdapterWatchGuard {}
impl Drop for TestAdapterWatchGuard {
    fn drop(&mut self) {
        let _ = self.tx.send(AdapterOp::Unwatch(self.key));
    }
}


/// The test environment.
#[derive(Clone)]
pub struct FakeEnv {
    /// The manager in charge of all adapters.
    manager: AdapterManager,

    ///
    on_event: Box<ExtSender<FakeEnvEvent>>,
    back_end: Box<ExtSender<AdapterOp>>,
}
impl ExecutableDevEnv for FakeEnv {
    // Don't bother stopping watches.
    type WatchGuard = <AdapterManager as API>::WatchGuard;
    type API = AdapterManager;

    fn api(&self) -> Self::API {
        self.manager.clone()
    }

    type TimerGuard = (); // FIXME: Implement
    fn start_timer(&self, duration: Duration, timer: Box<ExtSender<()>>) -> Self::TimerGuard {
        unimplemented!()
    }
}
impl FakeEnv {
    pub fn new(on_event: Box<ExtSender<FakeEnvEvent>>) -> Self {
        let (tx, rx) = channel();
        let on_event_clone = on_event.clone();
        thread::spawn(move || {
            let mut back_end = TestSharedAdapterBackend::new(on_event_clone);
            for msg in rx {
                back_end.execute(msg);
            }
        });

        FakeEnv {
            on_event: on_event,
            manager: AdapterManager::new(),
            back_end: Box::new(tx),
        }
    }

    fn report_error<T>(&self, result: Result<T, Error>) {
        match result {
            Ok(_) => {},
            Err(err) => {
                let _ = self.on_event.send(FakeEnvEvent::Error(err));
            }
        }
    }

    /// Execute instructions issued by the user.
    pub fn execute(&self, instruction: Instruction) {
        use self::Instruction::*;
        match instruction {
            AddAdapters(vec) => {
                for id in vec {
                    let id = Id::new(&id);
                    let adapter = Box::new(TestAdapter::new(id, self.back_end.clone()));
                    let result = self.manager.add_adapter(adapter);
                    self.report_error(result);
                }
            },
            AddServices(vec) => {
                for service in vec {
                    let result = self.manager.add_service(service);
                    self.report_error(result);
                }
            },
            AddGetters(vec) => {
                for getter in vec {
                    let result = self.manager.add_getter(getter);
                    self.report_error(result);
                }
            },
            AddSetters(vec) => {
                for setter in vec {
                    let result = self.manager.add_setter(setter);
                    self.report_error(result);
                }
            },
            InjectGetterValues(vec) => {
                self.back_end.send(AdapterOp::InjectGetterValues(vec)).unwrap()
            },
//            _ => unimplemented!()
        }
        let _ = self.on_event.send(FakeEnvEvent::Done);
    }
}
impl Serialize for FakeEnv {
    fn serialize<S>(&self, _: &mut S) -> Result<(), S::Error> where S: Serializer {
        panic!("Why are we attempting to serialize the env?")
    }
}
impl Deserialize for FakeEnv {
    fn deserialize<D>(_: &mut D) -> Result<Self, D::Error> where D: Deserializer {
         panic!("Why are we attempting to deserialize the env?")
    }
}

#[derive(Deserialize, Debug)]
/// Instructions given to the simulator by the user.
pub enum Instruction {
    AddAdapters(Vec<String>),
    AddServices(Vec<Service>),
    AddGetters(Vec<Channel<Getter>>),
    AddSetters(Vec<Channel<Setter>>),
    InjectGetterValues(Vec<(Id<Getter>, Value)>),
}

/// Operations internal to a TestAdapter.
enum AdapterOp {
    FetchValues {
        getters: Vec<Id<Getter>>,
        tx: Box<ExtSender<ResultMap<Id<Getter>, Option<Value>, Error>>>
    },
    SendValues {
        values: HashMap<Id<Setter>, Value>,
        tx: Box<ExtSender<ResultMap<Id<Setter>, (), Error>>>
    },
    Watch {
        source: Vec<(Id<Getter>, Option<Range>)>,
        cb: Box<ExtSender<WatchEvent>>,
        tx: Box<ExtSender<ResultMap<Id<Getter>, usize, Error>>>
    },
    Unwatch(usize),
    InjectGetterValues(Vec<(Id<Getter>, Value)>),
}



