use std::cmp::PartialEq;
use std::collections::HashMap;
use std::hash::{ Hash, Hasher };
use std::marker::PhantomData;
use std::fmt;

use string_cache::Atom;

use serde::ser::{ Serialize, Serializer };
use serde::de::{ Deserialize, Deserializer };

/// A marker for a request that a expects a specific value.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Exactly<T> {
    /// No constraint.
    Always,

    /// Expect a specific value.
    Exactly(T),

    /// Never accept a constraint. This can happen, for instance, we have have
    /// attempted to `and` two conflicting `Exactly`
    Never,
}

impl<T> PartialEq for Exactly<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        use self::Exactly::*;
        match (self, other) {
            (&Always, &Always) => true,
            (&Never, &Never) => true,
            (&Exactly(ref a), &Exactly(ref b)) => a == b,
            _ => false
        }
    }
}


impl<T> Exactly<T> where T: PartialEq {
    /// Combine two constraints.
    pub fn and(self, other: Self) -> Self {
        use self::Exactly::*;
        match (self, other) {
            (Never, _) | (_, Never) => Never,
            (Always, x) | (x, Always) => x,
            (Exactly(x), Exactly(y)) =>
                if x == y {
                    Exactly(y)
                } else {
                    Never
                }
        }
    }

    pub fn is_empty(&self) -> bool {
        match *self {
            Exactly::Always => true,
            _ => false,
        }
    }

    pub fn matches(&self, value: &T) -> bool {
        match *self {
            Exactly::Always => true,
            Exactly::Exactly(ref id) => id == value,
            _ => false
        }
    }
}

impl<T> Default for Exactly<T> {
    fn default() -> Self {
        Exactly::Always
    }
}

/// A variant of `PhantomData` that supports [De]serialization
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct Phantom<T> {
    phantom: PhantomData<T>
}

impl<T> Default for Phantom<T> {
    fn default() -> Self {
        Self::new()
    }
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
            ().serialize(serializer)
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


/// A bunch of results coming from different sources.
pub type ResultMap<K, T, E> = HashMap<K, Result<T, E>>;

/// A bunch of instructions, going to different targets.
pub type TargetMap<K, T> = Vec<Targetted<K, T>>;

#[derive(Clone)]
pub struct Targetted<K, T> where K: Clone, T: Clone {
    pub select: Vec<K>,
    pub payload: T
}
impl<K, T> Default for Targetted<K, T> where T: Default + Clone, K: Clone {
    fn default() -> Self {
        Targetted {
            select: vec![],
            payload: T::default()
        }
    }
}
impl<K, T> Targetted<K, T> where K: Clone, T: Clone {
    pub fn new(select: Vec<K>, payload: T) -> Self {
        Targetted {
            select: select,
            payload: payload
        }
    }
}

/// A unique id for values of a given kind.
///
/// # Performance
///
/// This data structure is tuned for specific use cases and should be used accordingly.
///
/// - Using an instances of `Id` as a key in a `HashMap` or `HashSet` is much faster than
///   using a `String`.
/// - Comparing two instances of `Id` is much faster than comparing two `String`s.
/// - Instances of `Id` take very little memory.
/// - Cloning an instance of `Id` is relatively fast, but should be avoided if possible.
/// - Calling `Id::new`, on the other hand, is *very slow*. **Always prefer cloning to calling
///   `Id::new`**.
///
/// # (De)serialization
///
/// Serialized values of this type are represented by plain strings.
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
///
/// #[derive(Debug)]
/// struct UniqueId;
///
/// let my_id = foxbox_taxonomy::misc::util::Id::<UniqueId>::new("Unique Identifier");
///
/// assert_eq!(my_id.to_string(), "Unique Identifier");
///
/// let my_serialized_id = serde_json::to_string(&my_id).unwrap();
/// assert_eq!(my_serialized_id, "\"Unique Identifier\"");
///
/// let my_deserialized_id: foxbox_taxonomy::misc::util::Id<UniqueId> =
///     serde_json::from_str("\"Unique Identifier\"").unwrap();
/// assert_eq!(my_deserialized_id, my_id);
/// ```
#[derive(Debug, Clone)]
pub struct Id<T> {
    id: Atom,

    phantom: Phantom<T>,
}

impl<T> Id<T> {
    pub fn new(id: &str) -> Self {
        Id {
            id: Atom::from(id),
            phantom: Phantom::new(),
        }
    }

    pub fn as_atom(&self) -> &Atom {
        &self.id
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id.as_ref())
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
        self.id.as_ref().serialize(serializer)
    }
}

impl<T> Deserialize for Id<T> {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer {
        let deserialized_string = try!(String::deserialize(deserializer));

        Ok(Id {
            id: Atom::from(deserialized_string),
            phantom: Phantom::new(),
        })
    }
}


#[derive(Clone, Debug)]
pub struct MimeTypeId;


/// Helper function, to check that a type implements Sync.
pub fn is_sync<T: Sync>() {}

pub fn ptr_eq<T>(a: *const T, b: *const T) -> bool { a == b }

#[derive(Clone)]
pub enum Expects<T: Clone + ?Sized> {
    Requires(T),
    Optional(T),
    Nothing
}

pub trait Description {
    fn description(&self) -> String;
}

