//! Definition of Thinkerbell scripts.
//!
//! Typical applications will not interact with script
//! objects. Rather, they will use module `parse` to parse a script
//! and module `run` to execute it.

use values::Range;
use util::Phantom;

use fxbox_taxonomy::values::Value;
use fxbox_taxonomy::devices::*;
use fxbox_taxonomy::selector::*;

use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Error};

/// A thinkerbell script.
#[derive(Serialize, Deserialize)]
pub struct Script<Ctx> where Ctx: Context {
    /// A set of rules, stating what must be done in which circumstance.
    pub rules: Vec<Rule<Ctx>>,

    #[serde(default)]
    #[allow(dead_code)]
    pub phantom: Phantom<Ctx>,
}

/// A single rule, i.e. "when some condition becomes true, do
/// something".
#[derive(Serialize, Deserialize)]
pub struct Rule<Ctx> where Ctx: Context {
    /// The condition in which to execute the trigger. The condition
    /// is matched once *all* the `Match` branches are true. Whenever
    /// `conditions` was false and becomes true, we execute `execute`.
    pub conditions: Vec<Match<Ctx>>,

    /// Stuff to do once `condition` is met.
    pub execute: Vec<Statement<Ctx>>,

    #[serde(default)]
    #[allow(dead_code)]
    pub phantom: Phantom<Ctx>,
}

/// An individual match.
///
/// Matchs always take the form: "data received from input service
/// is in given range".
///
/// A condition is true if *any* of the corresponding input services
/// yielded a value that is in the given range.
#[derive(Serialize, Deserialize)]
pub struct Match<Ctx> where Ctx: Context {
    /// The set of inputs to watch. Note that the set of inputs may
    /// change (e.g. when devices are added/removed) without rebooting
    /// the script.
    pub source: Vec<InputSelector>,

    /// The kind of service expected from `source`, e.g. "the current
    /// time of day", "is the door opened?", etc. During compilation,
    /// we make sure that we restrict to the elements of `source` that
    /// offer `kind`.
    pub kind: ServiceKind,

    /// The range of values for which the condition is considered met.
    /// During compilation, we check that the type of `range` is
    /// compatible with that of `input`.
    pub range: Range,

    #[serde(default)]
    #[allow(dead_code)]
    pub phantom: Phantom<Ctx>,
}


/// Stuff to actually do. In practice, this means placing calls to devices.
#[derive(Serialize, Deserialize)]
pub struct Statement<Ctx> where Ctx: Context {
    /// The set of outputs to which to send a command. Note that the
    /// set of outputs may change (e.g. when devices are
    /// added/removed) without rebooting the script.
    pub destination: Vec<OutputSelector>,

    /// Data to send to the resource. During compilation, we check
    /// that the type of `value` is compatible with that of
    /// `destination`.
    pub value: Value,

    /// The kind of service expected from `destination`, e.g. "close
    /// the door", "set the temperature", etc. During compilation, we
    /// make sure that we restrict to the elements of `destination` that
    /// offer `kind`.
    pub kind: ServiceKind,

    #[serde(default)]
    #[allow(dead_code)]
    pub phantom: Phantom<Ctx>,
}


/// A manner of representing internal nodes.
///
/// Two data structures implement `Context`:
///
/// - `UncheckedCtx`, designed to mark the fact that a script has not
/// been compiled/checked yet and must not be executed;
/// - `compile::CompiledCtx`, designed to mark the fact that a script
/// has been compiled and can be executed.
pub trait Context: Serialize + Deserialize + Default {
}

/// A Context used to represent a script that hasn't been compiled
/// yet.
#[derive(Default, Serialize, Deserialize)]
pub struct UncheckedCtx;
impl Context for UncheckedCtx {
}
