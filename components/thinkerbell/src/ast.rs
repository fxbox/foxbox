//! Definition of Thinkerbell scripts.
//!
//! Typical applications will not interact with script
//! objects. Rather, they will use module `parse` to parse a script
//! and module `run` to execute it.

use parse::*;

use foxbox_taxonomy::values::*;
use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::services::*;

use std::marker::PhantomData;

use serde_json::value::Value as JSON;
use serde_json;

/// A thinkerbell script.
pub struct Script<Ctx> where Ctx: Context {
    /// A set of rules, stating what must be done in which circumstance.
    pub rules: Vec<Rule<Ctx>>,

    pub phantom: PhantomData<Ctx>,
}

impl Script<UncheckedCtx> {
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let path = Path::new();
        match serde_json::from_str(s) {
            Err(err) => Err(ParseError::json(err)),
            Ok(source) => <Self as Parse<Script<UncheckedCtx>>>::parse(path, source)
        }
    }
}

impl Parse<Script<UncheckedCtx>> for Script<UncheckedCtx> {
    fn parse(path: Path, mut source: JSON) -> Result<Self, ParseError> {
        let rules = try!(Rule::take_vec(path.push(""), &mut source, "rules"));
        try!(check_no_more_fields(path, source));
        Ok(Script {
            rules: rules,
            phantom: PhantomData
        })
    }
}

/// A single rule, i.e. "when some condition becomes true, do
/// something".
pub struct Rule<Ctx> where Ctx: Context {
    /// The condition in which to execute the trigger. The condition
    /// is matched once *all* the `Match` branches are true. Whenever
    /// `conditions` was false and becomes true, we execute `execute`.
    pub conditions: Vec<Match<Ctx>>,

    /// Stuff to do once `condition` is met.
    pub execute: Vec<Statement<Ctx>>,

    pub phantom: PhantomData<Ctx>,
}
impl Parse<Rule<UncheckedCtx>> for Rule<UncheckedCtx> {
    fn parse(path: Path, mut source: JSON) -> Result<Self, ParseError> {
        let conditions = try!(Match::take_vec(path.push("conditions"), &mut source, "conditions"));
        let execute = try!(Statement::take_vec(path.push("execute"), &mut source, "execute"));
        try!(check_no_more_fields(path, source));
        Ok(Rule {
            conditions: conditions,
            execute: execute,
            phantom: PhantomData,
        })
    }
}
/// An individual match.
///
/// Matchs always take the form: "data received from getter channel
/// is in given range".
///
/// A condition is true if *any* of the corresponding getter channels
/// yielded a value that is in the given range.
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

    /// If specified, the values must remain in the `range` for at least
    /// `duration` before the match is considered valid. This is useful
    /// for sensors that may oscillate around a threshold or for detecting
    /// e.g. that a door has been forgotten open.
    pub duration: Option<Duration>,

    pub phantom: PhantomData<Ctx>,
}
impl Parse<Match<UncheckedCtx>> for Match<UncheckedCtx> {
    fn parse(path: Path, mut source: JSON) -> Result<Self, ParseError> {
        let sources = try!(GetterSelector::take_vec(path.push("source"), &mut source, "source"));
        let kind = try!(ChannelKind::take(path.push("kind"), &mut source, "kind"));
        let range = try!(Range::take(path.push("range"), &mut source, "range"));
        let duration = match Duration::take(path.push("range"), &mut source, "duration") {
            Err(ParseError::MissingField {..}) => None,
            Err(err) => return Err(err),
            Ok(ok) => Some(ok)
        };
        try!(check_no_more_fields(path, source));
        Ok(Match {
            source: sources,
            kind: kind,
            range: range,
            duration: duration,
            phantom: PhantomData,
        })
    }
}

/// Stuff to actually do. In practice, this means placing calls to devices.
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

    pub phantom: PhantomData<Ctx>,
}
impl Parse<Statement<UncheckedCtx>> for Statement<UncheckedCtx> {
    fn parse(path: Path, mut source: JSON) -> Result<Self, ParseError> {
        let destination = try!(SetterSelector::take_vec(path.push("destination"), &mut source, "destination"));
        let kind = try!(ChannelKind::take(path.push("kind"), &mut source, "kind"));
        let value = try!(Value::take(path.push("value"), &mut source, "value"));
        try!(check_no_more_fields(path, source));
        Ok(Statement {
            destination: destination,
            value: value,
            kind: kind,
            phantom: PhantomData,
        })
    }
}


/// A manner of representing internal nodes.
///
/// Two data structures implement `Context`:
///
/// - `UncheckedCtx`, designed to mark the fact that a script has not
/// been compiled/checked yet and must not be executed;
/// - `compile::CompiledCtx`, designed to mark the fact that a script
/// has been compiled and can be executed.
pub trait Context {
}

/// A Context used to represent a script that hasn't been compiled
/// yet.
pub struct UncheckedCtx;
impl Context for UncheckedCtx {
}

