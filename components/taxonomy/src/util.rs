use serde::ser::Serializer;
use serde::de::Deserializer;


/// A marker for a request that a expects a specific value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Exactly<Id> {
    /// No constraint.
    Empty,

    /// Expect a specific value.
    Exactly(Id),

    /// Two conflicting constraints (or more) have been put on the value.
    Conflict,
}

impl<T> Exactly<T> where T: PartialEq {
    /// Combine two constraints.
    pub fn and(self, other: Self) -> Self {
        use self::Exactly::*;
        match (self, other) {
            (Conflict, _) | (_, Conflict) => Conflict,
            (Empty, x@_) | (x@_, Empty) => x,
            (Exactly(x), Exactly(y)) =>
                if x == y {
                    Exactly(y)
                } else {
                    Conflict
                }
        }
    }

    pub fn is_empty(&self) -> bool {
        match *self {
            Exactly::Empty => true,
            _ => false,
        }
    }
}

impl<T> Default for Exactly<T> {
    fn default() -> Self {
        Exactly::Empty
    }
}
