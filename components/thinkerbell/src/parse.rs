//! Utilities for defining a JSON parser.


use std::rc::Rc;
use std::cell::RefCell;

use serde_json::error;
use serde::ser::{ Serialize, Serializer };
use serde_json::value::Value as JSON;
use serde::de::{ Deserialize, Deserializer, Error };

/// Utility function: Make sure that we have consumed all the fields of an object.
pub fn check_no_more_fields(path: Path, json: JSON) -> Result<(), ParseError> {
    if let JSON::Object(obj) = json {
        if obj.len() == 0 {
            Ok(())
        } else {
            Err(ParseError::unknown_fields(obj.keys().cloned().collect(), &path))
        }
    } else {
        Ok(())
    }
}

/// A path in the JSON tree. Used for displaying error messages.
pub struct Path {
    buf: Rc<RefCell<String>>,
    len: usize,
}
impl Path {
    /// Create an empty Path.
    pub fn new() -> Self {
        Path {
            buf: Rc::new(RefCell::new(String::new())),
            len: 0,
        }
    }

    /// Push a suffix after a path.
    pub fn push(&self, suffix: &str) -> Self {
        let buf = self.buf.clone();
        let len;
        {
            let mut str = buf.borrow_mut();
            str.push_str(" > ");
            str.push_str(suffix);
            len = str.len();
        }
        Path {
            buf: buf,
            len: len,
        }
    }
    pub fn to_string(&self) -> String {
        let mut str = self.buf.borrow().clone();
        str.truncate(self.len);
        str
    }
}
impl Drop for Path {
    fn drop(&mut self) {
        let mut str = self.buf.borrow_mut();
        str.truncate(self.len);
    }
}

/// An error during parsing.
#[derive(Debug)]
pub enum ParseError {
    JSON(JSONError),
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
    pub fn json(error: error::Error) -> Self {
        ParseError::JSON(JSONError(error))
    }
}

#[derive(Debug)]
pub struct JSONError(error::Error);

impl Serialize for JSONError {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        serializer.visit_str(&format!("{:?}", self))
    }
}

pub trait Parse<T: Sized> {
    fn parse(path: Path, source: JSON) -> Result<T, ParseError>;
    fn take(path: Path, source: &mut JSON, field_name: &str) -> Result<T, ParseError> {
        if let JSON::Object(ref mut obj) = *source {
            if let Some(v) = obj.remove(field_name) {
                return Self::parse(path, v)
            }
        }
        Err(ParseError::missing_field(field_name, &path))
    }
    fn take_vec(path: Path, source: &mut JSON, field_name: &str) -> Result<Vec<T>, ParseError>
    {
        if let JSON::Object(ref mut obj) = *source {
            if let Some(ref mut json) = obj.remove(field_name) {
                if let JSON::Array(ref mut vec) = *json {
                    let mut result = Vec::with_capacity(vec.len());
                    for (json, i) in vec.drain(..).zip(0..) {
                        let path = path.push(&format!("{}#{}", field_name, i));
                        let parsed = try!(Self::parse(path, json));
                        result.push(parsed);
                    }
                    return Ok(result)
                }
                return Err(ParseError::type_error(field_name, &path, "array"))
            }
        }
        Err(ParseError::missing_field(field_name, &path))
    }
}

impl<T> Parse<T> for T where T: Deserialize {
    fn parse(_: Path, source: JSON) -> Result<T, ParseError> {
        use serde_json;
        serde_json::from_value(source).map_err(ParseError::json)
    }
}
