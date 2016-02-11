
#![allow(unused_variables)]
#![allow(dead_code)]

/// Basic structure of a Monitor (aka Server App, aka wtttttt)
///
/// Monitors are designed so that the FoxBox can offer a simple
/// IFTTT-style Web UX to let users write their own scripts. More
/// complex monitors can installed from the web from a master device
/// (i.e. the user's cellphone or smart tv).

use dependencies::{DeviceKind, InputCapability, OutputCapability, Device, Range, Value, Watcher};

use std::collections::HashMap;
use std::sync::{Arc, RwLock}; // FIXME: Investigate if we really need so many instances of Arc.
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

extern crate chrono;
use self::chrono::{Duration, DateTime, UTC};

extern crate rustc_serialize;
use self::rustc_serialize::json::Json;

///
/// # Definition of the AST
///


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
pub struct Script<Input, Output, ConditionState> {
    /// Authorization, author, description, update url, version, ...
    metadata: (), // FIXME: Implement

    /// Monitor applications have sets of requirements (e.g. "I need a
    /// camera"), which are allocated to actual resources through the
    /// UX. Re-allocating resources may be requested by the user, the
    /// foxbox, or an application, e.g. when replacing a device or
    /// upgrading the app.
    requirements: Vec<Arc<Requirement>>,

    /// Resources actually allocated for each requirement.
    allocations: Vec<Resource<Input, Output>>,

    /// A set of rules, stating what must be done in which circumstance.
    rules: Vec<Trigger<Input, Output, ConditionState>>,
}

#[derive(Clone)]
struct Resource<Input, Output> {
    devices: Vec<Device>,
    sensor: Option<Input>,
    effector: Option<Output>,
}


/// A resource needed by this application. Typically, a definition of
/// device with some input our output capabilities.
#[derive(Clone)]
struct Requirement {
    /// The kind of resource, e.g. "a flashbulb".
    kind: DeviceKind,

    /// Input capabilities we need from the device, e.g. "the time of
    /// day", "the current temperature".
    inputs: Vec<InputCapability>,

    /// Output capabilities we need from the device, e.g. "play a
    /// sound", "set luminosity".
    outputs: Vec<OutputCapability>,
    
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
struct Trigger<Input, Output, ConditionState> {
    /// The condition in which to execute the trigger.
    condition: Conjunction<Input, ConditionState>,

    /// Stuff to do once `condition` is met.
    execute: Vec<Statement<Input, Output, ConditionState>>,

    /// Minimal duration between two executions of the trigger.  If a
    /// duration was not picked by the developer, a reasonable default
    /// duration should be picked (e.g. 10 minutes).
    cooldown: Duration,
}

/// A conjunction (e.g. a "and") of conditions.
#[derive(Clone)]
struct Conjunction<Input, ConditionState> {
    /// The conjunction is true iff all of the following expressions evaluate to true.
    all: Vec<Condition<Input, ConditionState>>,
    state: ConditionState,
}

/// An individual condition.
///
/// Conditions always take the form: "data received from sensor is in
/// given range".
///
/// A condition is true if *any* of the sensors allocated to this
/// requirement has yielded a value that is in the given range.
#[derive(Clone)]
struct Condition<Input, ConditionState> {
    input: Input,
    capability: InputCapability,
    range: Range,
    state: ConditionState,
}


/// Stuff to actually do. In practice, this maps to a REST call.
#[derive(Clone)]
struct Statement<Input, Output, ConditionState> {
    /// The resource to which this command applies.  e.g. "all
    /// heaters", "a single communication channel", etc.
    destination: Output,

    /// The action to execute on the resource.
    action: OutputCapability,

