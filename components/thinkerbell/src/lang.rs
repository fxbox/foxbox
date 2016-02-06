/// Basic structure of a Monitor (aka Server App, aka wtttttt)
///
/// Monitors are designed so that the FoxBox can offer a simple
/// IFTTT-style Web UX to let users write their own scripts. More
/// complex monitors can installed from the web from a master device
/// (i.e. the user's cellphone or smart tv).

use dependencies::{DeviceKind, InputCapability, OutputCapability, Device, Range, Watcher};

use std::time::Duration;
use std::collections::HashMap;

extern crate rustc_serialize;
use self::rustc_serialize::json::Json;

/// A Monitor Application, i.e. an application (or a component of an
/// application) executed on the server.
///
/// Monitor applications are typically used for triggering an action
/// in reaction to an event: changing temperature when night falls,
/// ringing an alarm when a door is opened, etc.
///
/// Monitor applications are installed from a paired device. They may
/// either be part of a broader application (which can install them
/// through a web/REST API) or live on their own.
struct MonitorApp {
    metadata: (), // FIXME: Authorizations, author, description, update url, version, ...

    /// `true` if the user has decided to activate the app, `false` if
    /// the user has turned it off.
    is_activated_by_user: bool,

    /// Monitor applications have sets of requirements (e.g. "I need a
    /// camera"), which are allocated to actual resources through the
    /// UX. Re-allocating resources may be requested by the user, the
    /// foxbox, or an application, e.g. when replacing a device or
    /// upgrading the app.
    ///
    /// The position in the vector is important, as it is used to
    /// represent the instances of resources in the script.
    /// FIXME: Turn this `Vec` into a data structure in which this property is built-in.
    requirements: Vec<Named<Requirement>>,

    /// The resources actually allocated to match the requirements.
    /// Allocations can be done in several ways:
    ///
    /// - when entering the script through a UX (either the scrip, the UX can suggest
    ///   allocations;
    /// - when installing 
    /// Allocations are typically done by the user when installing the
    /// app or the devices. Behind-the-scenes, each allocation is a
    /// mapping to a set of (local) REST APIs.
    ///
    /// FIXME: We also want a user-readable name for the allocations,
    /// for the sake of the front-end. These names may even be
    /// internationalizable. Later.
    allocations: Vec<Vec<Named<Device>>>,

    code: Vec<Trigger>,
}

impl MonitorApp {
    fn monitor(&self) {
        if !self.is_activated_by_user {
            return;
        }
        
        // Build a watcher for all the input conditions.
        // FIXME: In future versions, there will be a lot to optimize here.
        let mut watcher = Watcher::new();
        let mut witnesses = Vec::new();

        // Build a representation of the state.
        // This state will be updated whenever one of the inputs changes state.
        let mut full_state = Vec::new();

        let mut index : usize = 0;
        for req in &self.requirements {
            let ref devices = self.allocations[index];
            for device in devices {
                let mut device_state = Vec::new();
                let input_index = 0;
                for input in &req.data.inputs {
                    device_state.push(Json::Null);
                    let cb = |data| {
                        device_state[input_index] = data;
                        // FIXME: Also, check whether the conditions
                        // are now valid.
                    };
                    // FIXME: For the moment, we assume that
                    // conditions on inputs are a OR. Decide whether
                    // this is a good idea.
                    let witness = watcher.add(&device.data,
                                              &input.data,
                                              &Range::any(),
                                              cb);
                    // FIXME: We could do better than Range::any()
                    witnesses.push(witness);
                }
                full_state.push(device_state);
            }
            index += 1;
        }
    }
}

/// Data labelled with a user-readable name.
struct Named<T> {
    /// User-readable name.
    name: String,

    data: T,
}

/// A resource needed by this application. Typically, a definition of
/// device with some input our output capabilities.
struct Requirement {
    /// The kind of resource, e.g. "a flashbulb".
    kind: DeviceKind,

    /// Input capabilities we need from the device, e.g. "the time of
    /// day", "the current temperature".
    inputs: Vec<Named<InputCapability>>,

    /// Output capabilities we need from the device, e.g. "play a
    /// sound", "set luminosity".
    outputs: Vec<Named<OutputCapability>>,
    
    /// Minimal number of resources required. If unspecified in the
    /// script, this is 1.
    min: u32,

    /// Maximal number of resources that may be handled. If
    /// unspecified in the script, this is the same as `min`.
    max: u32,

    // FIXME: We may need cooldown properties.
}


/// A single trigger, i.e. "when some condition becomes true, do
/// something".
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
    json(Json),
    blob{data: Vec<u8>, mime_type: String},
}

/// An expression in the language.  Note that expressions may contain
/// inputs, which are typically asynchronous. Consequently,
/// expressions are evaluated asynchronously.
enum Expression {
    Const {
        value: Value,

        /// We are dealing with real-world values, so physical units
        /// will prevent real-world accidents.
        unit: (), // FIXME: An actual unit

        /// The source for this value.  Used whenever we need to find
        /// out which sensor triggered the trigger (e.g. "where is the
        /// fire?").
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
        property: InputCapability,
    },

    /// Pure functions on values. The fact that they are pure
    /// functions is important, as it lets us find out automatically
    /// which subexpressions (including inputs) do not need to be
    /// re-evaluated.
    Function {
        function: Function,
        // FIXME: use a Box<> for now to avoid recursive type.
        arguments: Vec<Box<Expression>>
    },
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

    action: OutputCapability,

    arguments: HashMap<String, Option<Expression>>
}

