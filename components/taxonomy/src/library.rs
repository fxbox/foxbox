//! Standardized components.

use io::parse::*;
use io::serialize::*;
use io::types::*;

use std::sync::Arc;

/// A value that may be "on" or "off".
#[derive(PartialEq, PartialOrd, Debug)]
pub struct IsOn(pub bool);
impl Into<Value> for IsOn {
    fn into(self) -> Value {
        Value {
            data: Arc::new(self)
        }
    }
}

/// (De)serialization format for `IsOn`.
pub struct IsOnFormat;
impl Format for IsOnFormat {
    fn description(&self) -> String {
        "isOn (true|false)".to_owned()
    }
    fn serialize(&self, value: &Value, _: &SerializeSupport) -> Result<JSON, SerializeError> {
        match value.cast::<IsOn>() {
            None => Err(SerializeError::ExpectedType(self.description())),
            Some(value) => {
                if value.0 {
                    Ok(JSON::Bool(true))
                } else {
                    Ok(JSON::Bool(false))
                }
            }
        }
    }
    fn deserialize(&self, path: Path, data: &JSON, _: &DeserializeSupport) -> Result<Value, ParseError> {
        match *data {
            JSON::Bool(ref b) => Ok(IsOn(*b).into()),
            JSON::String(ref s) => {
                if &*s == "on" {
                    Ok(IsOn(true).into())
                } else if &*s == "off" {
                    Ok(IsOn(false).into())
                } else {
                    Err(ParseError::TypeError {
                        expected: self.description(),
                        at: path.to_string(),
                        name: "IsOn".to_owned()
                    })
                }
            }
            _ => Err(ParseError::TypeError {
                expected: self.description(),
                at: path.to_string(),
                name: "IsOn".to_owned()
            })
        }
    }
}