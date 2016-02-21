extern crate docopt;

extern crate fxbox_thinkerbell;
use fxbox_thinkerbell::run::Execution;
use fxbox_thinkerbell::parse::Parser;
use fxbox_thinkerbell::dependencies::{DevEnv, ExecutableDevEnv, Watcher};
use fxbox_thinkerbell::values::Range;

extern crate fxbox_taxonomy;
use fxbox_taxonomy::values::Value;

extern crate serde_json;

#[macro_use]
extern crate lazy_static;

use std::io::prelude::*;
use std::fs::File;
use std::sync::Mutex;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

const USAGE: &'static str = "
Usage: simulator [options]...
       simulator --help

-h, --help            Show this message.
-r, --ruleset <path>  Load rules from a file.
";


/// An implementation of DevEnv for the purpose of unit testing.
lazy_static!(
    static ref OUTPUTS: Mutex<HashMap</*device*/String, HashMap</*capability*/String, HashMap<String, Value>> >> = Mutex::new(HashMap::new());
    );

struct TestEnv;

impl TestEnv {
    fn reset() {
        let mut outputs = OUTPUTS.lock().unwrap();
        outputs.clear();
    }

    fn get_state(device: &String, cap: &String) -> Option<HashMap<String, Value>> {
        println!("TestEnv: Checking the state of {}, {}", device, cap);
        let outputs = OUTPUTS.lock().unwrap();
        outputs.get(device).and_then(|per_device| {
            per_device.get(cap).cloned()
        })
    }

    fn set_state(device: &String, cap: &String, state: HashMap<String, Value>) {
        println!("TestEnv: Setting the state of {}, {}", device, cap);
        let mut outputs = OUTPUTS.lock().unwrap();
        if !outputs.contains_key(device) {
            outputs.insert(device.clone(), HashMap::new());
        }
        let per_device = outputs.get_mut(device).unwrap();
        per_device.insert(cap.clone(), state);
    }
}

impl DevEnv for TestEnv {
    type DeviceKind = String;
    type Device = String;
    type InputCapability = String;
    type OutputCapability = String;
}

impl ExecutableDevEnv for TestEnv {
        type Watcher = TestWatcher;

    fn get_watcher() -> Self::Watcher {
        println!("Initializing watcher");
        Self::Watcher::new()
    }

    fn get_device_kind(key: &String) -> Option<String> {
        // A set of well-known device kinds
        for s in vec!["clock", "display device", "kind 3"] {
            if s == key {
                return Some(key.clone());
            }
        }
        None
    }

    fn get_device(key: &String) -> Option<String> {
        // A set of well-known devices
        for s in vec!["built-in clock", "built-in display 1", "built-in display 2"] {
            if s == key {
                return Some(key.clone());
            }
        }
        None
    }

    fn get_input_capability(key: &String) -> Option<String> {
        // A set of well-known inputs
        for s in vec!["ticks", "input 2:string", "input 3: bool"] {
            if s == key {
                return Some(key.clone());
            }
        }
        None
    }

    fn get_output_capability(key: &String) -> Option<String> {
        for s in vec!["show", "output 2", "output 3"] {
            if s == key {
                return Some(key.clone());
            }
        }
        None
    }

    fn send(device: &Self::Device, cap: &Self::OutputCapability, value: &HashMap<String, Value>) {
        println!("Sending {} to {} with value {:?}", device, cap, value);
        TestEnv::set_state(device, cap, value.clone());
    }
}

/// A mock watcher that informs clients with new values regularly.

enum TestWatcherMsg {
    Stop,
    Insert((String, String), Box<Fn(Value) + Send>)
}

struct TestWatcher {
    tx: Sender<TestWatcherMsg>,
}

impl TestWatcher {
    fn new() -> Self {
        use TestWatcherMsg::*;
        let (tx, rx) = channel();

        thread::spawn(move || {
            let mut watchers = HashMap::new();
            let mut ticks = 0;

            let clock_key = ("built-in clock".to_owned(), "ticks".to_owned());
            loop {
                ticks += 1;
                if ticks >= 10 {
                    assert!(false, "TestWatcher: timeout");
                }
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        Stop => {
                            return;
                        },
                        Insert(k, cb) => {
                            watchers.insert(k, cb);
                        }
                    }
                } else {
                    thread::sleep(std::time::Duration::new(1, 0));

                    let clock_key = clock_key.clone();
                    let ticks = ticks.clone();
                    match watchers.get(&clock_key) {
                        None => {},
                        Some(ref watcher) => {
                            let val = Value::Duration(Duration::new(ticks, 0));
                            watcher(val);
                        }
                    }
                }
            }
        });
        TestWatcher {
            tx: tx,
        }
    }
}

impl Watcher for TestWatcher {
    type Witness = ();
    type Device = String;
    type InputCapability = String;

    fn add<F>(&mut self,
              device: &Self::Device,
              input: &Self::InputCapability,
              _condition: &Range,
              cb: F) -> Self::Witness where F:Fn(Value) + Send + 'static
{
        self.tx.send(TestWatcherMsg::Insert((device.clone(), input.clone()), Box::new(cb))).unwrap();
        ()
    }
}

impl Drop for TestWatcher {
    fn drop(&mut self) {
        self.tx.send(TestWatcherMsg::Stop).unwrap();
    }
}

fn main () {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.argv(std::env::args().into_iter()).parse())
        .unwrap_or_else(|e| e.exit());

    let mut runners = Vec::new();
    
    for path in args.get_vec("--ruleset") {
        print!("Loading ruleset from {}... ", path);
        let mut file = File::open(path).unwrap();
        let mut source = String::new();
        file.read_to_string(&mut source).unwrap();
        let json : self::serde_json::Value = self::serde_json::from_str(&source).unwrap();
        let script = Parser::parse(json).unwrap();
        print!("starting... ");

        let mut runner = Execution::<TestEnv>::new();
        let (tx, rx) = channel();
        runner.start(script, move |res| {tx.send(res).unwrap();});
        rx.recv().unwrap().unwrap();

        runners.push(runner);
        println!("ready.");
    }

    loop {}
}
