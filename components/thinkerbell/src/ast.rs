//! Definition of Thinkerbell scripts.
//!
//! Typical applications will not interact with script
//! objects. Rather, they will use module `parse` to parse a script
//! and module `run` to execute it.

use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::*;

use std::marker::PhantomData;

/// A thinkerbell script.
pub struct Script<Ctx> where Ctx: Context {
    /// A set of rules, stating what must be done in which circumstance.
    pub rules: Vec<Rule<Ctx>>,

    pub phantom: PhantomData<Ctx>,
}

impl Parser<Script<UncheckedCtx>> for Script<UncheckedCtx> {
    fn description() -> String {
        "Script".to_owned()
    }
    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let rules = try!(path.push("", |path| Rule::take_vec(path, source, "rules")));
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
impl Parser<Rule<UncheckedCtx>> for Rule<UncheckedCtx> {
    fn description() -> String {
        "Rule".to_owned()
    }

    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let conditions = try!(path.push("conditions",
            |path| Match::take_vec(path, source, "conditions"))
        );
        let execute = try!(path.push("execute",
            |path| Statement::take_vec(path, source, "execute"))
        );
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
impl Parser<Match<UncheckedCtx>> for Match<UncheckedCtx> {
    fn description() -> String {
        "Match".to_owned()
    }

    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let sources = try!(path.push("source",
            |path| GetterSelector::take_vec(path, source, "source"))
        );
        let kind = try!(path.push("kind",
            |path| ChannelKind::take(path, source, "kind"))
        );
        let range = try!(path.push("range",
            |path| Range::take(path, source, "range"))
        );
        let duration = match path.push("range",
            |path| Duration::take(path, source, "duration"))
        {
            Err(ParseError::MissingField {..}) => None,
            Err(err) => return Err(err),
            Ok(ok) => Some(ok)
        };
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
impl Parser<Statement<UncheckedCtx>> for Statement<UncheckedCtx> {
    fn description() -> String {
        "Parser".to_owned()
    }

    fn parse(path: Path, source: &mut JSON) -> Result<Self, ParseError> {
        let destination = try!(path.push("destination",
            |path| SetterSelector::take_vec(path, source, "destination"))
        );
        let kind = try!(path.push("kind",
            |path| ChannelKind::take(path, source, "kind"))
        );
        let value = try!(path.push("value",
            |path| Value::take(path, source, "value"))
        );
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

