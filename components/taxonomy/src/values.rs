//!
//! Values manipulated by endpoints
//!

use std::time::Duration;
use std::cmp::{PartialOrd, Ordering};

extern crate chrono;
use self::chrono::{DateTime, Local};

extern crate serde_json;

///
/// The type of values manipulated by endpoints.
///
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
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
    String, 
    ///
    /// ...
    ///
    Color,
    Json,
    Binary,
    ExtNumeric,
}

/// A temperature. Internal representation may be either Fahrenheit or
/// Celcius. The FoxBox adapters are expected to perform conversions
/// to the format requested by their devices.
#[derive(Debug, Clone, PartialEq)]
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

impl PartialOrd for Temperature {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_c().partial_cmp(&other.as_c())
    }
}

/// A color. Internal representation may vary. The FoxBox adapters are
/// expected to perform conversions to the format requested by their
/// device.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Color {
    RGBA(f64, f64, f64, f64, f64)
}

/// Representation of an object in JSON. It is often (albeit not
/// always) possible to choose a more precise data structure for
/// representing values send/accepted by a service. If possible,
/// adapters should rather pick such more precise data structure.
#[derive(Debug, Clone, PartialEq)]
pub struct Json(serde_json::value::Value);

impl PartialOrd for Json {
    /// Two Json objects are never comparable to each other.
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}

/// A data structure holding a numeric value of a type that has not
/// been standardized yet.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtNumeric {
    pub value: f64,

    /// The vendor. Used for namespacing purposes, to avoid
    /// confusing two incompatible extensions with similar
    /// names. For instance, "foxlink@mozilla.com".
    pub vendor: String,

    /// Identification of the adapter introducing this value.
    /// Designed to aid with tracing and debugging.
    pub adapter: String,

    /// A string describing the nature of the value, designed to
    /// aid with type-checking.
    ///
    /// Examples: `"GroundHumidity"`.
    pub kind: String,
}

impl PartialOrd for ExtNumeric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.vendor != other.vendor {
            return None;
        } else if self.kind != other.kind {
            return None;
        } else {
            self.value.partial_cmp(&other.value)
        }
    }
}

/// Representation of an actual value that can be sent to/received
/// from a service.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unit,
    Bool(bool),
    Duration(Duration),
    TimeStamp(chrono::DateTime<Local>),
    Temperature(Temperature),
    Color(Color),
    String(String),

    // FIXME: Add more as we identify needs

    /// A numeric value representing a unit that has not been
    /// standardized yet into the API.
    ExtNumeric(ExtNumeric),
    Json(Json),
    Binary {data: Vec<u8>, mimetype: String}
}

impl Value {
    pub fn get_type(&self) -> Type {
        match *self {
            Value::Unit => Type::Unit,
            Value::Bool(_) => Type::Bool,
            Value::String(_) => Type::String,
            Value::Duration(_) => Type::Duration,
            Value::TimeStamp(_) => Type::TimeStamp,
            Value::Temperature(_) => Type::Temperature,
            Value::Color(_) => Type::Color,
            Value::Json(_) => Type::Json,
            Value::Binary{..} => Type::Binary,
            Value::ExtNumeric(_) => Type::ExtNumeric,
        }
    }
}

impl PartialOrd for Value {
    /// Two values of the same type can be compared using the usual
    /// comparison for values of this type. Two values of distinct
    /// types cannot be compared.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use self::Value::*;
        use std::cmp::Ordering::*;
        match (self, other) {
            (&Unit, &Unit) => Some(Equal),
            (&Unit, _) => None,

            (&Bool(a), &Bool(b)) => a.partial_cmp(&b),
            (&Bool(_), _) => None,

            (&Duration(ref a), &Duration(ref b)) => a.partial_cmp(b),
            (&Duration(_), _) => None,

            (&TimeStamp(ref a), &TimeStamp(ref b)) => a.partial_cmp(b),
            (&TimeStamp(_), _) => None,

            (&Temperature(ref a), &Temperature(ref b)) => a.partial_cmp(b),
            (&Temperature(_), _) => None,

            (&Color(ref a), &Color(ref b)) => a.partial_cmp(b),
            (&Color(_), _) => None,

            (&ExtNumeric(ref a), &ExtNumeric(ref b)) => a.partial_cmp(b),
            (&ExtNumeric(_), _) => None,

            (&String(ref a), &String(ref b)) => a.partial_cmp(b),
            (&String(_), _) => None,

            (&Json(ref a), &Json(ref b)) => a.partial_cmp(b),
            (&Json(_), _) => None,

            (&Binary{mimetype: ref a_mimetype, data: ref a_data},
             &Binary{mimetype: ref b_mimetype, data: ref b_data}) if a_mimetype == b_mimetype => a_data.partial_cmp(b_data),
            (&Binary{..}, _) => None,
        }
    }
}
