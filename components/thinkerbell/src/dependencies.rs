#![allow(dead_code)]

use foxbox_taxonomy::api::API;

use serde::de::Deserialize;
/// APIs that we need to implement the code in module lang.


/*
pub trait ExecutableDevEnv: DevEnv {
    fn get_device_kind(&String) -> Option<Self::DeviceKind>;
    fn get_device(&String) -> Option<Self::Device>;
    fn get_input_capability(&String) -> Option<Self::InputCapability>;
    fn get_output_capability(&String) -> Option<Self::OutputCapability>;

    
    type Watcher: Watcher + Watcher<Device=Self::Device, InputCapability=Self::InputCapability>;
    fn get_watcher() -> Self::Watcher; // FIXME: Maybe this should only appear in a subtrait.
    fn send(&Self::Device, &Self::OutputCapability, &HashMap<String, Value>); // FIXME: Define errors    
}

/// An object that may be used to track state changes in devices.
pub trait Watcher {
    type Witness;
    type Device;
    type InputCapability;
    /// Watch a property of a device.
    fn add<F>(&mut self,
              device: &Self::Device,
              input: &Self::InputCapability,
              condition: &Range,
              cb: F) -> Self::Witness where F:Fn(Value) + Send;
}


*/


