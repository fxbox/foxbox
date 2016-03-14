#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate docopt;
extern crate serde;
extern crate serde_json;

extern crate foxbox_adapters;
extern crate foxbox_thinkerbell;
extern crate foxbox_taxonomy;

extern crate transformable_channels;

use foxbox_adapters::adapter::*;
use foxbox_adapters::manager::*;

use foxbox_thinkerbell::compile::ExecutableDevEnv;
use foxbox_thinkerbell::run::Execution;
use foxbox_thinkerbell::parse::Parser;

use foxbox_taxonomy::api::{ API, Error, ResultMap };
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::*;
use foxbox_taxonomy::util::Id;

use std::io::prelude::*;
use std::fs::File;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::str::FromStr;

use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer};

use transformable_channels::mpsc::*;

const USAGE: &'static str = "
Usage: simulator [options]...
       simulator --help

-h, --help            Show this message.
-r, --ruleset <path>  Load decision rules from a file.
-e, --events <path>   Load events from a file.
-s, --slowdown <num>  Duration of each tick, in floating point seconds. Default: no slowdown.
";

static VERSION : [u32;4] = [0, 0, 0, 0];

/// A back-end holding values and watchers, shared by all the virtual adapters of this simulator.
struct TestSharedAdapterBackend {
    /// The latest known value for each getter.
    getter_values: HashMap<Id<Getter>, Value>,

    /// A channel to inform when things happen.
    on_event: Box<ExtSender<SimulatorEvent>>,

    /// All watchers. 100% counter-optimized
    counter: usize,
    watchers: HashMap<usize, (Id<Getter>, Option<Range>, Box<ExtSender<WatchEvent>>)>,
}

impl TestSharedAdapterBackend {
    fn new(on_event: Box<ExtSender<SimulatorEvent>>) -> Self {
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

    fn send_values(&self, mut values: Vec<(Id<Setter>, Value)>) -> ResultMap<Id<Setter>, (), Error> {
        values.drain(..).map(|(id, value)| {
            let event = SimulatorEvent::Send {
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
        use AdapterOp::*;
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
enum SimulatorEvent {
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
    fn send_values(&self, values: Vec<(Id<Setter>, Value)>) -> ResultMap<Id<Setter>, (), Error> {
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
        received.iter().cloned().map(|(id, result)| {
            let tx_unregister = tx_unregister.clone();
            (id, match result {
                Err(err) => Err(err),
                Ok(key) => {
                    let guard = TestAdapterWatchGuard {
                        tx: Box::new(tx_unregister),
                        key: key
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
struct TestEnv {
    /// The manager in charge of all adapters.
    manager: AdapterManager,

    ///
    on_event: Box<ExtSender<SimulatorEvent>>,
    back_end: Box<ExtSender<AdapterOp>>,
}
impl ExecutableDevEnv for TestEnv {
    // Don't bother stopping watches.
    type WatchGuard = <AdapterManager as API>::WatchGuard;
    type API = AdapterManager;

    fn api(&self) -> Self::API {
        self.manager.clone()
    }
}
impl TestEnv {
    fn new(on_event: Box<ExtSender<SimulatorEvent>>) -> Self {
        let (tx, rx) = channel();
        let on_event_clone = on_event.clone();
        thread::spawn(move || {
            let mut back_end = TestSharedAdapterBackend::new(on_event_clone);
            for msg in rx {
                back_end.execute(msg);
            }
        });

        TestEnv {
            on_event: on_event,
            manager: AdapterManager::new(),
            back_end: Box::new(tx),
        }
    }

    fn report_error<T>(&self, result: Result<T, Error>) {
        match result {
            Ok(_) => {},
            Err(err) => {
                let _ = self.on_event.send(SimulatorEvent::Error(err));
            }
        }
    }

    /// Execute instructions issued by the user.
    fn execute(&self, instruction: Instruction) {
        use self::Instruction::*;
        match instruction {
            AddAdapters(vec) => {
                for id in vec {
                    let id = Id::new(id);
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
        let _ = self.on_event.send(SimulatorEvent::Done);
    }
}
impl Serialize for TestEnv {
    fn serialize<S>(&self, _: &mut S) -> Result<(), S::Error> where S: Serializer {
        panic!("Why are we attempting to serialize the env?")
    }
}
impl Deserialize for TestEnv {
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
        values: Vec<(Id<Setter>, Value)>,
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

fn main () {
    use foxbox_thinkerbell::run::ExecutionEvent::*;

    println!("Preparing simulator.");
    let (tx, rx) = channel();
    let env = TestEnv::new(Box::new(tx));
    let (tx_done, rx_done) = channel();
    thread::spawn(move || {
        for event in rx {
            match event {
                SimulatorEvent::Done => {
                    let _ = tx_done.send(()).unwrap();
                },
                event => println!("<<< {:?}", event)
            }
        }
    });

    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.argv(std::env::args().into_iter()).parse())
        .unwrap_or_else(|e| e.exit());

    let slowdown = match args.find("--slowdown") {
        None => Duration::new(0, 0),
        Some(value) => {
            let vec = value.as_vec();
            if vec.is_empty() || vec[0].is_empty() {
                Duration::new(0, 0)
            } else {
                let s = f64::from_str(vec[0]).unwrap();
                Duration::new(s as u64, (s.fract() * 1_000_000.0) as u32)
            }
        }
    };

    let mut runners = Vec::new();

    println!("Loading rulesets.");
    for path in args.get_vec("--ruleset") {
        print!("Loading ruleset from {}\n", path);
        let mut file = File::open(path).unwrap();
        let mut source = String::new();
        file.read_to_string(&mut source).unwrap();
        let script = Parser::parse(source).unwrap();
        print!("Ruleset loaded, launching... ");

        let mut runner = Execution::<TestEnv>::new();
        let (tx, rx) = channel();
        runner.start(env.api(), script, tx).unwrap();
        match rx.recv().unwrap() {
            Starting { result: Ok(()) } => println!("ready."),
            err => panic!("Could not launch script {:?}", err)
        }
        runners.push(runner);
    }

    println!("Loading sequences of events.");
    for path in args.get_vec("--events") {
        println!("Loading events from {}...", path);
        let mut file = File::open(path).unwrap();
        let mut source = String::new();
        file.read_to_string(&mut source).unwrap();
        let script : Vec<Instruction> = serde_json::from_str(&source).unwrap();
        println!("Sequence of events loaded, playing...");

        for event in script {
            thread::sleep(slowdown.clone());
            println!(">>> {:?}", event);
            env.execute(event);
            rx_done.recv().unwrap();
        }
    }

    println!("Simulation complete.");
    thread::sleep(Duration::new(100, 0));
}

