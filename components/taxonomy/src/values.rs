//!
//! Values manipulated by services
//!
use util::*;

use std::cmp::{ PartialOrd, Ordering };
use std::str::FromStr;
use std::sync::Arc;

use chrono::{ Duration as ChronoDuration, DateTime, Local, TimeZone, UTC };

use serde_json;
use serde::ser::{ Serialize, Serializer };
use serde::de::{ Deserialize, Deserializer, Error, Visitor as DeserializationVisitor };

/// Representation of a type error.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TypeError {
    /// The type we expected.
    pub expected: Type,

    /// The type we actually got.
    pub got: Type,
}

///
/// The type of values manipulated by endpoints.
///
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub enum Type {
    ///
    /// # Trivial values
    ///

    /// An empty value. Used for instance to inform that a countdown
    /// has reached 0 or that a device is ready.
    Unit,

    ///
    /// # Boolean values
    ///

    /// A boolean on/off state. Used for various two-states switches.
    OnOff,

    /// A boolean open/closed state. Used for instance for doors,
    /// windows, etc.
    OpenClosed,

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

    ExtBool,
    ExtNumeric,
}

impl Type {
    /// Determine whether using `Range::Eq` for this type is
    /// appropriate. Typically, using `Range::Eq` for a floating point
    /// number is a bad idea.
    pub fn supports_eq(&self) -> bool {
        use self::Type::*;
        match *self {
            Duration | TimeStamp | Temperature | ExtNumeric | Color => false,
            Unit | String | Json | Binary | OnOff | OpenClosed | ExtBool => true,
        }
    }

    pub fn ensure_eq(&self, other: &Self) -> Result<(), TypeError> {
        if self == other {
            Ok(())
        } else {
            Err(TypeError {
                expected: self.clone(),
                got: other.clone(),
            })
        }
    }
}

/// An on/off state.
///
/// # (De)serialization
///
/// Values of this state are represented by strings "On"|"Off".
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
///
/// let on = serde_json::to_string(&foxbox_taxonomy::values::OnOff::On).unwrap();
/// assert_eq!(on, "\"On\"");
///
/// let on : foxbox_taxonomy::values::OnOff = serde_json::from_str("\"On\"").unwrap();
/// assert_eq!(on, foxbox_taxonomy::values::OnOff::On);
///
/// let off = serde_json::to_string(&foxbox_taxonomy::values::OnOff::Off).unwrap();
/// assert_eq!(off, "\"Off\"");
///
/// let off : foxbox_taxonomy::values::OnOff = serde_json::from_str("\"Off\"").unwrap();
/// assert_eq!(off, foxbox_taxonomy::values::OnOff::Off);
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OnOff {
    On,
    Off,
}

impl OnOff {
    fn as_bool(&self) -> bool {
        match *self {
            OnOff::On => true,
            OnOff::Off => false,
        }
    }
}

impl PartialOrd for OnOff {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OnOff {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bool().cmp(&other.as_bool())
    }
}

impl Serialize for OnOff {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        match *self {
            OnOff::On => "On".serialize(serializer),
            OnOff::Off => "Off".serialize(serializer)
        }
    }
}
impl Deserialize for OnOff {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error> where D: Deserializer {
        deserializer.visit_string(TrivialEnumVisitor::new(|source| {
            match source {
                "On" => Ok(OnOff::On),
                "Off" => Ok(OnOff::Off),
                _ => Err(())
            }
        }))
    }
}

/// An open/closed state.
///
/// # (De)serialization
///
/// Values of this state are represented by strings "Open"|"Closed".
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
///
/// let open = serde_json::to_string(&foxbox_taxonomy::values::OpenClosed::Open).unwrap();
/// assert_eq!(open, "\"Open\"");
///
/// let open : foxbox_taxonomy::values::OpenClosed = serde_json::from_str("\"Open\"").unwrap();
/// assert_eq!(open, foxbox_taxonomy::values::OpenClosed::Open);
///
/// let closed = serde_json::to_string(&foxbox_taxonomy::values::OpenClosed::Closed).unwrap();
/// assert_eq!(closed, "\"Closed\"");
///
/// let closed : foxbox_taxonomy::values::OpenClosed = serde_json::from_str("\"Closed\"").unwrap();
/// assert_eq!(closed, foxbox_taxonomy::values::OpenClosed::Closed);
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OpenClosed {
    Open,
    Closed,
}

