/// Basic structure of a Monitor (aka Server App, aka wtttttt)
///
/// Monitors are designed so that the FoxBox can offer a simple
/// IFTTT-style Web UX to let users write their own scripts. More
/// complex monitors can installed from the web from a master device
/// (i.e. the user's cellphone or smart tv).
///
///
/// # Example
///
/// "During the night, reduce the temperature of heaters to x degress."
///
/// Condition: Time-of-day > 9pm OR Time-of-day < 7am
/// Execute: Temperature-of-heater-1
///
///
/// # Example
///
/// "When I leave the house, if the oven is on, send me a message."
///
///
/// # Example
///
/// "When I haven't seen any movement in 10 minutes, turn off the lights."


struct ServerApp {
    metadata: (), // FIXME: Authorizations, author, description, update url, version, ...
    code: Vec<Trigger>,
}

/// A single trigger, i.e. "when some condition is true, do something".
struct Trigger {
    /// The condition in which to execute the trigger. Its a disjunction of conjunctions.
    ///
    /// # Example
    /// Door alarm #1 rings OR door alarm #2 rings
    condition: Disjunction,

    /// Stuff to do once `condition` is met.
    execute: Vec<Command>,

    /// Minimal duration between two executions of the trigger.
    cooldown: Duration,

    /// The list of inputs used by the trigger. The `Resource` is the
    /// requirement of the app ("a temperature sensor"), while the
    /// `Path` is the actual REST path used to perform calls.
    ///
    /// Mappings are picked when the application is installed and can
    /// change as devices are added/removed.
    inputs: Map<Resource, Path>,
    outputs: Map<Resource, Path>,
}

struct Disjunction {
    /// The disjunction is true iff any of the following conjunctions is true.
    any: Vec<Conjunction>
}

struct Conjunction {
    /// The conjunction is true iff all of the following expressions evaluate to true.
    all: Vec<Expression>
}


/// An elementary condition. Typically, this is a comparison between two values.
struct Expression {
    // FIXME: Emulate GADTs to ensure that stuff is correctly typed?
    // Nice, but a bit heavy and probably not useful for a prototype.
    left: Operand,
    operator: Operator,
    right: Operand,
}

/// A value to be compared.
enum Operand {
    // Constants
    Num(f64),
    String(String),
    Bool(bool),
    Date(Date),
    Duration(Duration),

    /// Dynamic values, including both actual sensors and higher-level values.
    ///
    /// # Example
    ///
    /// "Is there movement on motion detector" (may be true/false)
    ///
    /// # Example
    ///
    /// "Date of the latest motion on motion detector" (a Date)
    Input (Input),
}

enum Operator {
    // Operations on all values.
    Equals,
    NotEquals,

    // Operations on numbers, dates, durations.
    GreaterEq,
    Greater,
    LowerEq,
    Lower,
    Plus,
    Minus,

    // Operations on strings
    Contains,
    NotContains,
}

/// An input value. It may come from an actual sensor, from some
/// metadata, of from a value stored previously by the application.
struct Input {
    /// Minimal duration between two checks of the value.
    refresh: Duration,

    /// Path to a local URI for a REST call.
    // FIXME: Really? This kind of assumes that we are polling. That's
    // probably not what we want.
    source: Resource,
}

/// Stuff to actually do. In practice, this is always a REST call.
// FIXME: Need to decide how much we wish to sandbox apps.
struct Command {
    /// The resource to which this command applies.
    destination: Resource,

    /// The API call. Typically, this will map immediately to a REST
    /// path + method + JSON format.
    api: API,

    arguments: Map<String, Option<Expression>>
}

