#![allow(dead_code)]

use values::{Value, Range};

/// APIs that we need to implement the code in module lang.

/// The environment in which the code is meant to be executed.  This
/// can typically be instantiated either with actual bindings to
/// devices, or with a unit-testing framework.
pub trait DeviceAccess: Send {
    type DeviceKind: Clone + Send;
    type Device: Clone + Send;
    type InputCapability: Clone + Send;
    type OutputCapability: Clone + Send;
    type Watcher: Watcher + Watcher<Device=Self::Device, InputCapability=Self::InputCapability>;

    fn get_watcher() -> Self::Watcher; // FIXME: Maybe this should only appear in a subtrait.
    fn get_device_kind(&String) -> Option<Self::DeviceKind>;
    fn get_device(&String) -> Option<Self::Device>;
    fn get_input_capability(&String) -> Option<Self::InputCapability>;
    fn get_output_capability(&String) -> Option<Self::OutputCapability>;
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





