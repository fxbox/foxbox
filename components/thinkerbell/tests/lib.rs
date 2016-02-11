use std::sync::Arc;
use std::marker::PhantomData;


extern crate thinkerbell;

use thinkerbell::dependencies::{DeviceAccess, Watcher};
use thinkerbell::values::{Value, Range};
use thinkerbell::lang::{ExecutionTask, UncheckedCtx, UncheckedEnv, Script, Requirement, Resource};


/// An implementation of DeviceAccess for the purpose of unit testing.
struct TestEnv;

impl DeviceAccess for TestEnv {
    type DeviceKind = String;
    type Device = String;
    type InputCapability = String;
    type OutputCapability = String;
    type Watcher = TestWatcher;

    fn get_device_kind(key: &String) -> Option<String> {
        for s in vec!["kind 1", "kind 2", "kind 3"] {
            if s == key {
                return Some(key.clone());
            }
        }
        None
    }

    fn get_device(key: &String) -> Option<String> {
        for s in vec!["device 1", "device 2", "device 3"] {
            if s == key {
                return Some(key.clone());
            }
        }
        None
    }

    fn get_input_capability(key: &String) -> Option<String> {
        for s in vec!["input 1", "input 2", "input 3"] {
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

struct TestWatcher; // FIXME: Implement this
impl Watcher for TestWatcher {
    type Witness = ();
    type Device = String;
    type InputCapability = String;

    fn new() -> Self {
        TestWatcher
    }

    fn add<F>(&mut self,
              device: &Self::Device,
              input: &Self::InputCapability,
              condition: &Range,
              cb: F) -> Self::Witness where F:FnOnce(Value)
    {
        panic!("TestWatcher is not implemented yet");
    }

}

#[test]
/// Attempt to compile an empty script. This should succeed.
fn test_compile_empty_script() {
    let script : Script<UncheckedCtx, UncheckedEnv> = Script {
        metadata: (),
        requirements: vec![],
        allocations: vec![],
        rules: vec![],
    };

    // Compiling an empty script should succeed.
    let task = ExecutionTask::<TestEnv>::new(&script);
    assert!(task.is_ok());
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
            kind: "kind 1".to_owned(), // This kind exists, so that shouldn't cause a failure.
            inputs: vec!["input 1".to_owned()], // This input exists, so that shouldn't cause a failure.
            outputs: vec![],
            min: 1,
            max: 1,
            phantom: PhantomData
        })],

        // No allocations
        allocations: vec![],
        rules: vec![],
    };

    let task = ExecutionTask::<TestEnv>::new(&script);


    match task {
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
            inputs: vec!["input 1".to_owned()], // This input exists, so that shouldn't cause a failure.
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

    let task = ExecutionTask::<TestEnv>::new(&script);


    match task {
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