    /// Data to send to the resource.
    arguments: HashMap<String, Expression<Input, ConditionState>>
}

#[derive(Clone)]
struct InputSet<Input, ConditionState> {
    /// The set of inputs from which to grab the value.
    condition: Condition<Input, ConditionState>,
    /// The value to grab.
    capability: InputCapability,
}

/// A value that may be sent to an output.
#[derive(Clone)]
enum Expression<Input, ConditionState> {
    /// A dynamic value, which must be read from one or more inputs.
    // FIXME: Not ready yet
    Input(InputSet<Input, ConditionState>),

    /// A constant value.
    Value(Value),

    /// More than a single value.
    Vec(Vec<Expression<Input, ConditionState>>)
}

///
/// # Launching and running the script
///

/// A script ready to be executed.
/// Each script is meant to be executed in an individual thread.
struct ExecutionTask {
    /// The current state of execution the script.
    state: Script<Arc<InputEnv>, Arc<OutputEnv>, ConditionEnv>,

    /// Communicating with the thread running script.
    tx: Sender<ExecutionOp>,
    rx: Receiver<ExecutionOp>,
}


/// Data, labelled with its latest update.
struct DatedData {
    updated: DateTime<UTC>,
    data: Value,
}

/// A single input device, ready to use, with its latest known state.
struct SingleInputEnv {
    device: Device,
    state: RwLock<Option<DatedData>>
}
type InputEnv = Vec<SingleInputEnv>;

/// A single output device, ready to use.
struct SingleOutputEnv {
    device: Device
}
type OutputEnv = Vec<SingleOutputEnv>;

struct ConditionEnv {
    is_met: bool
}

enum ExecutionOp {
    /// An input has been updated, time to check if we have triggers
    /// ready to be executed.
    Update,

    /// Time to stop executing the script.
    Stop
}


impl ExecutionTask {
    /// Create a new execution task.
    ///
    /// The caller is responsible for spawning a new thread and
    /// calling `run()`.
    fn new<'a>(script: &'a Script<usize, usize, ()>) -> Self {
        // Prepare the script for execution:
        // - replace instances of Input with InputEnv, which map
        //   to a specific device and cache the latest known value
        //   on the input.
        // - replace instances of Output with OutputEnv
        let precompiler = Precompiler::new(script);
        let bound = script.rebind(&precompiler);
        
        let (tx, rx) = channel();
        ExecutionTask {
            state: bound,
            rx: rx,
            tx: tx
        }
    }

    /// Get a channel that may be used to send commands to the task.
    fn get_command_sender(&self) -> Sender<ExecutionOp> {
        self.tx.clone()
    }

    /// Execute the monitoring task.
    /// This currently expects to be executed in its own thread.
    fn run(&mut self) {
        let mut watcher = Watcher::new();
        let mut witnesses = Vec::new();

        // Start listening to all inputs that appear in conditions.
        // Some inputs may appear only in expressions, so we are
        // not interested in their value.
        for rule in &self.state.rules  {
            for condition in &rule.condition.all {
                for single in &*condition.input {
                    witnesses.push(
                        // We can end up watching several times the
                        // same device + capability + range.  For the
                        // moment, we do not attempt to optimize
                        // either I/O (which we expect will be
                        // optimized by `watcher`) or condition
                        // checking (which we should eventually
                        // optimize, if we find out that we end up
                        // with large rulesets).
                        watcher.add(
                            &single.device,
                            &condition.capability,
                            &condition.range,
                            |value| {
                                // One of the inputs has been updated.
                                *single.state.write().unwrap() = Some(DatedData {
                                    updated: UTC::now(),
                                    data: value
                                });
                                // Note that we can unwrap() safely,
                                // as it fails only if the thread is
                                // already in panic.

                                // Find out if we should execute one of the
                                // statements of the trigger.
                                let _ignored = self.tx.send(ExecutionOp::Update);
                                // If the thread is down, it is ok to ignore messages.
                            }));
                    }
            }
        }

        // FIXME: We are going to end up with stale data in some inputs.
        // We need to find out how to get rid of it.
        // FIXME(2): We now have dates.

        // Now, start handling events.
        for msg in &self.rx {
            use self::ExecutionOp::*;
            match msg {
                Stop => {
                    // Leave the loop.
                    // The watcher and the witnesses will be cleaned up on exit.
                    // Any further message will be ignored.
                    return;
                }

                Update => {
                    // Find out if we should execute triggers.
                    for mut rule in &mut self.state.rules {
                        let is_met = rule.is_met();
                        if !(is_met.new && !is_met.old) {
                            // We should execute the trigger only if
                            // it was false and is now true. Here,
                            // either it was already true or it isn't
                            // false yet.
                            continue;
                        }

                        // Conditions were not met, now they are, so
                        // it is time to start executing.

                        // FIXME: We do not want triggers to be
                        // triggered too often. Handle cooldown.
                        
                        for statement in &rule.execute {
                            // FIXME: Execute
                        }
                    }
                }
            }
        }
    }
}

