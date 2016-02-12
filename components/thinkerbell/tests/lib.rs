use std::sync::Arc;
use std::marker::PhantomData;
use std::collections::HashMap;
use std::sync::mpsc::{channel, sync_channel, Sender};
use std::thread;

extern crate thinkerbell;
use thinkerbell::dependencies::{DeviceAccess, Watcher};
use thinkerbell::values::{Value, Range, Number};
use thinkerbell::lang::{Execution, UncheckedCtx, UncheckedEnv, Script, Requirement, Resource, Trigger, Conjunction, Condition};

extern crate chrono;
use self::chrono::Duration;

/// An implementation of DeviceAccess for the purpose of unit testing.
struct TestEnv;

impl DeviceAccess for TestEnv {
    type DeviceKind = String;
    type Device = String;
    type InputCapability = String;
    type OutputCapability = String;
    type Watcher = TestWatcher;

    fn get_watcher() -> Self::Watcher {
        Self::Watcher::new()
    }

    fn get_device_kind(key: &String) -> Option<String> {
        // A set of well-known device kinds
        for s in vec!["clock", "kind 2", "kind 3"] {
            if s == key {
                return Some(key.clone());
            }
        }
        None
    }

    fn get_device(key: &String) -> Option<String> {
        // A set of well-known devices
        for s in vec!["built-in clock", "device 2", "device 3"] {
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
        for s in vec!["output 1", "output 2", "output 3"] {
            if s == key {
                return Some(key.clone());
            }
        }
        None
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
                if ticks >= 5 {
                    assert!(false, "TestWatcher: timeout");
                }
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        Stop => {
                            println!("TestWatcher: done");
                            return;
                        },
                        Insert(k, cb) => {
                            watchers.insert(k, cb);
                        }
                    }
                } else {
                    println!("TestWatcher: The clock is ticking: {}s", ticks);
                    thread::sleep(std::time::Duration::new(1, 0));

                    let clock_key = clock_key.clone();
                    let ticks = ticks.clone();
                    match watchers.get(&clock_key) {
                        None => {},
                        Some(ref watcher) => {
                            println!("TestWatcher: Informing watcher");
                            let val = Value::Num(Number::new(ticks as f64, ()));
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

///
/// Compilation tests
///

#[test]
/// Attempt to start an empty script. This should succeed.
fn test_compile_empty_script() {
    let script : Script<UncheckedCtx, UncheckedEnv> = Script {
        metadata: (),
        requirements: vec![],
        allocations: vec![],
        rules: vec![],
    };


    // Compiling an empty script should succeed.
    let (tx, rx) = channel();
    Execution::<TestEnv>::new().start(script, move |res| {tx.send(res).unwrap();});
    let result = rx.recv().unwrap();
    assert!(result.is_ok());
}

#[test]
/// Attempt to compile a script with the wrong number of allocations.
/// This should fail.
fn test_compile_bad_number_of_allocations() {
    use thinkerbell::lang::SourceError::*;
    use thinkerbell::lang::Error::*;

    let script : Script<UncheckedCtx, UncheckedEnv> = Script {
        metadata: (),

        // One requirement
        requirements: vec![Arc::new(Requirement {
            kind: "clock".to_owned(), // This kind exists, so that shouldn't cause a failure.
            inputs: vec!["ticks".to_owned()], // This input exists, so that shouldn't cause a failure.
            outputs: vec![],
            min: 1,
            max: 1,
            phantom: PhantomData
        })],

        // No allocations
        allocations: vec![],
        rules: vec![],
    };


    let (tx, rx) = channel();
    Execution::<TestEnv>::new().start(script, move |res| {tx.send(res).unwrap();});
    let result = rx.recv().unwrap();


    match result {
        Err(SourceError(AllocationLengthError{..})) => (), // success
        Err(err) => {
            println!("Wrong error {:?}", err);
            assert!(false);
        },
        Ok(_) => {
            assert!(false, "Compilation should have failed");
        }
    }
}

#[test]
/// Attempt to compile a script with a resource of a kind that doesn't exist on the box.
/// This should fail.
fn test_compile_wrong_kind() {
    use thinkerbell::lang::DevAccessError::*;
    use thinkerbell::lang::Error::*;

    let script : Script<UncheckedCtx, UncheckedEnv> = Script {
        metadata: (),

        // One requirement
        requirements: vec![Arc::new(Requirement {
            kind: "not available on this foxbox".to_owned(), // This kind doesn't exists on the system, so that should cause a failure.
            inputs: vec!["ticks".to_owned()], // This input exists, so that shouldn't cause a failure.
            outputs: vec![],
            min: 1,
            max: 1,
            phantom: PhantomData
        })],

        // As many allocations
        allocations: vec![Resource {
            devices: vec![],
            phantom: PhantomData
        }],
        rules: vec![],
    };

    let (tx, rx) = sync_channel(0);
    Execution::<TestEnv>::new().start(script, move |res| {
        tx.send(res).unwrap();
    });
    let result = rx.recv().unwrap();

    println!("test_compile_wrong_kind: result {:?}", &result);
    match result {
        Err(DevAccessError(DeviceKindNotFound)) => (), // success
        Err(err) => {
            println!("Wrong error {:?}", err);
            assert!(false);
        },
        Ok(_) => {
            assert!(false, "Compilation should have failed");
        }
    }
}

///
/// Execution tests
///

#[test]
fn test_start_stop() {
    let script : Script<UncheckedCtx, UncheckedEnv> = Script {
        metadata: (),

        // One requirement
        requirements: vec![Arc::new(Requirement {
            kind: "clock".to_owned(),
            inputs: vec!["ticks".to_owned()],
            outputs: vec![],
            min: 1,
            max: 1,
            phantom: PhantomData
        })],

        // As many allocations
        allocations: vec![Resource {
            devices: vec!["built-in clock".to_owned()],
            phantom: PhantomData
        }],
        rules: vec![],
    };

    println!("test_start_stop 1");
    let (tx, rx) = sync_channel(0);
    let mut runner = Execution::<TestEnv>::new();
    println!("test_start_stop 2");
    runner.start(script, move |res| {tx.send(res).unwrap();});
    println!("test_start_stop 3");

    let result = rx.recv().unwrap();
    assert!(result.is_ok(), "Compilation should succeed {:?}", result);
    println!("test_start_stop 4");
    println!("test_start_stop 5");

    // Wait until the script has stopped
    let (tx2, rx2) = channel();
    runner.stop(move |result| {
        println!("test_start_stop: stop cb 1");
        tx2.send(result).unwrap();
        println!("test_start_stop: stop cb 2");
    });
    let result = rx2.recv().unwrap();
    assert!(result.is_ok());
    println!("test_start_stop 6");

}

#[test]
fn test_watch_one_input() {
    let script : Script<UncheckedCtx, UncheckedEnv> = Script {
        metadata: (),

        // One requirement
        requirements: vec![Arc::new(Requirement {
            kind: "clock".to_owned(),
            inputs: vec!["ticks".to_owned()],
            outputs: vec![],
            min: 1,
            max: 1,
            phantom: PhantomData
        })],

        // As many allocations
        allocations: vec![Resource {
            devices: vec!["built-in clock".to_owned()],
            phantom: PhantomData
        }],
        rules: vec![Trigger{
            condition: Conjunction {
                all: vec![Condition {
                    input: 0, // The first (and only) input
                    capability: "ticks".to_owned(),
                    range: Range::Geq(Number::new(3.0, ())),
                    state: (),
                }],
                state: (),
            },
            execute: vec![],
            cooldown: Duration::seconds(0),
        }],
    };

    let (tx, rx) = sync_channel(0);
    let mut runner = Execution::<TestEnv>::new();
    runner.start(script, move |res| {tx.send(res).unwrap();});
    let result = rx.recv().unwrap();
    assert!(result.is_ok());

    thread::sleep(std::time::Duration::new(5, 0));
    // Wait until the script has stopped
    // Wait until the script has stopped
    let (tx, rx) = sync_channel(0);
    runner.stop(move |result| {tx.send(result).unwrap();} );
    let result = rx.recv().unwrap();
    assert!(result.is_ok());
}
