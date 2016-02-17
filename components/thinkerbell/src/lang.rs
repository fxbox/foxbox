#![allow(unused_variables)]
#![allow(dead_code)]

/// Basic structure of a Monitor (aka Server App)
///
/// Monitors are designed so that the FoxBox can offer a simple
/// IFTTT-style Web UX to let users write their own scripts. More
/// complex monitors can installed from the web from a master device
/// (i.e. the user's cellphone or smart tv).

use dependencies::DevEnv;
use values::{Value, Range};

use std::collections::HashMap;
use std::marker::PhantomData;


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
pub struct Script<Ctx, Env> where Env: DevEnv, Ctx: Context {
    /// Authorization, author, description, update url, version, ...
    pub metadata: (), // FIXME: Implement

    /// Monitor applications have sets of requirements (e.g. "I need a
    /// camera"), which are allocated to actual resources through the
    /// UX. Re-allocating resources may be requested by the user, the
    /// foxbox, or an application, e.g. when replacing a device or
    /// upgrading the app.
    pub requirements: Vec<Requirement<Ctx, Env>>,

    /// Resources actually allocated for each requirement.
    /// This must have the same size as `requirements`.
    pub allocations: Vec<Resource<Ctx, Env>>,

    /// A set of rules, stating what must be done in which circumstance.
    pub rules: Vec<Trigger<Ctx, Env>>,
}

pub struct Resource<Ctx, Env> where Env: DevEnv, Ctx: Context {
    pub devices: Vec<Env::Device>,
    pub phantom: PhantomData<Ctx>,
}


/// A resource needed by this application. Typically, a definition of
/// device with some input our output capabilities.
pub struct Requirement<Ctx, Env> where Env: DevEnv, Ctx: Context {
    /// The kind of resource, e.g. "a flashbulb".
    pub kind: Env::DeviceKind,

    /// Input capabilities we need from the device, e.g. "the time of
    /// day", "the current temperature".
    pub inputs: Vec<Env::InputCapability>,

    /// Output capabilities we need from the device, e.g. "play a
    /// sound", "set luminosity".
    pub outputs: Vec<Env::OutputCapability>,
    
    pub phantom: PhantomData<Ctx>,
    // FIXME: We may need cooldown properties.
}

/// A single trigger, i.e. "when some condition becomes true, do
/// something".
pub struct Trigger<Ctx, Env> where Env: DevEnv, Ctx: Context {
    /// The condition in which to execute the trigger.
    pub condition: Conjunction<Ctx, Env>,

    /// Stuff to do once `condition` is met.
    pub execute: Vec<Statement<Ctx, Env>>,

    /*
    /// Minimal duration between two executions of the trigger.  If a
    /// duration was not picked by the developer, a reasonable default
    /// duration should be picked (e.g. 10 minutes).
    FIXME: Implement
    pub cooldown: Duration,
     */
}

/// A conjunction (e.g. a "and") of conditions.
pub struct Conjunction<Ctx, Env> where Env: DevEnv, Ctx: Context {
    /// The conjunction is true iff all of the following expressions evaluate to true.
    pub all: Vec<Condition<Ctx, Env>>,
    pub state: Ctx::ConditionState,
}

/// An individual condition.
///
/// Conditions always take the form: "data received from sensor is in
/// given range".
///
/// A condition is true if *any* of the sensors allocated to this
/// requirement has yielded a value that is in the given range.
pub struct Condition<Ctx, Env> where Env: DevEnv, Ctx: Context {
    pub input: Ctx::InputSet,
    pub capability: Env::InputCapability,
    pub range: Range,
    pub state: Ctx::ConditionState,
}


/// Stuff to actually do. In practice, this means placing calls to devices.
pub struct Statement<Ctx, Env> where Env: DevEnv, Ctx: Context {
    /// The resource to which this command applies.  e.g. "all
    /// heaters", "a single communication channel", etc.
    pub destination: Ctx::OutputSet,

    /// The action to execute on the resource.
    pub action: Env::OutputCapability,

    /// Data to send to the resource.
    pub arguments: HashMap<String, Expression<Ctx, Env>>
}

pub struct InputSet<Ctx, Env> where Env: DevEnv, Ctx: Context {
    /// The set of inputs from which to grab the value, i.e.
    /// all the inputs matching some condition.
    pub condition: Condition<Ctx, Env>,

    /// The value to grab.
    pub capability: Env::InputCapability,
}

/// A value that may be sent to an output.
pub enum Expression<Ctx, Env> where Env: DevEnv, Ctx: Context {
    /// A dynamic value, which must be read from one or more inputs.
    // FIXME: Not ready yet
    Input(InputSet<Ctx, Env>),

    /// A constant value.
    Value(Value),

    /// More than a single value.
    Vec(Vec<Expression<Ctx, Env>>)
}

/// A manner of representing internal nodes.
pub trait Context {
    /// A representation of one or more input devices.
    type InputSet;

    /// A representation of one or more output devices.
    type OutputSet;

    /// A representation of the current state of a condition.
    type ConditionState;
}

/// A Context used to represent a script that hasn't been compiled
/// yet. Rather than pointing to specific device + capability, inputs
/// and outputs are numbers that are meaningful only in the AST.
pub struct UncheckedCtx;
impl Context for UncheckedCtx {
    /// In this implementation, each input is represented by its index
    /// in the array of allocations.
    type InputSet = usize;

    /// In this implementation, each output is represented by its
    /// index in the array of allocations.
    type OutputSet = usize;

    /// In this implementation, conditions have no state.
    type ConditionState = ();
}

/// A DevEnv used to represent a script that hasn't been
/// compiled yet. Rather than having typed devices, capabilities,
/// etc. everything is represented by a string.
pub struct UncheckedEnv;
impl DevEnv for UncheckedEnv {
    type Device = String;
    type DeviceKind = String;
    type InputCapability = String;
    type OutputCapability = String;
}
