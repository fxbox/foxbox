use std::marker::PhantomData;

use string_cache::Atom;

use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Error as DeserializationError, Type as DeserializationType};

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
            (Empty, x) | (x, Empty) => x,
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

/// A unique id for values of a given kind.
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
/// let my_id = foxbox_taxonomy::util::Id::<UniqueId>::new("Unique Identifier".to_owned());
///
/// let my_serialized_id = serde_json::to_string(&my_id).unwrap();
/// assert_eq!(my_serialized_id, "\"Unique Identifier\"");
///
/// let my_deserialized_id: foxbox_taxonomy::util::Id<UniqueId> =
///     serde_json::from_str("\"Unique Identifier\"").unwrap();
/// assert_eq!(my_deserialized_id, my_id);
/// ```
#[derive(Debug, Clone)]
pub struct Id<T> {
    id: Atom,

    phantom: Phantom<T>,
}
impl<T> Id<T> {
    pub fn new(id: String) -> Self {
        Id {
            id: Atom::from(id),
            phantom: Phantom::new(),
        }
    }

    pub fn as_atom(&self) -> &Atom {
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

/// A bunch of results grouped in an array of (key, result).
pub type ResultSet<I, T, E> = Vec<(I, Result<T, E>)>;


/// By default, the (de)serialization of trivial enums by Serde is surprising, e.g.
/// in JSON,  `enum Foo {A, B, C}` will produce `{"\"A\": []"}` for `A`, where `"\"A\""`
/// would be expected.
///
/// Implementing serialization is very simple, but deserialization is much more annoying.
/// This struct lets us implement simply the deserialization to a predictable and well-specified
/// list of strings.
///
/// # Example
///
/// ```
/// extern crate serde;
/// use serde::de::{Deserialize, Deserializer};
///
/// extern crate foxbox_taxonomy;
/// use foxbox_taxonomy::util::TrivialEnumVisitor;
///
/// enum Foo { A, B, C }
///
/// impl Deserialize for Foo {
///   fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error> where D: Deserializer {
///     deserializer.visit_string(TrivialEnumVisitor::new(|source| {
///       match source {
///         "A" => Ok(Foo::A),
///         "B" => Ok(Foo::B),
///         "C" => Ok(Foo::C),
///          _ => Err(())
///       }
///    }))
///   }
/// }
///
/// # fn main() {}
/// ```
pub struct TrivialEnumVisitor<T> where T: Deserialize {
    parser: Box<Fn(&str) -> Result<T, ()>>
}
impl<T> TrivialEnumVisitor<T> where T: Deserialize {
    pub fn new<F>(parser: F) -> Self
        where F: Fn(&str) -> Result<T, ()> + 'static {
            TrivialEnumVisitor {
                parser: Box::new(parser)
            }
        }
    fn parse<E>(&self, source: &str) -> Result<T, E>
        where E: DeserializationError
    {
        (self.parser)(source)
            .map_err(|()| E::unknown_field(&source.to_owned()))
    }
}

use serde::de::Visitor;
impl<T> Visitor for TrivialEnumVisitor<T> where T: Deserialize {
    type Value = T;
    fn visit_str<E>(&mut self, v: &str) -> Result<T, E>
        where E: DeserializationError,
    {
        self.parse(v)
    }

    fn visit_string<E>(&mut self, v: String) -> Result<T, E>
        where E: DeserializationError,
    {
        self.parse(&v)
    }

    fn visit_bytes<E>(&mut self, v: &[u8]) -> Result<T, E>
        where E: DeserializationError,
    {
        use std::str;
        match str::from_utf8(v) {
            Ok(s) => self.parse(s),
            Err(_) => Err(E::type_mismatch(DeserializationType::String)),
        }
    }

    fn visit_byte_buf<E>(&mut self, v: Vec<u8>) -> Result<T, E>
        where E: DeserializationError,
    {
        match String::from_utf8(v) {
            Ok(s) => self.parse(&s),
            Err(_) => Err(DeserializationError::type_mismatch(DeserializationType::String)),
        }
    }
}
