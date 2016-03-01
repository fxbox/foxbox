#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate docopt;
extern crate serde;
extern crate serde_json;

extern crate foxbox_thinkerbell;
extern crate foxbox_taxonomy;

use foxbox_thinkerbell::compile::ExecutableDevEnv;
use foxbox_thinkerbell::run::Execution;
use foxbox_thinkerbell::parse::Parser;

use foxbox_taxonomy::devices::*;
use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::values::*;
use foxbox_taxonomy::api::{API, WatchEvent, WatchOptions};
use foxbox_taxonomy::util::Id;

type APIError = foxbox_taxonomy::api::Error;

use std::io::prelude::*;
use std::fs::File;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::str::FromStr;

use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer};
const USAGE: &'static str = "
Usage: simulator [options]...
       simulator --help

-h, --help            Show this message.
-r, --ruleset <path>  Load decision rules from a file.
-e, --events <path>   Load events from a file.
-s, --slowdown <num>  Duration of each tick, in floating point seconds. Default: no slowdown. 
";

#[derive(Default, Serialize, Deserialize)]
struct TestEnv {
    front: APIFrontEnd,
}
impl ExecutableDevEnv for TestEnv {
    // Don't bother stopping watches.
    type WatchGuard = ();
    type API = APIFrontEnd;

    fn api(&self) -> Self::API {
        self.front.clone()
    }
}
impl TestEnv {
    fn new<F>(cb: F) -> Self
        where F: Fn(Update) + Send + 'static {
        TestEnv {
            front: APIFrontEnd::new(cb)
        }
    }

    pub fn execute(&self, instruction: Instruction) {
        self.front.tx.send(instruction.as_op()).unwrap();
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// Instructions given to the simulator.
pub enum Instruction {
    AddNodes(Vec<Node>),
    AddGetters(Vec<Channel<Getter>>),
    AddSetters(Vec<Channel<Setter>>),
    InjectGetterValue{id: Id<Getter>, value: Value},
}
impl Instruction {
    fn as_op(self) -> Op {
        use Instruction::*;
        match self {
            AddNodes(vec) => Op::AddNodes(vec),
            AddGetters(vec) => Op::AddGetters(vec),
            AddSetters(vec) => Op::AddSetters(vec),
            InjectGetterValue{id, value} => Op::InjectGetterValue{id:id, value: value}
        }
    }
}


/// Operations internal to the simulator.
enum Op {
    AddNodes(Vec<Node>),
    AddGetters(Vec<Channel<Getter>>),
    AddSetters(Vec<Channel<Setter>>),
    AddWatch{options: Vec<WatchOptions>, cb: Box<Fn(WatchEvent) + Send + 'static>},
    SendValue{selectors: Vec<SetterSelector>, value: Value, cb: Box<Fn(Vec<(Id<Setter>, Result<(), APIError>)>) + Send>},
    InjectGetterValue{id: Id<Getter>, value: Value},
}

#[derive(Debug)]
enum Update {
    Put { id: Id<Setter>, value: Value, result: Result<(), String> },
    Done,
}

#[derive(Debug)]
struct GetterWithState {
    getter: Channel<Getter>,
    state: Option<Value>,
}
impl GetterWithState {
    fn set_state(&mut self, val: Value) {
        self.state = Some(val);
    }
}

struct APIBackEnd {
    nodes: HashMap<Id<NodeId>, Node>,
    getters: HashMap<Id<Getter>, GetterWithState>,
    setters: HashMap<Id<Setter>, Channel<Setter>>,
    watchers: Vec<(WatchOptions, Arc<Box<Fn(WatchEvent)>>)>,
    post_updates: Arc<Fn(Update)>
}
impl APIBackEnd {
    fn new<F>(cb: F) -> Self
        where F: Fn(Update) + Send + 'static {
        APIBackEnd {
            nodes: HashMap::new(),
            getters: HashMap::new(),
            setters: HashMap::new(),
            watchers: Vec::new(),
            post_updates: Arc::new(cb)
        }
    }
    
    fn add_nodes(&mut self, nodes: Vec<Node>) {
        for node in nodes {
            let previous = self.nodes.insert(node.id.clone(), node);
            if previous.is_some() {
                assert!(previous.is_none());
            }
        }
        // In a real implementation, this should update all NodeSelector
    }
    fn add_getters(&mut self, getters: Vec<Channel<Getter>>) {
        for getter in getters {
            let previous = self.getters.insert(
                getter.id.clone(),
                GetterWithState {
                    getter:getter,
                    state: None
                });
            assert!(previous.is_none());
        }
        // In a real implementation, this should update all GetterSelectors
    }
    fn add_setters(&mut self, setters: Vec<Channel<Setter>>)  {
        for setter in setters {
            let previous = self.setters.insert(setter.id.clone(), setter);
            assert!(previous.is_none());
        }
        // In a real implementation, this should update all SetterSelectors
    }

    fn add_watch(&mut self, options: Vec<WatchOptions>, cb: Box<Fn(WatchEvent)>) {
        let cb = Arc::new(cb);
        for opt in options {
            self.watchers.push((opt, cb.clone()));
        }
    }

    fn inject_getter_value(&mut self, id: Id<Getter>, value: Value) {
        let mut getter = self.getters.get_mut(&id).unwrap();
        getter.set_state(value.clone());

        // The list of watchers watching for new values on this getter.
        let watchers = self.watchers.iter().filter(|&&(ref options, _)| {
            options.should_watch_values &&
                options.source.matches(&getter.getter)
        });
        for watcher in watchers {
            watcher.1(WatchEvent::Value {
                from: id.clone(),
                value: value.clone()
            });
        }
    }

    fn put_value(&mut self,
                 selectors: Vec<SetterSelector>,
                 value: Value,
                 cb: Box<Fn(Vec<(Id<Setter>, Result<(), APIError>)>)>)
    {
        // Very suboptimal implementation.
        let setters = self.setters
            .values()
            .filter(|setter|
                    selectors.iter()
                    .find(|selector| selector.matches(setter))
                    .is_some());
        let results = setters.map(|setter| {
            let result;
            let internal_result;
            if value.get_type() == setter.mechanism.kind.get_type() {
                result = Ok(());
                internal_result = Ok(());
            } else {
                result = Err(foxbox_taxonomy::api::Error::TypeError);
                internal_result = Err(format!("Invalid type, expected {:?}, got {:?}", value.get_type(), setter.mechanism.kind.get_type()));
            }
            (*self.post_updates)(Update::Put {
                id: setter.id.clone(),
                value: value.clone(),
                result: internal_result
            });
            (setter.id.clone(), result)
        }).collect();
        cb(results)
    }
}

#[derive(Clone)]
struct APIFrontEnd {
    // By definition, the cell is never empty
    tx: Sender<Op>
}
impl Serialize for APIFrontEnd {
    fn serialize<S>(&self, _: &mut S) -> Result<(), S::Error> where S: Serializer {
        panic!("WTF are we doing serializing the front-end?");
    }
}
impl Deserialize for APIFrontEnd {
    fn deserialize<D>(_: &mut D) -> Result<Self, D::Error> where D: Deserializer {
        panic!("WTF are we doing deserializing the front-end?");
    }
}
impl Default for APIFrontEnd {
    fn default() -> Self {
        panic!("WTF are we doing calling default() for the front-end?");
    }
}

impl APIFrontEnd {
    pub fn new<F>(cb: F) -> Self
        where F: Fn(Update) + Send + 'static {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let mut api = APIBackEnd::new(cb);
            for msg in rx.iter() {
                use Op::*;
                match msg {
                    AddNodes(vec) => api.add_nodes(vec),
                    AddGetters(vec) => api.add_getters(vec),
                    AddSetters(vec) => api.add_setters(vec),
                    AddWatch{options, cb} => api.add_watch(options, cb),
                    SendValue{selectors, value, cb} => api.put_value(selectors, value, cb),
                    InjectGetterValue{id, value} => api.inject_getter_value(id, value),
                }
                (*api.post_updates)(Update::Done)
            }
        });
        APIFrontEnd {
            tx: tx
        }
    }
}

