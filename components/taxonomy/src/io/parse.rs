//! Utilities for defining a JSON parser.

use io::serialize::*;
use misc::util::{ Exactly, Id };

use std::cell::RefCell;
use std::error::Error as StdError;
use std::fmt::{ Display, Debug, Error as FmtError, Formatter };
use std::rc::Rc;
use std::sync::Arc;

use serde::de::{ Deserialize, Deserializer, Error };
use serde::ser::{ Serialize, Serializer };
use serde_json;
use serde_json::error;
pub use serde_json::value::Value as JSON;

/// Utility function: Make sure that we have consumed all the fields of an object.
pub fn check_fields(path: Path, json: &JSON) -> Result<(), ParseError> {
    if let JSON::Object(ref obj) = *json {
        if obj.is_empty() {
            Ok(())
        } else {
            Err(ParseError::unknown_fields(obj.keys().cloned().collect(), &path))
        }
    } else {
        Ok(())
    }
}

/// A container holding data that may be necessary for deserialization.
///
/// For instance, if a HTTP client has sent a multipart request,
/// the `DeserializeSupport` will contain all the parts.
pub trait DeserializeSupport: Send + Sync {
    /// Get a binary represented by a given index.
    fn get_binary(&self, index: usize) -> Result<&[u8], ParseError>;
}

/// A trivial implementation of `DeserializeSupport` that offers no data.
/// Useful mostly for testing.
pub struct EmptyDeserializeSupportForTests;
impl DeserializeSupport for EmptyDeserializeSupportForTests {
    fn get_binary(&self, _: usize) -> Result<&[u8], ParseError> {
        panic!("This should never be called");
    }
}

/// A path in the JSON tree. Used for displaying error messages.
#[derive(Clone, Debug)]
pub struct Path {
    buf: Rc<RefCell<String>>,
    len: usize,
}
impl Default for Path {
    fn default() -> Self {
        Path::new()
    }
}
impl Path {
    /// Create an empty Path.
    pub fn new() -> Self {
        Path {
            buf: Rc::new(RefCell::new(String::new())),
            len: 0,
        }
    }

    pub fn named(name: &str) -> Self {
        Path {
            buf: Rc::new(RefCell::new(name.to_owned())),
            len: name.len()
        }
    }

    /// Push a suffix after a path.
    pub fn push_str<F, T>(&self, suffix: &str, cb: F) -> T
        where F: FnOnce(Path) -> T
    {
        let buf = self.buf.clone();
        let len;
        {
            let mut str = buf.borrow_mut();
            str.push_str(suffix);
            len = str.len();
        }
        let path = Path {
            buf: buf,
            len: len,
        };
        let result = cb(path);
        {
            let mut str = self.buf.borrow_mut();
            str.truncate(self.len)
        }
        result
    }

    pub fn push_index<F, T>(&self, index: usize, cb: F) -> T
        where F: FnOnce(Path) -> T
    {
        self.push_str(&format!("[{}]", index), cb)
    }

    pub fn push<F, T>(&self, suffix: &str, cb: F) -> T
        where F: FnOnce(Path) -> T
    {
        self.push_str(&format!(".{}", suffix), cb)
    }

    pub fn to_string(&self) -> String {
        let mut str = self.buf.borrow().clone();
        str.truncate(self.len);
        str
    }
}

/// An error during parsing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ParseError {
    JSON(JSONError),
    NoDeserializer(String),
    NoBinary(usize),
    MissingField {
        name: String,
        at: String,
    },
    UnknownFields {
        names: Vec<String>,
        at: String,
    },
    TypeError {
        name: String,
        at: String,
        expected: String,
    },
    EmptyObject {
        at: String
    },
    UnknownConstant {
        at: String,
        constant: String,
    }
}
impl ToJSON for ParseError {
    fn to_json(&self, support: &SerializeSupport) -> JSON {
        vec![("ParseError", format!("{:?}", self))].to_json(support)
    }
}


impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FmtError> {
        (self as &Debug).fmt(formatter)
    }
}
impl StdError for ParseError {
    fn description(&self) -> &str {
        "Error while parsing to JSON"
    }
}

