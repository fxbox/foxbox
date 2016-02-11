#![allow(unused_variables)]
#![allow(dead_code)]

/// APIs that we need to implement the code in module lang.

use std::sync::Arc;

extern crate rustc_serialize;

use self::rustc_serialize::json::Json;

use std::cmp::{Eq, Ord, Ordering};


/// A description of a device, e.g. "a lightbulb".
#[derive(Clone)]
pub struct DeviceKind;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InputCapability;

#[derive(Clone)]
pub struct OutputCapability;

#[derive(Clone)]
pub struct Device;

impl Device {
    fn fetch(&self, cap: &InputCapability) -> Result<Value, ()> {
        panic!("Not implemented yet");
    }
}

#[derive(Clone)]
pub enum Range {
    /// Operations on numbers.

    /// Leq(x) accepts any value v such that v <= x.
    Leq(Number),

    /// Geq(x) accepts any value v such that v >= x.
    Geq(Number),

    /// BetweenEq {min, max} accepts any value v such that `min <= v`
    /// and `v <= max`. If `max < min`, it never accepts anything.
    BetweenEq {min:Number, max:Number},

    /// OutOfStrict {min, max} accepts any value v such that `v < min`
    /// or `max < v`
    OutOfStrict {min:Number, max:Number},


    /// Operations on strings

    /// `EqString(s) accepts any value v such that v == s`.
    EqString(String),

    /// Operations on bools

    /// `EqBool(s) accepts any value v such that v == s`.
    EqBool(bool),

    /// Operations on anything

    /// `Any` accepts all values.
    Any,
}

#[derive(Clone)]
pub struct Number {
    value: f64,
    physical_unit: (), // FIXME: Implement
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        panic!("Not implemented")
    }
}

impl PartialOrd<Number> for Number {
    fn partial_cmp(&self, other: &Number) -> Option<Ordering> {
        panic!("Not implemented")
    }
}

impl PartialEq<Number> for Number {
    fn eq(&self, other: &Number) -> bool {
        panic!("Not implemented")
    }
}

impl Eq for Number {
}

#[derive(Clone)]
pub enum Value {
    String(String),
    Bool(bool),
    Num(Number),
    Json(Json),
    Blob{data: Arc<Vec<u8>>, mime_type: String},
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
                  cb: F) -> Witness where F:FnOnce(Value){
        panic!("Not implemented");
    }
}

/// A structure used to stop watching a property once it is dropped.
pub struct Witness; // FIXME: Define