impl API for APIFrontEnd {
    type WatchGuard = ();

    fn get_nodes(&self, _: &Vec<NodeSelector>) -> Vec<Node> {
        unimplemented!()
    }

    fn put_node_tag(&self, _: &Vec<NodeSelector>, _: &Vec<String>) -> usize {
        unimplemented!()
    }

    fn delete_node_tag(&self, _: &Vec<NodeSelector>, _: String) -> usize {
        unimplemented!()
    }

    fn get_getter_channels(&self, _: &Vec<GetterSelector>) -> Vec<Channel<Getter>> {
        unimplemented!()
    }
    fn get_setter_channels(&self, _: &Vec<SetterSelector>) -> Vec<Channel<Setter>> {
        unimplemented!()
    }
    fn put_getter_tag(&self, _: &Vec<GetterSelector>, _: &Vec<String>) -> usize {
        unimplemented!()
    }
    fn put_setter_tag(&self, _: &Vec<SetterSelector>, _: &Vec<String>) -> usize {
        unimplemented!()
    }
    fn delete_getter_tag(&self, _: &Vec<GetterSelector>, _: &Vec<String>) -> usize {
        unimplemented!()
    }
    fn delete_setter_tag(&self, _: &Vec<SetterSelector>, _: &Vec<String>) -> usize {
        unimplemented!()
    }
    fn get_channel_value(&self, _: &Vec<GetterSelector>) -> Vec<(Id<Getter>, Result<Value, APIError>)> {
        unimplemented!()
    }
    fn put_channel_value(&self, selectors: &Vec<SetterSelector>, value: Value) -> Vec<(Id<Setter>, Result<(), APIError>)> {
        let (tx, rx) = channel();
        self.tx.send(Op::SendValue {
            selectors: selectors.clone(),
            value: value,
            cb: Box::new(move |result| { tx.send(result).unwrap(); })
        }).unwrap();
        rx.recv().unwrap()
    }
    fn register_channel_watch(&self, options: Vec<WatchOptions>, cb: Box<Fn(WatchEvent) + Send + 'static>) -> Self::WatchGuard {
        self.tx.send(Op::AddWatch {
            options: options,
            cb: cb
        }).unwrap();
        ()
    }

}
fn main () {
    use foxbox_thinkerbell::run::ExecutionEvent::*;

    println!("Preparing simulator.");
    let (tx, rx) = channel();
    let env = TestEnv::new(move |event| {
        let _ = tx.send(event);
    });
    let (tx_done, rx_done) = channel();
    thread::spawn(move || {
        for event in rx.iter() {
            match event {
                Update::Done => {
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
        runner.start(env.api(), script, move |res| {
            let _ = tx.send(res);
        });
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