impl ParseError {
    pub fn missing_field(name: &str, at: &Path) -> Self {
        ParseError::MissingField {
            name: name.to_owned(),
            at: at.to_string(),
        }
    }
    pub fn type_error(name: &str, at: &Path, expected: &str) -> Self {
        ParseError::TypeError {
            name: name.to_owned(),
            at: at.to_string(),
            expected: expected.to_owned()
        }
    }
    pub fn unknown_fields(names: Vec<String>, at: &Path) -> Self {
        ParseError::UnknownFields {
            names: names,
            at: at.to_string()
        }
    }
    pub fn unknown_constant(value: &str, at: &Path) -> Self {
        ParseError::UnknownConstant {
            constant: value.to_owned(),
            at: at.to_string()
        }
    }
    pub fn empty_object(at: &Path) -> Self {
        ParseError::EmptyObject {
            at: at.to_string(),
        }
    }
    pub fn json(error: error::Error) -> Self {
        ParseError::JSON(JSONError(format!("{:?}", error)))
    }
}

#[derive(Clone, Debug)]
pub struct JSONError(String);

impl Deserialize for JSONError {
    fn deserialize<D>(_: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        unimplemented!()
    }
}

impl Serialize for JSONError {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        serializer.serialize_str(&format!("{:?}", self))
    }
}

/// An object that knows how to parse values from JSON into type T.
///
/// The JSON object is expected to be consumed along the way. A successful parse will
/// typically leave an empty JSON object.
pub trait Parser<T: Sized> {
    fn description() -> String;
    fn from_str(source: &str, support: &DeserializeSupport) -> Result<T, ParseError> {
        match serde_json::from_str(source) {
            Err(err) => Err(ParseError::json(err)),
            Ok(json) => Self::parse(Path::new(), &json, support)
        }
    }

    /// Parse a single value from JSON, consuming as much as necessary from JSON.
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<T, ParseError>;

    /// Parse a field from JSON, consuming it.
    fn take(path: Path, source: &JSON, field_name: &str, support: &DeserializeSupport) -> Result<T, ParseError> {
        match Self::take_opt(path.clone(), source, field_name, support) {
            Some(result) => result,
            None => Err(ParseError::missing_field(field_name, &path))
        }
    }

    /// Parse a field from JSON, consuming it.
    fn take_opt(path: Path, source: &JSON, field_name: &str, support: &DeserializeSupport) -> Option<Result<T, ParseError>> {
        if let JSON::Object(ref obj) = *source {
            if let Some(v) = obj.get(field_name) {
                Some(Self::parse(path, v, support))
            } else {
                None
            }
        } else {
            Some(Err(ParseError::type_error(field_name, &path, "object")))
        }
    }

    /// Parse a field containing an array from JSON, consuming the field.
    fn take_vec_opt(path: Path, source: &JSON, field_name: &str, support: &DeserializeSupport) -> Option<Result<Vec<T>, ParseError>>
    {
        if let JSON::Object(ref obj) = *source {
            if let Some(json) = obj.get(field_name) {
                if let JSON::Array(ref vec) = *json {
                    let mut result = Vec::with_capacity(vec.len());
                    for (json, i) in vec.iter().zip(0..) {
                        match path.push_index(i,
                            |path| Self::parse(path, json, support)
                        ) {
                            Err(error) => return Some(Err(error)),
                            Ok(parsed) => result.push(parsed)
                        }
                    }
                    Some(Ok(result))
                } else {
                    Some(Err(ParseError::type_error(field_name, &path, "array")))
                }
            } else {
                None
            }
        } else {
            Some(Err(ParseError::missing_field(field_name, &path)))
        }
    }

    fn take_vec(path: Path, source: &JSON, field_name: &str, support: &DeserializeSupport) -> Result<Vec<T>, ParseError> {
        match Self::take_vec_opt(path.clone(), source, field_name, support) {
            Some(result) => result,
            None => Err(ParseError::missing_field(field_name, &path))
        }
    }
}

impl Parser<f64> for f64 {
    fn description() -> String {
        "Number".to_owned()
    }
    fn parse(path: Path, source: &JSON, _: &DeserializeSupport) -> Result<Self, ParseError> {
        match *source {
            JSON::I64(val) => Ok(val as f64),
            JSON::F64(val) => Ok(val),
            JSON::U64(val) => Ok(val as f64),
            _ => Err(ParseError::type_error("as float", &path, "number"))
        }
    }
}