impl OpenClosed {
    fn as_bool(&self) -> bool {
        match *self {
            OpenClosed::Open => true,
            OpenClosed::Closed => false,
        }
    }
}

impl PartialOrd for OpenClosed {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OpenClosed {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bool().cmp(&other.as_bool())
    }
}

impl Serialize for OpenClosed {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        match *self {
            OpenClosed::Open => "Open".serialize(serializer),
            OpenClosed::Closed => "Closed".serialize(serializer)
        }
    }
}
impl Deserialize for OpenClosed {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error> where D: Deserializer {
        deserializer.visit_string(TrivialEnumVisitor::new(|source| {
            match source {
                "Open" | "open" => Ok(OpenClosed::Open),
                "Closed" | "closed" => Ok(OpenClosed::Closed),
                _ => Err(())
            }
        }))
    }
}

/// A temperature. Internal representation may be either Fahrenheit or
/// Celcius. The FoxBox adapters are expected to perform conversions
/// to the format requested by their devices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Color {
    RGBA(f64, f64, f64, f64, f64)
}

/// Representation of an object in JSON. It is often (albeit not
/// always) possible to choose a more precise data structure for
/// representing values send/accepted by a service. If possible,
/// adapters should rather pick such more precise data structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Json(pub serde_json::value::Value);

impl PartialOrd for Json {
    /// Two Json objects are never comparable to each other.
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}

/// A data structure holding a boolean value of a type that has not
/// been standardized yet.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtBool {
    pub value: bool,

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
    /// Examples: `"PresenceDetected"`.
    pub kind: String,
}

impl PartialOrd for ExtBool {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.vendor != other.vendor
        || self.kind != other.kind {
            None
        } else {
            self.value.partial_cmp(&other.value)
        }
    }
}

/// A data structure holding a numeric value of a type that has not
/// been standardized yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        if self.vendor != other.vendor
        || self.kind != other.kind {
            None
        } else {
            self.value.partial_cmp(&other.value)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Binary {
   /// The actual data. We put it behind an `Arc` to make sure
   /// that cloning remains inexpensive.
   data: Arc<Vec<u8>>,

   /// The mime type. Should probably be an Id<MimeTypeId>.
   mimetype: Id<MimeTypeId>,
}
#[derive(Clone, Debug)]
struct MimeTypeId;

/// Representation of an actual value that can be sent to/received
/// from a service.
///
/// # (De)serialization
///
/// Values of this state are represented by an object `{ key: value }`, where key is one of
/// `Unit`, `OnOff`, `OpenClosed`, ... Note that the `value` for `Unit` is `[]`.
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
///
/// # fn main() {
/// use foxbox_taxonomy::values::Value::*;
/// use foxbox_taxonomy::values::OnOff::*;
/// use foxbox_taxonomy::values::OpenClosed::*;
///
/// let unit = serde_json::to_string(&Unit).unwrap();
/// assert_eq!(unit, "{\"Unit\":[]}");
///
/// let unit : foxbox_taxonomy::values::Value = serde_json::from_str("{\"Unit\":[]}").unwrap();
/// assert_eq!(unit, Unit);
///
/// let on = serde_json::to_string(&OnOff(On)).unwrap();
/// assert_eq!(on, "{\"OnOff\":\"On\"}");
///
/// let on : foxbox_taxonomy::values::Value = serde_json::from_str("{\"OnOff\":\"On\"}").unwrap();
/// assert_eq!(on, OnOff(On));
///
/// let open = serde_json::to_string(&OpenClosed(Open)).unwrap();
/// assert_eq!(open, "{\"OpenClosed\":\"Open\"}");
///
/// let open : foxbox_taxonomy::values::Value = serde_json::from_str("{\"OpenClosed\":\"Open\"}").unwrap();
/// assert_eq!(open, OpenClosed(Open));
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Unit,
    OnOff(OnOff),
    OpenClosed(OpenClosed),
    Duration(Duration),
    TimeStamp(TimeStamp),
    Temperature(Temperature),
    Color(Color),
    String(Arc<String>),

    // FIXME: Add more as we identify needs

    /// A boolean value representing a unit that has not been
    /// standardized yet into the API.
    ExtBool(ExtBool),

    /// A numeric value representing a unit that has not been
    /// standardized yet into the API.
    ExtNumeric(ExtNumeric),

    /// A Json value. We put it behind an `Arc` to make sure that
    /// cloning remains inexpensive.
    Json(Arc<Json>),

    /// Binary data.
    Binary(Binary),
}

