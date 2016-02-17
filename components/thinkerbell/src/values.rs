#![allow(unused_variables)]
#![allow(dead_code)]


use std::cmp::{Eq, Ordering};
use std::sync::Arc;

extern crate serde_json;

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

#[derive(Clone, Debug)]
pub struct Number {
    value: f64,
    physical_unit: (), // FIXME: Implement
}
impl Number {
    pub fn new(value: f64, physical_unit: ()) -> Self {
        Number {
            value: value,
            physical_unit: physical_unit
        }
    }
}

impl PartialOrd<Number> for Number {
    fn partial_cmp(&self, other: &Number) -> Option<Ordering> {
        assert!(self.physical_unit == other.physical_unit, "Conversion of units is not implemented");
        self.value.partial_cmp(&other.value)
    }
}

impl PartialEq<Number> for Number {
    fn eq(&self, other: &Number) -> bool {
        assert!(self.physical_unit == other.physical_unit, "Conversion of units is not implemented");
        self.value.eq(&other.value)
    }
}

impl Eq for Number {
}

#[derive(Clone, Debug)]
pub enum Value {
    String(String),
    Bool(bool),
    Num(Number),
    Vec(Vec<Value>),
    Json(self::serde_json::Value),
    Blob{data: Arc<Vec<u8>>, mime_type: String},
}