///
/// # Evaluating conditions
///

struct IsMet {
    old: bool,
    new: bool,
}

impl<O> Trigger<Arc<InputEnv>, O, ConditionEnv> {
    fn is_met(&mut self) -> IsMet {
        self.condition.is_met()
    }
}

impl Conjunction<Arc<InputEnv>, ConditionEnv> {
    /// For a conjunction to be true, all its components must be true.
    fn is_met(&mut self) -> IsMet {
        let old = self.state.is_met.clone();
        let mut new = true;

        for mut single in &mut self.all {
            if !single.is_met().new {
                new = false;
                // Don't break. We want to make sure that we update
                // `is_met` of all individual conditions.
            }
        }
        self.state.is_met = new;
        IsMet {
            old: old,
            new: new,
        }
    }
}

impl Condition<Arc<InputEnv>, ConditionEnv> {
    /// Determine if one of the devices serving as input for this
    /// condition meets the condition.
    fn is_met(&mut self) -> IsMet {
        let old = self.state.is_met.clone();
        let mut new = false;
        for single in &*self.input {
            // This will fail only if the thread has already panicked.
            let state = single.state.read().unwrap();
            let is_met = match *state {
                None => { false /* We haven't received a measurement yet.*/ },
                Some(ref data) => {
                    use dependencies::Range::*;
                    use dependencies::Value::*;

                    match (&data.data, &self.range) {
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
            };
            if is_met {
                new = true;
                break;
            }
        }

        self.state.is_met = new;
        IsMet {
            old: old,
            new: new,
        }
    }
}

///
/// # Changing the kind of variable used.
///

trait Rebinder<Input, Output, Condition, Input2, Output2, Condition2> where
    Input2: Clone,
    Output2: Clone
{
    fn alloc_input(&self, &Input) -> Input2;
    fn alloc_output(&self, &Output) -> Output2;
    fn alloc_condition(&self, &Condition) -> Condition2;
}

impl<Input, Output, Condition> Script<Input, Output, Condition> {
    fn rebind<Input2, Output2, Condition2>(&self, rebinder: &Rebinder<Input, Output, Condition, Input2, Output2, Condition2>) -> Script<Input2, Output2, Condition2> where
        Input2: Clone,
        Output2: Clone
    {
        let rules = self.rules.iter().map(|ref rule| {
            rule.rebind(rebinder)
        }).collect();

        let allocations = self.allocations.iter().map(|ref res| {
            Resource {
                devices: res.devices.clone(),
                sensor: res.sensor.as_ref().map(|ref old| rebinder.alloc_input(old).clone()),
                effector: res.effector.as_ref().map(|ref old| rebinder.alloc_output(old).clone()),
            }
        }).collect();

        Script {
            metadata: self.metadata.clone(),
            requirements: self.requirements.clone(),
            allocations: allocations,
            rules: rules,
        }
    }
}

impl<Input, Output, Condition> Trigger<Input, Output, Condition> {
    fn rebind<Input2, Output2, Condition2>(&self, rebinder: &Rebinder<Input, Output, Condition, Input2, Output2, Condition2>) -> Trigger<Input2, Output2, Condition2> where
        Input2: Clone,
        Output2: Clone
    {
        let execute = self.execute.iter().map(|ref ex| {
            ex.rebind(rebinder)
        }).collect();
        Trigger {
            cooldown: self.cooldown.clone(),
            execute: execute,
            condition: self.condition.rebind(rebinder),
        }
    }
}


impl<Input, ConditionState> Conjunction<Input, ConditionState> {
    fn rebind<Output, Input2, Output2, Condition2>(&self, rebinder: &Rebinder<Input, Output, ConditionState, Input2, Output2, Condition2>) -> Conjunction<Input2, Condition2> where
        Input2: Clone,
        Output2: Clone {
        Conjunction {
            all: self.all.iter().map(|c| c.rebind(rebinder)).collect(),
            state: rebinder.alloc_condition(&self.state),
        }
    }
}

impl<Input, ConditionState> Condition<Input, ConditionState> {
    fn rebind<Output, Input2, Output2, Condition2>(&self, rebinder: &Rebinder<Input, Output, ConditionState, Input2, Output2, Condition2>) -> Condition<Input2, Condition2> where
        Input2: Clone,
        Output2: Clone {
        Condition {
            range: self.range.clone(),
            capability: self.capability.clone(),
            input: rebinder.alloc_input(&self.input).clone(),
            state: rebinder.alloc_condition(&self.state),
        }
    }
}

impl <Input, Output, ConditionState> Statement<Input, Output, ConditionState> {
    fn rebind<Input2, Output2, Condition2>(&self, rebinder: &Rebinder<Input, Output, ConditionState, Input2, Output2, Condition2>) -> Statement<Input2, Output2, Condition2> where
        Input2: Clone,
        Output2: Clone {
            let arguments = self.arguments.iter().map(|(key, value)| {
                (key.clone(), value.rebind(rebinder))
            }).collect();
            Statement {
                destination: rebinder.alloc_output(&self.destination).clone(),
                action: self.action.clone(),
                arguments: arguments
            }
        }
}

impl<Input, ConditionState> Expression<Input, ConditionState> {
    fn rebind<Output, Input2, Output2, Condition2>(&self, rebinder: &Rebinder<Input, Output, ConditionState, Input2, Output2, Condition2>) -> Expression<Input2, Condition2> where
        Input2: Clone,
        Output2: Clone 
    {
        use self::Expression::*;
        match *self {
            Value(ref v) => Value(v.clone()),
            Vec(ref v) => Vec(v.iter().map(|x| x.rebind(rebinder)).collect()),
            //            Input(ref input) => Input(rebinder.alloc_input(input).clone()),
            Input(_) => panic!("Not impl implemented yet")
        }
    }
}


///
/// # Precompilation
///

struct Precompiler<'a> {
    script: &'a Script<usize, usize, ()>
}

impl<'a> Precompiler<'a> {
    fn new(source: &'a Script<usize, usize, ()>) -> Self {
        Precompiler {
            script: source
        }
    }
}
impl<'a> Rebinder<usize, usize, (), Arc<InputEnv>, Arc<OutputEnv>, ConditionEnv> for Precompiler<'a> {
    fn alloc_input(&self, &input: &usize) -> Arc<InputEnv> {
        Arc::new(
            self.script.allocations[input].devices.iter().map(|device| {
                SingleInputEnv {
                    device: device.clone(),
                    state: RwLock::new(None),
                }
            }).collect())
    }

    fn alloc_output(&self, &output: &usize) -> Arc<OutputEnv> {
        Arc::new(
            self.script.allocations[output].devices.iter().map(|device| {
                SingleOutputEnv {
                    device: device.clone()
                }
            }).collect())
    }

    fn alloc_condition(&self, _: &()) -> ConditionEnv {
        ConditionEnv {
            is_met: false
        }
    }
}

/*
impl Script {
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

*/