impl Value {
    pub fn get_type(&self) -> Type {
        match *self {
            Value::Unit => Type::Unit,
            Value::OnOff(_) => Type::OnOff,
            Value::OpenClosed(_) => Type::OpenClosed,
            Value::String(_) => Type::String,
            Value::Duration(_) => Type::Duration,
            Value::TimeStamp(_) => Type::TimeStamp,
            Value::Temperature(_) => Type::Temperature,
            Value::Color(_) => Type::Color,
            Value::Json(_) => Type::Json,
            Value::Binary(_) => Type::Binary,
            Value::ExtBool(_) => Type::ExtBool,
            Value::ExtNumeric(_) => Type::ExtNumeric,
        }
    }

    pub fn as_timestamp(&self) -> Result<&TimeStamp, TypeError> {
        match *self {
            Value::TimeStamp(ref x) => Ok(x),
            _ => Err(TypeError {expected: Type::TimeStamp, got: self.get_type()})
        }
    }

    pub fn as_duration(&self) -> Result<&Duration, TypeError> {
        match *self {
            Value::Duration(ref x) => Ok(x),
            _ => Err(TypeError {expected: Type::Duration, got: self.get_type()})
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

            (&OnOff(ref a), &OnOff(ref b)) => a.partial_cmp(b),
            (&OnOff(_), _) => None,

            (&OpenClosed(ref a), &OpenClosed(ref b)) => a.partial_cmp(b),
            (&OpenClosed(_), _) => None,

            (&Duration(ref a), &Duration(ref b)) => a.partial_cmp(b),
            (&Duration(_), _) => None,

            (&TimeStamp(ref a), &TimeStamp(ref b)) => a.partial_cmp(b),
            (&TimeStamp(_), _) => None,

            (&Temperature(ref a), &Temperature(ref b)) => a.partial_cmp(b),
            (&Temperature(_), _) => None,

            (&Color(ref a), &Color(ref b)) => a.partial_cmp(b),
            (&Color(_), _) => None,

            (&ExtBool(ref a), &ExtBool(ref b)) => a.partial_cmp(b),
            (&ExtBool(_), _) => None,

            (&ExtNumeric(ref a), &ExtNumeric(ref b)) => a.partial_cmp(b),
            (&ExtNumeric(_), _) => None,

            (&String(ref a), &String(ref b)) => a.partial_cmp(b),
            (&String(_), _) => None,

            (&Json(ref a), &Json(ref b)) => a.partial_cmp(b),
            (&Json(_), _) => None,

            (&Binary(self::Binary {mimetype: ref a_mimetype, data: ref a_data}),
             &Binary(self::Binary {mimetype: ref b_mimetype, data: ref b_data})) if a_mimetype == b_mimetype => a_data.partial_cmp(b_data),
            (&Binary(_), _) => None,
        }
    }
}


#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct TimeStamp(DateTime<UTC>);
impl TimeStamp {
    pub fn from_datetime(datetime: DateTime<UTC>) -> Self {
        TimeStamp(datetime)
    }
    pub fn as_datetime(&self) -> &DateTime<UTC> {
        &self.0
    }
    pub fn from_s(s: i64) -> Self {
        use chrono;
        let naive = chrono::naive::datetime::NaiveDateTime::from_timestamp(s, 0);
        let date = DateTime::<UTC>::from_utc(naive, UTC);
        TimeStamp(date)
    }
}
impl Into<DateTime<UTC>> for TimeStamp  {
    fn into(self) -> DateTime<UTC> {
        self.0
    }
}
impl Into<DateTime<Local>> for TimeStamp  {
    fn into(self) -> DateTime<Local> {
        self.0.with_timezone(&Local)
    }
}
impl<T> From<DateTime<T>> for TimeStamp where T: TimeZone {
    fn from(date: DateTime<T>) -> Self {
        TimeStamp(date.with_timezone(&UTC))
    }
}

