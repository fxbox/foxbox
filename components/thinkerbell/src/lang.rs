
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
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

extern crate itertools;
use self::itertools::Zip;

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
struct Script<Input, Output, ConditionState> {
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
    execute: Vec<Statement<Input, Output>>,

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
struct Statement<Input, Output> {
    /// The resource to which this command applies,
    /// as an index in Trigger.requirements/allocations.
    destination: Output,

    action: OutputCapability,

    arguments: HashMap<String, Expression<Input>>
}

/// A value that may be sent to an output.
#[derive(Clone)]
enum Expression<Input> {
    /// A dynamic value, which must be read from an input.
    Input(Input),

    /// A constant value.
    Value(Value)
}

///
/// # Launching and running the script
///

struct SingleInputEnv {
    device: Device,
    state: RwLock<Option<Value>>
}
type InputEnv = Vec<SingleInputEnv>;

struct SingleOutputEnv {
    device: Device
}
type OutputEnv = Vec<SingleOutputEnv>;

struct ConditionEnv {
    is_met: bool
}

struct ExecutionTask {
    state: Script<Arc<InputEnv>, Arc<OutputEnv>, ConditionEnv>,
    tx: Sender<ExecutionOp>,
    rx: Receiver<ExecutionOp>,
}

enum ExecutionOp {
    Update,
//    Execute {state: InputState, commands: Vec<Statement>},
    Stop
}


impl ExecutionTask {
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

        let mut instant : u64 = 0;

        // Start listening to all inputs that appear in conditions.
        // Some inputs may appear only in expressions, so we are
        // not interested in their value.
        for rule in &self.state.rules  {
            for condition in &rule.condition.all {
                for single in &*condition.input {
                    witnesses.push(
                        // We can be watching several times the same device + capability + range.
                        // For the moment, we assume that the watcher will optimize the I/O for us.
                        // For the time being, we do not attempt to optimize condition checking.
                        watcher.add(
                            &single.device,
                            &condition.capability,
                            &condition.range,
                            |value| {
                                // One of the inputs has been updated.
                                *single.state.write().unwrap() = Some(value);
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
                        
                        // FIXME: Handle cooldown.

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
        let mut old = self.state.is_met.clone();
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
        let mut old = self.state.is_met.clone();
        let mut new = false;
        for single in &*self.input {
            // This will fail only if the thread has already panicked.
            let state = single.state.read().unwrap();
            let is_met = match *state {
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

impl <Input, Output> Statement<Input, Output> {
    fn rebind<Condition, Input2, Output2, Condition2>(&self, rebinder: &Rebinder<Input, Output, Condition, Input2, Output2, Condition2>) -> Statement<Input2, Output2> where
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

impl<Input> Expression<Input> {
    fn rebind<Condition, Output, Input2, Output2, Condition2>(&self, rebinder: &Rebinder<Input, Output, Condition, Input2, Output2, Condition2>) -> Expression<Input2> where
        Input2: Clone,
        Output2: Clone 
    {
        use self::Expression::*;
        match *self {
            Input(ref input) => Input(rebinder.alloc_input(input).clone()),
            Value(ref v) => Value(v.clone())
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


impl Condition {
    /// Find out if *any* of the sensors allocated to this requirement
    /// has yielded a value that is in the given range.
    fn is_met(&self, input_state: &InputState) -> bool {
    }
}
*/



/*
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


    /// Either `None` if the application is not launched, or `Some(tx)`,
    /// where `tx` is a channel used to communicate with the thread
    /// running the application.
    command_sender: Option<Sender<MonitorOp>>,
}


impl<'a> Script {
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
    fn get_individual_device(&self, app: &Script) -> Arc<Device> {
        app.allocations[self.allocation][self.device].data.clone()
    }

    /// Get the input capability for this binding.
    fn get_input(&self, app: &Script) -> InputCapability {
        app.requirements[self.allocation].data.inputs[self.input].data.clone()
    }

    /// Update the state attached to this input binding.
    fn set_state(&self, state: &mut MonitorTaskState, value: Option<Value>) {
        state.input_state[self.allocation][self.device][self.input] = value;
    }
}
 */
/*
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
    app: Script,
}

/// The part of the monitor used to communicate between threads.
///
/// Kept separate to help the borrow checker understand what we're
/// accessing at any time.
struct MonitorComm {
    tx: Sender<MonitorOp>,
    rx: Receiver<MonitorOp>,
}

/// A Script currently being executed.
///
/// Each MonitorTask runs on its own thread.
struct MonitorTask {
    state: MonitorTaskState,
    comm: MonitorComm,
}
*/
/*
impl MonitorTask {

    /// Create a new MonitorTask.
    ///
    /// To initiate watching, use method `run()`.
    fn new(app: Script<?>, ) -> Self {
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

 */
/*
enum MonitorOp {
    Update {data: Value, index: InputBinding},
    Execute {state: InputState, commands: Vec<Statement>},
    Stop
}
*/
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
