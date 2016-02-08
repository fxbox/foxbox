
#![allow(unused_variables)]
#![allow(dead_code)]

/// Basic structure of a Monitor (aka Server App, aka wtttttt)
///
/// Monitors are designed so that the FoxBox can offer a simple
/// IFTTT-style Web UX to let users write their own scripts. More
/// complex monitors can installed from the web from a master device
/// (i.e. the user's cellphone or smart tv).

use dependencies::{DeviceKind, InputCapability, OutputCapability, Device, Range, Watcher, Witness};

use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

extern crate rustc_serialize;
use self::rustc_serialize::json::Json;

extern crate itertools;
use self::itertools::Zip;

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
#[derive(Clone)]
struct MonitorApp {
    metadata: (), // FIXME: Authorizations, author, description, update url, version, ...

    /// `true` if the app is on, `false` otherwise.
    is_active: bool,

    /// Monitor applications have sets of requirements (e.g. "I need a
    /// camera"), which are allocated to actual resources through the
    /// UX. Re-allocating resources may be requested by the user, the
    /// foxbox, or an application, e.g. when replacing a device or
    /// upgrading the app.
    ///
    /// The position in the vector is important, as it is used to
    /// represent the instances of resources in the script.
    ///
    /// FIXME: Turn this `Vec` (and others) into a data structure in
    /// which this indexing property is built-in.
    requirements: Vec<Named<Requirement>>,

    /// For each requirement, the resources actually allocated to
    /// match the requirements. This may be 1 or more individual
    /// devices.
    ///
    /// Allocations can be done in several ways:
    ///
    /// - when entering the script through a UX (either the script or
    ///   the UX can suggest allocations);
    /// - when installing an application with a web front-end.
    allocations: Vec<Vec<Named<Arc<Device>>>>,

    code: Vec<Trigger>,

    is_running: bool,
}

impl<'a> MonitorApp {
/*
    Ideally, I'd like something along these lines.
    But, as rustc kindly pointed out to me, that's pretty much unsafe.

    fn iter_state_index(&'a self) -> Box<Iterator<Item=InputIndex> + 'a>
    {
        Box::new(
            Zip::new((&self.requirements, &self.allocations, 0..)).
                flat_map(|(req, allocation, allocation_index)| {
                    let allocation_index_clone = allocation_index.clone();
                    Zip::new((allocation, 0..)).
                        flat_map(|(individual_device, individual_device_index)| {
                            Zip::new((&req.data.inputs, 0..)).
                                map(|(_, individual_input_index)| {
                                    InputIndex {
                                        allocation: allocation_index_clone,
                                        device: individual_device_index,
                                        input: individual_input_index
                                    }
                                })
                        })
                }))
}
     */
    fn iter_state_index(&'a self) -> Vec<InputIndex>
    {
        let mut vec = Vec::new();
        for (req, allocation, allocation_index) in Zip::new((&self.requirements, &self.allocations, 0..)) {
            for (individual_device, individual_device_index) in Zip::new((allocation, 0..)) {
                for (input, individual_input_index) in Zip::new((&req.data.inputs, 0..)) {
                    vec.push(InputIndex {
                        allocation: allocation_index,
                        device: individual_device_index,
                        input: individual_input_index
                    })
                }
            }
        }
        vec
    }

    fn get_individual_device(&self, index: &InputIndex) -> Arc<Device> {
        self.allocations[index.allocation][index.device].data.clone()
    }

    fn get_input(&self, index: &InputIndex) -> InputCapability {
        self.requirements[index.allocation].data.inputs[index.input].data.clone()
    }
}

struct InputIndex {
    allocation: usize,
    device: usize,
    input: usize,
}


struct ConditionState {
    are_conditions_met: bool
}

type InputState = Vec<Vec<Vec<Option<Json>>>>;
struct MonitorTaskState {
    // Invariant: len() is the same as that of app.code
    trigger_condition_state: Vec<ConditionState>,

    /// For each device set allocated, for each individual device
    /// allocated, for each input watched by the app, the latest state
    /// received.
    // Invariant: outer len() is the same as that of self.requirements.
    // Invariant: inner len() is the same as number of input devices bound
    // to the corresponding requirement. May be empty.
    input_state: InputState,

    witnesses: Vec<Witness>,

    app: MonitorApp,
}
impl MonitorTaskState {
    fn set_state(&mut self, index: &InputIndex, value: Option<Json>) {
        self.input_state[index.allocation][index.device][index.input] = value;
    }
}

struct MonitorComm {
    tx: Sender<MonitorOp>,
    rx: Receiver<MonitorOp>,
}
struct MonitorTask {
    state: MonitorTaskState,
    comm: MonitorComm,
}