impl Serialize for TimeStamp {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        let str = self.0.to_rfc3339();
        str.serialize(serializer)
    }
}
impl Deserialize for TimeStamp {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer {
        let str = try!(String::deserialize(deserializer));
        match DateTime::<UTC>::from_str(&str) {
            Ok(dt) => Ok(TimeStamp(dt)),
            Err(_) => Err(D::Error::syntax("Invalid date"))
        }
    }
}


#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
/// A comparison between two values.
pub enum Range {
    /// Leq(x) accepts any value v such that v <= x.
    Leq(Value),

    /// Geq(x) accepts any value v such that v >= x.
    Geq(Value),

    /// BetweenEq {min, max} accepts any value v such that `min <= v`
    /// and `v <= max`. If `max < min`, it never accepts anything.
    BetweenEq { min:Value, max:Value },

    /// OutOfStrict {min, max} accepts any value v such that `v < min`
    /// or `max < v`
    OutOfStrict { min:Value, max:Value },


    /// Eq(x) accespts any value v such that v == x
    Eq(Value),
}

impl Range {
    /// Determine if a value is accepted by this range.
    pub fn contains(&self, value: &Value) -> bool {
        use self::Range::*;
        match *self {
            Leq(ref max) => value <= max,
            Geq(ref min) => value >= min,
            BetweenEq { ref min, ref max } => min <= value && value <= max,
            OutOfStrict { ref min, ref max } => value < min || max < value,
            Eq(ref val) => value == val,
        }
    }

    /// Get the type associated to this range.
    ///
    /// If this range has a `min` and a `max` with conflicting types,
    /// produce an error.
    pub fn get_type(&self) -> Result<Type, TypeError> {
        use self::Range::*;
        match *self {
            Leq(ref v) | Geq(ref v) | Eq(ref v) => Ok(v.get_type()),
            BetweenEq {ref min, ref max} | OutOfStrict {ref min, ref max} => {
                let min_typ = min.get_type();
                let max_typ = max.get_type();
                if min_typ == max_typ {
                    Ok(min_typ)
                } else {
                    Err(TypeError {
                        expected: min_typ,
                        got: max_typ
                    })
                }
            }
        }
    }
}


/// A serializable wrapper around chrono::Duration.
///
/// # Serialization
///
/// Values are deserialized to a floating-point number of seconds.
///
/// ```
/// extern crate serde;
/// extern crate serde_json;
/// extern crate foxbox_taxonomy;
/// extern crate chrono;
///
/// # fn main() {
/// use foxbox_taxonomy::values::*;
///
/// let duration = Duration::from(chrono::Duration::milliseconds(3141));
///
/// let duration_as_json = serde_json::to_string(&duration).unwrap();
/// assert_eq!(duration_as_json, "3.141");
///
/// let duration_back : Duration = serde_json::from_str(&duration_as_json).unwrap();
/// assert_eq!(duration, duration_back);
/// # }
/// ```
#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Duration(ChronoDuration);

impl Serialize for Duration {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
     {
         serializer.visit_f64(self.0.num_milliseconds() as f64 / 1000 as f64)
     }
}
impl From<ChronoDuration> for Duration {
    fn from(source: ChronoDuration) -> Self {
        Duration(source)
    }
}
impl Into<ChronoDuration> for Duration {
    fn into(self) -> ChronoDuration {
        self.0
    }
}

impl Deserialize for Duration {
    /// Deserialize this value given this `Deserializer`.
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        struct DurationVisitor;
        impl DeserializationVisitor for DurationVisitor
        {
            type Value = Duration;
            fn visit_f64<E>(&mut self, v: f64) -> Result<Self::Value, E>
                where E: Error,
            {
                Ok(Duration(ChronoDuration::milliseconds((v * 1000.) as i64)))
            }
            fn visit_i64<E>(&mut self, v: i64) -> Result<Self::Value, E>
                where E: Error,
            {
                Ok(Duration(ChronoDuration::milliseconds(v * 1000)))
            }
            fn visit_u64<E>(&mut self, v: u64) -> Result<Self::Value, E>
                where E: Error,
            {
                self.visit_i64(v as i64)
            }
        }
        deserializer.visit_f64(DurationVisitor)
            .or_else(|_| deserializer.visit_i64(DurationVisitor))
    }
}
