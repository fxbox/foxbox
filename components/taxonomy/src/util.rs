use std::marker::PhantomData;

use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer};

use std::cmp::PartialEq;
use std::hash::{Hash, Hasher};

/// A marker for a request that a expects a specific value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Exactly<T> {
    /// No constraint.
    Empty,

    /// Expect a specific value.
    Exactly(T),

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

    pub fn matches(&self, value: &T) -> bool {
        match *self {
            Exactly::Exactly(ref id) => id == value,
            Exactly::Empty => true,
            _ => false
        }
    }
}

impl<T> Default for Exactly<T> {
    fn default() -> Self {
        Exactly::Empty
    }
}



/// A variant of `PhantomData` that supports [De]serialization
#[derive(Clone, Debug, Default, PartialEq, Hash, Eq)]
pub struct Phantom<T> {
    phantom: PhantomData<T>
}

impl<T> Phantom<T> {
    pub fn new() -> Self {
        Phantom {
            phantom: PhantomData
        }
    }
}

impl<T> Serialize for Phantom<T> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        serializer.visit_unit()
    }
}
impl<T> Deserialize for Phantom<T> {
    fn deserialize<D>(_: &mut D) -> Result<Self, D::Error>
        where D: Deserializer {
        // Nothing to consume
        Ok(Phantom {
            phantom: PhantomData
        })
    }
}

/// A unique id for values of a given kind.
#[derive(Debug, Clone)]
pub struct Id<T> {
    id: String,

    phantom: Phantom<T>
}
impl<T> Id<T> {
    pub fn new(id: String) -> Self {
        Id {
            id: id,
            phantom: Phantom::new()
        }
    }

    pub fn as_string(&self) -> &String {
        &self.id
    }
}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}
impl<T> Eq for Id<T> {
}
impl<T> Hash for Id<T> {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.id.hash(state)
    }
}
impl<T> Serialize for Id<T> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        serializer.visit_str(&self.id)
    }
}
impl<T> Deserialize for Id<T> {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer {
        Ok(Id {
            id: try!(String::deserialize(deserializer)),
            phantom: Phantom::new()
        })
    }
}
