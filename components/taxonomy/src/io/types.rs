use io::parse::*;
use io::serialize::*;

use misc::util::Description;

use std::cmp::*;
use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

pub use mopa::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SerializeError {
    NoSerializer(String),
    ExpectedType(String),
}
impl ToJSON for SerializeError {
    fn to_json(&self, support: &SerializeSupport) -> JSON {
        vec![("SerializeError", format!("{:?}", self))].to_json(support)
    }
}



pub trait Format: Send + Sync + 'static {
    fn description(&self) -> String;
    fn serialize(&self, value: &Value, support: &SerializeSupport) -> Result<JSON, SerializeError>;
    fn deserialize(&self, path: Path, value: &JSON, support: &DeserializeSupport) -> Result<Value, ParseError>;
}
impl fmt::Debug for Format {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.pad(&format!("Format: {}", self.description()))
    }
}
pub trait Data: Any + Debug + Send + Sync {
    fn dynamic_eq(&self, other: &Data) -> bool;
    fn dynamic_cmp(&self, other: &Data) -> Option<Ordering>;
}
impl<T: Any + Debug + Send + Sync + PartialEq + PartialOrd> Data for T {
    fn dynamic_eq(&self, other: &Data) -> bool {
        match other.downcast_ref::<T>() {
            None => false,
            Some(v) => self.eq(v)
        }
    }

    fn dynamic_cmp(&self, other: &Data) -> Option<Ordering> {
        match other.downcast_ref::<T>() {
            None => None,
            Some(v) => self.partial_cmp(v)
        }
    }
}
impl PartialEq for Data {
    fn eq(&self, other: &Data) -> bool {
        self.dynamic_eq(other)
    }
}
impl PartialOrd for Data {
    fn partial_cmp(&self, other: &Data) -> Option<Ordering> {
        self.dynamic_cmp(other)
    }
}


mopafy!(Data);


#[derive(Clone, Debug)]
pub struct Value {
    pub data: Arc<Data>,
}
impl Value {
    pub fn new<T>(data: T) -> Self where T: Data {
        Value {
            data: Arc::new(data)
        }
    }
}
impl Value {
    pub fn cast<T>(&self) -> Option<&T> where T: Data {
        self.data.downcast_ref::<T>()
    }
}
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        (&*self.data).eq(&*other.data)
    }
}
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        (&*self.data).partial_cmp(&*other.data)
    }
}

pub trait FromValue {
    fn encode(&self, format: &Format, support: &SerializeSupport) -> Result<JSON, SerializeError>;
}
pub trait AsValue: Send + Sync {
    fn decode(&self, format: &Format, support: &DeserializeSupport) -> Result<Value, ParseError>;
}
impl AsValue for JSON {
    fn decode(&self, format: &Format, support: &DeserializeSupport) -> Result<Value, ParseError> {
        format.deserialize(Path::new(), self, support)
    }
}
impl AsValue for Value {
    fn decode(&self, _: &Format, _: &DeserializeSupport) -> Result<Value, ParseError> {
        Ok(self.clone())
    }
}
impl Format for String {
    fn description(&self) -> String {
        "String".to_owned()
    }
    fn deserialize(&self, path: Path, data: &JSON, _: &DeserializeSupport) -> Result<Value, ParseError> {
        match *data {
            JSON::String(ref s) => Ok(Value { data: Arc::new(s.clone()) }),
            _ => Err(ParseError::type_error("String", &path, "string"))
        }
    }
    fn serialize(&self, value: &Value, _: &SerializeSupport) -> Result<JSON, SerializeError> {
        match value.data.downcast_ref::<String>() {
            None => Err(SerializeError::ExpectedType(self.description())),
            Some(s) => {
                Ok(JSON::String(s.clone()))
            }
        }
    }
}
