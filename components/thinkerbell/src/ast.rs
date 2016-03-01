//! Definition of Thinkerbell scripts.
//!
//! Typical applications will not interact with script
//! objects. Rather, they will use module `parse` to parse a script
//! and module `run` to execute it.

use foxbox_taxonomy::values::*;
use foxbox_taxonomy::devices::*;
use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::util::Phantom;

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
/// Matchs always take the form: "data received from getter channel
/// is in given range".
///
/// A condition is true if *any* of the corresponding getter channels
/// yielded a value that is in the given range.
#[derive(Serialize, Deserialize)]
pub struct Match<Ctx> where Ctx: Context {
    /// The set of getters to watch. Note that the set of getters may
    /// change (e.g. when devices are added/removed) without rebooting
    /// the script.
    pub source: Vec<GetterSelector>,

    /// The kind of channel expected from `source`, e.g. "the current
    /// time of day", "is the door opened?", etc. During compilation,
    /// we make sure that we restrict to the elements of `source` that
    /// offer `kind`.
    pub kind: ChannelKind,

    /// The range of values for which the condition is considered met.
    /// During compilation, we check that the type of `range` is
    /// compatible with that of `getter`.
    pub range: Range,

    #[serde(default)]
    #[allow(dead_code)]
    pub phantom: Phantom<Ctx>,
}


/// Stuff to actually do. In practice, this means placing calls to devices.
#[derive(Serialize, Deserialize)]
pub struct Statement<Ctx> where Ctx: Context {
    /// The set of setters to which to send a command. Note that the
    /// set of setters may change (e.g. when devices are
    /// added/removed) without rebooting the script.
    pub destination: Vec<SetterSelector>,

    /// Data to send to the resource. During compilation, we check
    /// that the type of `value` is compatible with that of
    /// `destination`.
    pub value: Value,

    /// The kind of channel expected from `destination`, e.g. "close
    /// the door", "set the temperature", etc. During compilation, we
    /// make sure that we restrict to the elements of `destination` that
    /// offer `kind`.
    pub kind: ChannelKind,

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
