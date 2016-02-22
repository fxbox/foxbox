use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer};

use std::marker::PhantomData;

/// Utility function. A variant of `map` that stops in case of error.
pub fn map<T, F, U, E>(vec: Vec<T>, cb: F) -> Result<Vec<U>, E> where F: Fn(T) -> Result<U, E> {
    let mut result = Vec::with_capacity(vec.len());
    for val in vec {
        result.push(try!(cb(val)));
    }
    Ok(result)
}


/// A variant of `PhantomData` that supports [De]serialization
#[derive(Clone, Debug, Default)]
pub struct Phantom<T> {
    pub phantom: PhantomData<T>
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
