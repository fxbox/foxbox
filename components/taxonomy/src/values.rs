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

/// Base definitions for actual values. Most API clients will rather
/// use `Value`.
///
/// Splitting the defintion between `Value` and `ValueBase` is mostly
/// a convenience to refine the automatically-derived `PartialOrd` on
/// `ValueBase` into something that will never compare two values with
/// different types.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ValueBase {
    Unit,
    Bool(bool),
    Duration(Duration),
    TimeStamp(chrono::DateTime<Local>),
    Temperature(Temperature),
    Color(Color),

    // FIXME: Add more as we identify needs

    /// A numeric value representing a unit that has not been
    /// standardized yet into the API.
    ExtNumeric(ExtNumeric),
    Json(Json),
    Binary {data: Vec<u8>, mimetype: String}
}

impl ValueBase {
    pub fn get_type(&self) -> Type {
        match *self {
            ValueBase::Unit => Type::Unit,
            ValueBase::Bool(_) => Type::Bool,
            ValueBase::Duration(_) => Type::Duration,
            ValueBase::TimeStamp(_) => Type::TimeStamp,
            ValueBase::Temperature(_) => Type::Temperature,
            ValueBase::Color(_) => Type::Color,
            ValueBase::Json(_) => Type::Json,
            ValueBase::Binary{..} => Type::Binary,
            ValueBase::ExtNumeric(_) => Type::ExtNumeric,
        }
    }
}

/// Representation of an actual value that can be sent to/received
/// from a service.
#[derive(Debug, Clone, PartialEq)]
pub struct Value(ValueBase);

impl Value {
    /// Get the type attached to a value.
    pub fn get_type(&self) -> Type {
        self.0.get_type()
    }
}

impl PartialOrd for Value {
    /// Two values of the same type can be compared using the usual
    /// comparison for values of this type. Two values of distinct
    /// types cannot be compared.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.get_type() != other.get_type() {
            None
        } else {
            self.0.partial_cmp(&other.0)
        }
    }
}
