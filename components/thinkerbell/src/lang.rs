
#![allow(unused_variables)]
#![allow(dead_code)]

/// Basic structure of a Monitor (aka Server App, aka wtttttt)
///
/// Monitors are designed so that the FoxBox can offer a simple
/// IFTTT-style Web UX to let users write their own scripts. More
/// complex monitors can installed from the web from a master device
/// (i.e. the user's cellphone or smart tv).

use dependencies::{DeviceKind, InputCapability, OutputCapability, Device, Range, Value, Watcher};

use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

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

    /// A set of rules, stating what must be done in which circumstance.
    rules: Vec<Trigger>,

    /// Either `None` if the application is not launched, or `Some(tx)`,
    /// where `tx` is a channel used to communicate with the thread
    /// running the application.
    command_sender: Option<Sender<MonitorOp>>,
}

impl<'a> MonitorApp {
    /// Index-safe iteration through all the possible
    /// allocations/individual devices/inputs.
    fn iter_state_index(&self) -> Vec<InputBinding>
    {
        // FIXME: Several possible optimizations here, including
        // caching the vector.
        let mut vec = Vec::new();
        for (req, allocation, allocation_index) in Zip::new((&self.requirements, &self.allocations, 0..)) {
            for (individual_device, individual_device_index) in Zip::new((allocation, 0..)) {
                for (input, individual_input_index) in Zip::new((&req.data.inputs, 0..)) {
                    vec.push(InputBinding {
                        allocation: allocation_index,
                        device: individual_device_index,
                        input: individual_input_index
                    })
                }
            }
        }
        vec
    }
}

/// The binding of an input capability to a specific device set.
struct InputBinding {
    /// The device set holding this input capability.
    /// Index in `app.allocations`.
    allocation: usize,

    /// The individual device holding this input capability.
    /// Index in `app.allocations[self.allocation]`.
    device: usize,

    /// The specific input holding this input capability.
    /// Index in `app.allocations[self.allocation][self.device]`.
    input: usize,
}

impl InputBinding {
    /// Get the device providing the InputBinding.
    fn get_individual_device(&self, app: &MonitorApp) -> Arc<Device> {
        app.allocations[self.allocation][self.device].data.clone()
    }

    /// Get the input capability for this binding.
    fn get_input(&self, app: &MonitorApp) -> InputCapability {
        app.requirements[self.allocation].data.inputs[self.input].data.clone()
    }

    /// Update the state attached to this input binding.
    fn set_state(&self, state: &mut MonitorTaskState, value: Option<Value>) {
        state.input_state[self.allocation][self.device][self.input] = value;
    }
}

/// The state of a given condition.
struct ConditionState {
    /// `true` if the conditions were met last time the state of the
    /// inputs changed. We use this to trigger an action only when
    /// conditions were previously unmet and are now met.
    are_conditions_met: bool,
    // FIXME: In the future, the cooldown should go here.
}

type InputState = Vec<Vec<Vec<Option<Value>>>>;
struct MonitorTaskState {
    /// The state of each trigger.
    ///
    /// Invariant: len() is the same as that of `self.app.rule`s
    trigger_condition_state: Vec<ConditionState>,

    /// For each device set allocated, for each individual device
    /// allocated, for each input watched by the app, the latest state
    /// received.
    ///
    /// Use `InputBinding` to access the data.
    input_state: InputState,

    /// A clone of the code being executed.
    app: MonitorApp,
}

/// The part of the monitor used to communicate between threads.
///
/// Kept separate to help the borrow checker understand what we're
/// accessing at any time.
struct MonitorComm {
    tx: Sender<MonitorOp>,
    rx: Receiver<MonitorOp>,
}

/// A MonitorApp currently being executed.
///
/// Each MonitorTask runs on its own thread.
struct MonitorTask {
    state: MonitorTaskState,
    comm: MonitorComm,
}

impl MonitorTask {

    /// Create a new MonitorTask.
    ///
    /// To initiate watching, use method `run()`.
    fn new(app: MonitorApp) -> Self {
        // Initialize condition state.
        let mut trigger_condition_state = Vec::with_capacity(app.rules.len());
        for _ in &app.rules {
            trigger_condition_state.push(ConditionState {
                are_conditions_met: false,
            });
        }
        assert_eq!(trigger_condition_state.len(), app.rules.len());
        
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
        let (tx, rx) = channel();

        MonitorTask {
            state: MonitorTaskState {
                trigger_condition_state: trigger_condition_state,
                input_state: full_input_state,
                app: app,
            },
            comm: MonitorComm {
                tx: tx,
                rx: rx,
            }
        }
    }

    /// Get a channel that may be used to send commands to the task.
    fn get_command_sender(&self) -> Sender<MonitorOp> {
        self.comm.tx.clone()
    }

