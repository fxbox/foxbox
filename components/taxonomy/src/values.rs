//!
//! Values manipulated by endpoints
//!

use std::time::Duration;

extern crate chrono;
use self::chrono::{DateTime, Local};

extern crate serde_json;

///
/// The type of values manipulated by endpoints.
///
#[derive(Debug, Clone)]
pub enum Type {
    ///
    /// # Trivial values
    ///

    /// An empty value. Used for instance to inform that a countdown
    /// has reached 0 or that a device is ready.
    Unit,

    /// A boolean. Used for instance for on-off switches, presence
    /// detectors, etc.
    Bool,

    ///
    /// # Time
    ///

    /// A duration. Used for instance in countdowns.
    Duration,

    /// A precise timestamp. Used for instance to determine when an
    /// event has taken place.
    TimeStamp,

    Temperature,

    ///
    /// ...
    ///
    Color,
    Json,
    Binary,
}

/// A temperature. Internal representation may be either Fahrenheit or
/// Celcius. The FoxBox adapters are expected to perform conversions
/// to the format requested by their devices.
#[derive(Debug, Clone)]
pub enum Temperature {
    /// Fahrenheit
    F(f64),
    /// Celcius
    C(f64),
}

impl Temperature {
    /// Get a temperature in Fahrenheit.
    pub fn as_f(&self) -> f64 {
        unimplemented!();
    }

    /// Get a temperature in Celcius.
    pub fn as_c(&self) -> f64 {
        unimplemented!();
    }
}

/// A color. Internal representation may vary. The FoxBox adapters are
/// expected to perform conversions to the format requested by their
/// device.
#[derive(Debug, Clone)]
pub enum Color {
    RGBA(f64, f64, f64, f64, f64)
}


/// An actual value that may be transmitted from/to a Service.
#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Bool(bool),
    Duration(Duration),
    TimeStamp(chrono::DateTime<Local>),
    Temperature(Temperature),
    Color(Color),

    // FIXME: Add more as we identify needs
    Json(serde_json::value::Value),
    Binary {data: Vec<u8>, mimetype: String}
}

impl Value {
    pub fn get_type(&self) -> Type {
        match *self {
            Value::Unit => Type::Unit,
            Value::Bool(_) => Type::Bool,
            Value::Duration(_) => Type::Duration,
            Value::TimeStamp(_) => Type::TimeStamp,
            Value::Temperature(_) => Type::Temperature,
            Value::Color(_) => Type::Color,
            Value::Json(_) => Type::Json,
            Value::Binary{..} => Type::Binary,
        }
    }
}