impl Parser<bool> for bool {
    fn description() -> String {
        "bool".to_owned()
    }
    fn parse(path: Path, source: &JSON, _: &DeserializeSupport) -> Result<Self, ParseError> {
        match *source {
            JSON::Bool(ref b) => Ok(*b),
            JSON::U64(0) | JSON::I64(0) => Ok(false),
            JSON::U64(1) | JSON::I64(1) => Ok(true),
            JSON::String(ref str) if str == "true" => Ok(true),
            JSON::String(ref str) if str == "false" => Ok(false),
            _ => Err(ParseError::type_error("as bool", &path, "boolean"))
        }
    }
}

impl<T> Parser<Vec<T>> for Vec<T> where T: Parser<T> {
    fn description() -> String {
        format!("Array<{}>", T::description())
    }
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        // Otherwise, parse as an actual array.
        match *source {
            JSON::Array(ref array) => {
                let mut result = Vec::with_capacity(array.len());
                for (source, i) in array.iter().zip(0..) {
                    let value = try!(path.push_index(i, |path| T::parse(path, source, support)));
                    result.push(value)
                }
                Ok(result)
            }
            JSON::Null => {
                // Accept `null` as an empty array.
                Ok(vec![])
            }
            _ => {
                // Attempt to promote the value to an array.
                let single = try!(path.push_str("", |path| T::parse(path, source, support)));
                Ok(vec![single])
            }
        }
    }
}

/*
impl<T, U> Parser<(T, U)> for (T, U) where T: Parser<T>, U: Parser<U> {
    fn description() -> String {
        format!("({}, {})", T::description(), U::description())
    }
    fn parse(path: Path, source: &JSON) -> Result<Self, ParseError> {
        match *source {
            JSON::Array(ref mut array) if array.len() == 2 => {
                let mut right = array.pop().unwrap(); // We just checked that length == 2
                let mut left = array.pop().unwrap(); // We just checked that length == 2
                let left_parsed = try!(path.push(&T::description() as &str, |path| {T::parse(path, &mut left)}));
                let right_parsed = try!(path.push(&U::description() as &str, |path| {U::parse(path, &mut right)}));
                Ok((left_parsed, right_parsed))
            }
            _ => Err(ParseError::type_error("pair of values", &path, "array"))
        }
    }
}
*/
impl Parser<String> for String {
    fn description() -> String {
        "String".to_owned()
    }
    fn parse(path: Path, source: &JSON, _: &DeserializeSupport) -> Result<Self, ParseError> {
        match source.as_string() {
            Some(str) => Ok(str.to_owned()),
            None => Err(ParseError::type_error("string", &path, "string"))
        }
    }
}


impl<T> Parser<Arc<T>> for Arc<T> where T: Parser<T> {
    fn description() -> String {
        T::description()
    }
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        Ok(Arc::new(try!(T::parse(path, source, support))))
    }
}


impl<T> Parser<Exactly<T>> for Exactly<T> where T: Parser<T> {
    fn description() -> String {
        T::description()
    }
    /// Parse a single value from JSON, consuming as much as necessary from JSON.
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        if let JSON::Null = *source {
            Ok(Exactly::Always)
        } else {
            T::parse(path, source, support).map(Exactly::Exactly)
        }
    }
}

impl<T> Parser<Id<T>> for Id<T> {
    fn description() -> String {
        "Id".to_owned()
    }
    /// Parse a single value from JSON, consuming as much as necessary from JSON.
    fn parse(path: Path, source: &JSON, _: &DeserializeSupport) -> Result<Self, ParseError> {
        if let JSON::String(ref string) = *source {
            Ok(Id::new(string))
        } else {
            Err(ParseError::type_error("id", &path, "string"))
        }
    }
}

impl Parser<JSON> for JSON {
    fn description() -> String {
        "JSON".to_owned()
    }
    /// Parse a single value from JSON, consuming as much as necessary from JSON.
    fn parse(_: Path, source: &JSON, _: &DeserializeSupport) -> Result<Self, ParseError> {
        Ok(source.clone())
    }
}

/*
impl<T> Parser<T> for T where T: Deserialize {
    fn parse(_: Path, source: &JSON) -> Result<T, ParseError> {
        use serde_json;
        serde_json::from_value(source.clone()).map_err(ParseError::json)
    }
}
*/