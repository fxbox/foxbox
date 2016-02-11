#![allow(unused_variables)]
#![allow(dead_code)]


use std::cmp::{Eq, Ord, Ordering};
use std::sync::Arc;

extern crate rustc_serialize;

use self::rustc_serialize::json::Json;


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