    /// Execute the monitoring task.
    /// This currently expects to be executed in its own thread.
    fn run(&mut self) {
        let mut watcher = Watcher::new();
        let mut witnesses = Vec::new();

        for state_index in self.state.app.iter_state_index() {
            // FIXME: We currently use `Range::Any` for simplicity.
            // However, in most cases, we should be able to look inside
            // the condition to build a better `Range`.

            // FIXME: We currently monitor all the inputs that are
            // used by this rule, even if they are not part of the
            // condition (e.g. we monitor the camera all the time even
            // if we only need to trigger when an intruder enters).
            witnesses.push(
                watcher.add(
                    &state_index.get_individual_device(&self.state.app),
                    &state_index.get_input(&self.state.app),
                    &Range::Any,
                    |data| {
                        let _ignored = self.comm.tx.send(MonitorOp::Update {
                            data: data,
                            index: state_index,
                        });
                        // Ignore errors. If the thread is shutting
                        // down, it's ok to lose messages.
                    }));
        }

        for msg in &self.comm.rx {
            use self::MonitorOp::*;
            match msg {
                Update {
                    data,
                    index,
                } => {
                    // Update the state
                    index.set_state(&mut self.state, Some(data));

                    // Find out if we should execute triggers.
                    // FIXME: We could optimize this by finding out which triggers
                    // are tainted by the update and only rechecking these.
                    for (trigger, trigger_condition_state) in Zip::new((&self.state.app.rules, &mut self.state.trigger_condition_state)) {
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
                        let _ignored = self.comm.tx.send(MonitorOp::Execute {
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
                    return;
                }
            }
        }
    }
}

enum MonitorOp {
    Update{data: Value, index: InputBinding},
    Execute{state: InputState, commands: Vec<Command>},
    Stop
}

impl MonitorApp {
    ///
    /// Start executing the application.
    ///
    pub fn start(&mut self) {
        if self.command_sender.is_some() {
            return;
        }
        let mut task = MonitorTask::new(self.clone());
        self.command_sender = Some(task.get_command_sender());
        thread::spawn(move || {
            task.run();
        });
    }

    ///
    /// Stop the execution of the application.
    ///
    pub fn stop(&mut self) {
        match self.command_sender {
            None => {
                /* Nothing to stop */
                return;
            },
            Some(ref tx) => {
                // Shutdown the application, asynchronously.
                let _ignored = tx.send(MonitorOp::Stop);
                // Do not return.
            }
        }
        self.command_sender = None;
    }
}




/// Data labelled with a user-readable name.
#[derive(Clone)]
struct Named<T> where T: Clone {
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
    condition: Conjunction,

    /// Stuff to do once `condition` is met.
    execute: Vec<Command>,

    /// Minimal duration between two executions of the trigger.  If a
    /// duration was not picked by the developer, a reasonable default
    /// duration should be picked (e.g. 10 minutes).
    cooldown: Duration,
}

/// A conjunction (e.g. a "and") of conditions.
#[derive(Clone)]
struct Conjunction {
    /// The conjunction is true iff all of the following expressions evaluate to true.
    all: Vec<Condition>
}

impl Conjunction {
    fn is_met(&self, input_state: &InputState) -> bool {
        for condition in &self.all {
            if !condition.is_met(input_state) {
                return false;
            }
        }
        return true;
    }
}

/// An individual condition.
///
/// Conditions always take the form: "data received from sensor is in
/// given range".
///
/// A condition is true if *any* of the sensors allocated to this
/// requirement has yielded a value that is in the given range.
#[derive(Clone)]
struct Condition {
    requirement_index: usize,
    input_index: usize,
    range: Range,
}

impl Condition {
    /// Find out if *any* of the sensors allocated to this requirement
    /// has yielded a value that is in the given range.
    fn is_met(&self, input_state: &InputState) -> bool {
        for (individual_device, individual_device_index) in Zip::new((&input_state[self.requirement_index], 0..)) {
            if match individual_device[self.input_index] {
                None => { false /* We haven't received a measurement yet.*/ },
                Some(ref data) => {
                    use dependencies::Range::*;
                    use dependencies::Value::*;

                    match (data, &self.range) {
                        // Any always matches
                        (_, &Any) => true,
                        // Operations on bools and strings
                        (&Bool(ref b), &EqBool(ref b2)) => b == b2,
                        (&String(ref s), &EqString(ref s2)) => s == s2,

                        // Numbers. FIXME: Implement physical units.
                        (&Num(ref x), &Leq(ref max)) => x <= max,
                        (&Num(ref x), &Geq(ref min)) => min <= x,
                        (&Num(ref x), &BetweenEq{ref min, ref max}) => min <= x && x <= max,
                        (&Num(ref x), &OutOfStrict{ref min, ref max}) => x < min || max < x,

                        // Type errors don't match.
                        (&Bool(_), _) => false,
                        (&String(_), _) => false,
                        (_, &EqBool(_)) => false,
                        (_, &EqString(_)) => false,

                        // There is no such thing as a range on json or blob.
                        (&Json(_), _) |
                        (&Blob{..}, _) => false,
                    }
                }
            }
            {
                return true;
            }
        }
        return false;
    }
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

