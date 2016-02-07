#![allow(unused_variables)]
#![allow(dead_code)]

extern crate rustc_serialize;

use self::rustc_serialize::json::Json;

/// APIs that we need to implement the code in module lang.

/// A description of a device, e.g. "a lightbulb".
#[derive(Clone)]
pub struct DeviceKind;

#[derive(Clone)]
pub struct InputCapability;

#[derive(Clone)]
pub struct OutputCapability;

pub struct Device;

pub struct Range;

impl Range {
    pub fn any() -> Range {
        panic!("Not implemented")
    }
    pub fn boundary() -> Range {
        panic!("Not implemented")
    }
}

pub struct Watcher;

impl Watcher {
  /// Create a new watcher.
    pub fn new() -> Watcher {
        panic!("Not implemented");
    }

  /// Watch a property of a device.
  ///
  /// If the device is smart enough, 
  ///
  /// # Example
  ///
  /// let witness = watcher.add(&the_terminator,
  ///                           &InputCapability::DetectSarahConnor,
  ///                           &Range::any(),
  ///                           &tx);
  ///
  /// Until `witness` is dropped, whenever property
  /// `DetectSarahConnor` of `the_terminator` changes, the watcher
  /// will send a message on `tx`.
  ///
  /// let witness_2 = watcher.add(&the_terminator,
  ///                             &InputCapability::Ammo,
  ///                             &Range::boundary(100),
  ///                             &tx_2);
  ///
  /// Until `witness_2` is dropped, whenever property `Ammo` of
  /// `the_terminator` goes above/beyond 100, the watcher will send a
    /// message on `tx_2`.
    #[allow(unused_variables)]
    #[allow(dead_code)]
    pub fn add<F>(&mut self,
                  device: &Device,
                  input: &InputCapability,
                  range: &Range,
                  cb: F) -> Witness where F:FnOnce(Json){
        panic!("Not implemented");
    }
}

/// A structure used to stop watching a property once it is dropped.
pub struct Witness; // FIXME: Define
