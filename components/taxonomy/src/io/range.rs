use io::types::*;

/// A comparison between two values.
///
/// # JSON
///
/// A range is an object with one field `{key: value}`.
///
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Range {
    /// Leq(x) accepts any value v such that v <= x.
    Leq(Value),

    /// Geq(x) accepts any value v such that v >= x.
    Geq(Value),

    /// BetweenEq {min, max} accepts any value v such that `min <= v`
    /// and `v <= max`. If `max < min`, it never accepts anything.
    BetweenEq { min: Value, max: Value },

    /// OutOfStrict {min, max} accepts any value v such that `v < min`
    /// or `max < v`
    OutOfStrict { min: Value, max: Value },

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
            BetweenEq { ref min, ref max } => min <= value && value <= max,
            OutOfStrict { ref min, ref max } => value < min || max < value,
            Eq(ref val) => value == val,
        }
    }
}
