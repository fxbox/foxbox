//! Utilities for serializing data to JSON.
use misc::util::*;

use std::collections::{ BTreeMap, HashMap, HashSet };
use std::hash::Hash;

use serde_json::value::Value as JSON;

/// A container holding data that may be necessary for serialization.
///
/// For instance, when responding to a HTTP client, JSON is a very poor
/// format for sending binary data. Rather, binary values should be stored
/// in the `SerializeSupport`. The server will pick an appropriate format
/// for actual transmission to the client.
pub trait SerializeSupport: Send + Sync {
    fn add_binary(&mut self, mimetype: Id<MimeTypeId>, binary: &[u8]) -> JSON;
}

pub struct EmptySerializeSupportForTests;
impl SerializeSupport for EmptySerializeSupportForTests {
    fn add_binary(&mut self, _: Id<MimeTypeId>, _: &[u8]) -> JSON {
        panic!("This should never be called");
    }
}

/// An imnplementation of `MultiPart` that stores everything in a `HashMap` and returns
/// JSON objects `"{part: i}"`, where `i` is the (integer) key in the `HashMap`.
#[derive(Default)]
pub struct MultiPart {
    pub buf: Vec<(Id<MimeTypeId>, Vec<u8>)>,
}
impl MultiPart {
    pub fn new() -> Self {
        MultiPart {
            buf: Vec::new(),
        }
    }
}
impl SerializeSupport for MultiPart {
    fn add_binary(&mut self, mimetype: Id<MimeTypeId>, binary: &[u8]) -> JSON {
        let mut vec = Vec::with_capacity(binary.len());
        vec.extend_from_slice(binary);
        self.buf.push((mimetype, vec));
        let mut map = BTreeMap::new();
        map.insert("part".to_owned(), JSON::U64(self.buf.len() as u64));
        JSON::Object(map)
    }
}

pub trait ToJSON {
    fn to_json(&self, parts: &SerializeSupport) -> JSON;
}

impl ToJSON for String {
    fn to_json(&self, _: &SerializeSupport) -> JSON {
        JSON::String(self.clone())
    }
}

impl ToJSON for bool {
    fn to_json(&self, _: &SerializeSupport) -> JSON {
        JSON::Bool(*self)
    }
}

impl ToJSON for f64 {
    fn to_json(&self, _: &SerializeSupport) -> JSON {
        JSON::F64(*self)
    }
}

impl ToJSON for usize {
    fn to_json(&self, _: &SerializeSupport) -> JSON {
        JSON::U64(*self as u64)
    }
}

impl ToJSON for JSON {
    fn to_json(&self, _: &SerializeSupport) -> JSON {
        self.clone()
    }
}

impl<T> ToJSON for HashSet<T> where T: ToJSON + Eq + Hash {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        JSON::Array((*self).iter().map(|x| x.to_json(parts)).collect())
    }
}

impl<T> ToJSON for HashMap<String, T> where T: ToJSON {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        JSON::Object(self.iter().map(|(k, v)| (k.clone(), T::to_json(v, parts))).collect())
    }
}

impl<T> ToJSON for Vec<T> where T: ToJSON {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        JSON::Array(self.iter().map(|x| x.to_json(parts)).collect())
    }
}

impl<'a, T> ToJSON for Vec<(&'a str, T)> where T: ToJSON {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        JSON::Object(self.iter().map(|&(ref k, ref v)| {
            ((*k).to_owned(), v.to_json(parts))
        }).collect())
    }
}

impl <'a> ToJSON for &'a str {
    fn to_json(&self, _: &SerializeSupport) -> JSON {
        JSON::String((*self).to_owned())
    }
}

impl<'a, T> ToJSON for &'a T where T: ToJSON {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        (**self).to_json(parts)
    }
}

impl<K, T, V> ToJSON for HashMap<Id<K>, Result<T, V>> where T: ToJSON, V: ToJSON {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        JSON::Object(self.iter().map(|(k, result)| {
            let k = k.to_string();
            let result = match *result {
                Ok(ref ok) => ok.to_json(parts),
                Err(ref err) => vec![("Error", err)].to_json(parts)
            };
            (k, result)
        }).collect())
    }
}

impl<T> ToJSON for Option<T> where T: ToJSON {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        match *self {
            None => JSON::Null,
            Some(ref result) => result.to_json(parts)
        }
    }
}

impl ToJSON for () {
    fn to_json(&self, _: &SerializeSupport) -> JSON {
        JSON::Null
    }
}

impl<T> ToJSON for Id<T> {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        self.to_string().to_json(parts)
    }
}

impl<T, U> ToJSON for HashMap<Id<U>, T> where T: ToJSON {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        JSON::Object(self.iter().map(|(k, v)| (k.to_string(), v.to_json(parts))).collect())
    }
}

/*
impl<T, R, E> ToJSON for ResultMap<T, R, E> where R: ToJSON, E: ToJSON {
    fn to_json(&self, support: &SerializeSupport) -> JSON {
        JSON::Object(self.iter().map(|(k, v)| {
            let payload = match v {
                Ok(ok) => ok.to_json(support),
                Err(err) => vec![("Error", err.to_json(support))].to_json(support)
            };
            (k.to_string(), payload)
        }).collect())
    }
}
*/