impl MonitorTask {
    fn new(app: MonitorApp) -> Self {
        // Initialize condition state.
        let mut trigger_condition_state = Vec::with_capacity(app.code.len());
        for _ in &app.code {
            trigger_condition_state.push(ConditionState {
                are_conditions_met: false,
            });
        }
        assert_eq!(trigger_condition_state.len(), app.code.len());
        
        // Initialize input state.
        let mut full_input_state = Vec::with_capacity(app.requirements.len());
        for (req, allocation) in app.requirements.iter().zip(&app.allocations) {

            // State for this allocation. A single allocation may map
            // to a number of inputs (e.g. "all fire alarms").
            let mut allocation_state = Vec::with_capacity(allocation.len());
            for individual_device in allocation {
                let mut individual_device_state = Vec::with_capacity(req.data.inputs.len());
                for _ in 0 .. req.data.inputs.len() {
                    individual_device_state.push(None);
                }
                allocation_state.push(individual_device_state);
            }
            assert_eq!(allocation_state.len(), allocation.len());
            full_input_state.push(allocation_state);
        }
        assert_eq!(full_input_state.len(), app.requirements.len());

        // Start watching
        let mut watcher = Watcher::new();
        let mut witnesses = Vec::new();
        let (tx, rx) = channel();

        for state_index in app.iter_state_index() {
            // FIXME: We currently use `Range::any()` for simplicity.
            // However, in most cases, we should be able to look inside
            // the condition to build a better `Range`.
            witnesses.push(
                watcher.add(
                    &app.get_individual_device(&state_index), // FIXME: Implement
                    &app.get_input(&state_index), // FIXME: Implement
                    &Range::any(),
                    |data| {
                        let _ = tx.send(MonitorOp::Update {
                            data: data,
                            index: state_index,
                        });
                        // Ignore errors. If the thread is shutting down, it's ok to lose messages.
                    }));
        }

        MonitorTask {
            state: MonitorTaskState {
                trigger_condition_state: trigger_condition_state,
                input_state: full_input_state,
                witnesses: witnesses,
                app: app,
            },
            comm: MonitorComm {
                tx: tx,
                rx: rx,
            }
        }
    }

    fn run(&mut self) {
        for msg in &self.comm.rx {
            use self::MonitorOp::*;
            match msg {
                Update {
                    data,
                    index,
                } => {
                    // Update the state
                    self.state.set_state(&index, Some(data));

                    // Find out if we should execute triggers.
                    // FIXME: We could optimize this by finding out which triggers
                    // are tainted by the update and only rechecking these.
                    for (trigger, trigger_condition_state) in Zip::new((&self.state.app.code, &mut self.state.trigger_condition_state)) {
                        let is_met = trigger.condition.is_met(&self.state.input_state);
                        if is_met == trigger_condition_state.are_conditions_met {
                            // No change in conditions. Nothing to do.
                            continue;
                        }
                        trigger_condition_state.are_conditions_met = is_met;
                        if !is_met {
                            // Conditions were met, now they are not anymore.
                            // The next time they are met, we can trigger
                            // a new execution.
                            continue;
                        }
                        // Conditions were not met, now they are, so it is
                        // time to start executing. We copy the inputs
                        // and dispatch to a background thread

                        // FIXME: Handle cooldown.
                                
                        trigger_condition_state.are_conditions_met = true;
                        let _ = self.comm.tx.send(MonitorOp::Execute {
                            state: self.state.input_state.clone(),
                            commands: trigger.execute.clone()
                        });
                        // Ignore errors. If the thread is shutting down, it's ok to lose messages.
                    }
                },
                Execute {..} => {
                    panic!("Not implemented");
                }
                Stop => {
                    // Clean up watcher, stop the thread.
                    self.state.witnesses.clear();
                    return;
                }
            }
        }
    }
}

enum MonitorOp {
    Update{data: Json, index: InputIndex},
    Execute{state: InputState, commands: Vec<Command>},
    Stop
}

impl MonitorApp {
    pub fn start(&mut self) {
        if self.is_running {
            return;
        }
        self.is_running = true;

        let mut task = MonitorTask::new(self.clone());
        thread::spawn(move || {
            task.run();
        });
    }
}



/// Data labelled with a user-readable name.
#[derive(Clone)]
struct Named<T> {
    /// User-readable name.
    name: String,

    data: T,
}

/// A resource needed by this application. Typically, a definition of
/// device with some input our output capabilities.
#[derive(Clone)]
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
#[derive(Clone)]
struct Trigger {
    /// The condition in which to execute the trigger.
    condition: Disjunction,

    /// Stuff to do once `condition` is met.
    execute: Vec<Command>,

    /// Minimal duration between two executions of the trigger.  If a
    /// duration was not picked by the developer, a reasonable default
    /// duration should be picked (e.g. 10 minutes).
    cooldown: Duration,
}

/// A disjunction (e.g. a "or") of conditions.
///
/// # Example
///
/// Door alarm #1 OR door alarm #2
#[derive(Clone)]
struct Disjunction {
    /// The disjunction is true iff any of the following conjunctions is true.
    any: Vec<Conjunction>
}

impl Disjunction {
    fn is_met(&self, input_state: &InputState) -> bool {
        panic!("Not implemented");
    }
}

/// A conjunction (e.g. a "and") of conditions.
#[derive(Clone)]
struct Conjunction {
    /// The conjunction is true iff all of the following expressions evaluate to true.
    all: Vec<Expression>
}

#[derive(Clone)]
enum Value {
    Json(Json),
    Blob{data: Arc<Vec<u8>>, mime_type: String},
}

/// An expression in the language.  Note that expressions may contain
/// inputs, which are typically asynchronous. Consequently,
/// expressions are evaluated asynchronously.
#[derive(Clone)]
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

#[derive(Clone)]
enum Function {
    // Operations on all values.
    InRange,
    OutOfRange,
    // Operations on strings
    Contains,
    NotContains,
}

/*
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
 */

/// Stuff to actually do. In practice, this is always a REST call.
// FIXME: Need to decide how much we wish to sandbox apps.
#[derive(Clone)]
struct Command {
    /// The resource to which this command applies,
    /// as an index in Trigger.requirements/allocations.
    destination: usize,  // FIXME: Use custom type.

    action: OutputCapability,

    arguments: HashMap<String, Option<Expression>>
}

