use std::sync::mpsc::Sender;
use std::collections::BTreeMap;

/// APIs that we need to implement the code in module lang.

/// A description of a device, e.g. "a lightbulb".
pub struct DeviceKind;

pub struct InputCapability;

pub struct OutputCapability;

pub struct Device;

/// The path to an API used to access a specific feature of a specific
/// resource.
pub struct Path; // FIXME: Define

pub struct Range;

impl Range {
    fn any() -> Range {
        panic!("Not implemented")
    }
    fn boundary() -> Range {
        panic!("Not implemented")
    }
}

trait Watcher {
  /// Create a new watcher.
  fn new() -> Watcher;

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
  fn add<T>(&mut self, &Device, &InputCapability, &Range, &Sender<BTreeMap<String, String>>) -> Witness;
}

/// A structure used to stop watching a property once it is dropped.
pub struct Witness; // FIXME: Define
