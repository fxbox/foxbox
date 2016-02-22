//! Manipulating values received from services and/or meant to be sent
//! to services.
use fxbox_taxonomy::values::{Value, Type};

use serde::ser::Serializer;
use serde::de::Deserializer;

#[derive(Clone, Deserialize, Serialize)]
/// A comparison between two values.
pub enum Range {
    /// Leq(x) accepts any value v such that v <= x.
    Leq(Value),

    /// Geq(x) accepts any value v such that v >= x.
    Geq(Value),

    /// BetweenEq {min, max} accepts any value v such that `min <= v`
    /// and `v <= max`. If `max < min`, it never accepts anything.
    BetweenEq {min:Value, max:Value},

    /// OutOfStrict {min, max} accepts any value v such that `v < min`
    /// or `max < v`
    OutOfStrict {min:Value, max:Value},


    /// Eq(x) accespts any value v such that v == x
    Eq(Value),
}

impl Range {
    /// Determine if a value is accepted by this range.
    pub fn contains(&self, value: &Value) -> bool {
        use self::Range::*;
        match *self {
            Leq(ref max) => value <= max,
            Geq(ref min) => value >= min,
            BetweenEq {ref min, ref max} => min <= value && value <= max,
            OutOfStrict {ref min, ref max} => value < min || max < value,
            Eq(ref val) => value == val,
        }
    }

    /// Get the type associated to this range.
    ///
    /// If this range has a `min` and a `max` with conflicting types,
    /// produce an error.
    pub fn get_type(&self) -> Result<Type, ()> {
        use self::Range::*;
        match *self {
            Leq(ref v) | Geq(ref v) | Eq(ref v) => Ok(v.get_type()),
            BetweenEq{ref min, ref max} | OutOfStrict{ref min, ref max} => {
                let min_typ = min.get_type();
                let max_typ = max.get_type();
                if min_typ == max_typ {
                    Ok(min_typ)
                } else {
                    Err(())
                }
            }
        }
    }
}
