extern crate thinkerbell;

use thinkerbell::dependencies::{DeviceAccess, Watcher};
use thinkerbell::values::{Value, Range};
use thinkerbell::lang::{ExecutionTask, UncheckedCtx, UncheckedEnv, Script};

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
