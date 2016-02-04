/// Basic structure of a Monitor (aka Server App, aka wtttttt)
///
/// Monitors are designed so that the FoxBox can offer a simple
/// IFTTT-style Web UX to let users write their own scripts. More
/// complex monitors can installed from the web from a master device
/// (i.e. the user's cellphone or smart tv).

use dependencies::Path;

struct ServerApp {
    metadata: (), // FIXME: Authorizations, author, description, update url, version, ...

    /// `true` if the user has decided to activate the app, `false` if
    /// the user has turned it off.
    isActivatedByUser: bool,

    /// A set of requirements (e.g. "a temperature sensor" / "all
    /// temperature sensors" / "the date since the latest movement in
    /// any motion sensor"). These are specified in the source code
    /// and do not change unless the source code changes.
    ///
    /// The position in the vector is important, as it is used to
    /// represent the instances of resources in the script.
    ///
    /// FIXME: We also want a user-readable name for the requirements,
    /// for the sake of the front-end. These names may even be
    /// internationalizable. Later.
    requirements: Vec<Requirement>,

    /// The resources actually allocated to match the requirements.
    /// Allocations are typically done by the user when installing the
    /// app or the devices. Behind-the-scenes, each allocation is a
    /// mapping to a set of (local) REST APIs.
    ///
    /// FIXME: We also want a user-readable name for the allocations,
    /// for the sake of the front-end. These names may even be
    /// internationalizable. Later.
    allocations: Vec<Vec<Path>>,

    code: Vec<Trigger>,
}

/// A resource needed by this application. Typically, a definition of
/// an input or output device.
struct Requirement {
    /// The kind of resource, e.g. "flashbulb".
    kind: String, // FIXME: There must be some kind of standard, no?

    /// The set of properties of the resource (e.g. luminosity,
    /// temperature).
    properties: Vec<String>,

    /// Minimal number of resources required.
    min: u32,

    /// Maximal number of resources that may be handled.
    max: u32,

    /// The minimal duration between two reads from this device
    /// or `None` if this device is not used for input.
    refresh: Option<Duration>,

    /// The minimal duration between two outputs to this device
    /// or `None` if this device is not used for output.
    cooldown: Option<Duration>,
}


/// A single trigger, i.e. "when some condition is true, do something".
struct Trigger {
    /// The condition in which to execute the trigger.
    condition: Disjunction,

    /// Stuff to do once `condition` is met.
    execute: Vec<Command>,

    /// Minimal duration between two executions of the trigger.  If a
    /// duration was not picked by the developer, a reasonable default
    /// duration is picked (e.g. 10 minutes).
    cooldown: Duration,
}

/// A disjunction (e.g. a "or") of conditions.
///
/// # Example
///
/// Door alarm #1 OR door alarm #2
struct Disjunction {
    /// The disjunction is true iff any of the following conjunctions is true.
    any: Vec<Conjunction>
}

/// A conjunction (e.g. a "and") of conditions.
struct Conjunction {
    /// The conjunction is true iff all of the following expressions evaluate to true.
    all: Vec<Expression>
}

enum Value {
    /// Constants
    Num(f64),
    String(String),
    Bool(bool),
    Date(Date),
    Duration(Duration),
}

enum Expression {
    Const {
        value: Value,

        /// Taint values with their source. Used to e.g. display the
        /// name of the sensor.
        sources: Vec<usize>,
    },

    /// Dynamic values, including both actual sensors and higher-level values.
    ///
    /// # Example
    ///
    /// "Is there movement on motion detector" (may be true/false)
    ///
    /// # Example
    ///
    /// "Date of the latest motion on motion detector" (a Date)
    Input {
        /// A reference to the device used for input.
        /// This is an index in `requirements` and `allocations`.
        index: usize, // FIXME: We should use a custom type.

        /// A property to fetch (e.g. "luminosity" or "meta/latest-on").
        property: String,
    },

    Variable(Variable),

    LetBinding {
        // FIXME: We should be able to find something more
        // user-friendly than let-binding.
        variable: Variable,
        expr: Expression,
    },

    /// Pure functions on values.
    Function {
        function: Function,
        arguments: Vec<Expression>
    },
}

struct Variable {
    index: usize
}

enum Function {
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

    // Etc.  FIXME: We'll need operations on dates, extracting name
    // from device, etc.
}

/// Stuff to actually do. In practice, this is always a REST call.
// FIXME: Need to decide how much we wish to sandbox apps.
struct Command {
    /// The resource to which this command applies,
    /// as an index in Trigger.requirements/allocations.
    destination: usize,  // FIXME: Use custom type.

    arguments: Map<String, Option<Expression>>
}

