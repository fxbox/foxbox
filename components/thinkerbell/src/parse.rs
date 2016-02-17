use lang::{UncheckedCtx, UncheckedEnv, Script, Requirement, Resource, Trigger, Conjunction, Condition, Statement, Expression};
use values::{Number, Value, Range};

use std::collections::HashMap;
use std::marker::PhantomData;

extern crate serde_json;

pub type Json = self::serde_json::Value;

#[derive(Debug)]
pub enum StatementError {
    NotAnObject,
    InvalidDestination,
    InvalidAction,
    InvalidArgs,
}

#[derive(Debug)]
pub enum ExpressionError {
    InvalidStructure,
    InvalidNumber,
}

#[derive(Debug)]
pub enum ConditionError {
    NotAnObject,
    InvalidInput,
    InvalidCapability,
    InvalidNotIn,
    InvalidRange,
}

#[derive(Debug)]
pub enum ConjunctionError {
    NotAnArray,
}

#[derive(Debug)]
pub enum TriggerError {
    NotAnObject,
    NoCondition,
    NoAction,
}

#[derive(Debug)]
pub enum RequirementError {
    NotAnObject,
    NoKind,
    InvalidInput,
    InvalidOutput,
}


#[derive(Debug)]
pub enum ResourceError {
    NotAnArray,
    InvalidResource,
}

#[derive(Debug)]
pub enum ScriptError {
    NotAnObject,
    NoRequirements,
    NoAllocations,
    NoRules,
}

#[derive(Debug)]
pub enum Error {
    Expression(ExpressionError),
    Statement(StatementError),
    Condition(ConditionError),
    Conjunction(ConjunctionError),
    Trigger(TriggerError),
    Requirement(RequirementError),
    Resource(ResourceError),
    Script(ScriptError),
}

// FIXME: Reading from a json::Parser instead of a json::Json would let us attach a position in the source code.

pub struct Parser;
impl Parser {
    /// Parse a Json object into an unchecked script.
    pub fn parse(source: Json) -> Result<Script<UncheckedCtx, UncheckedEnv>, Error> {
        Self::parse_script(source)
    }

    pub fn parse_script(source: Json) -> Result<Script<UncheckedCtx, UncheckedEnv>, Error> {
        use self::serde_json::Value::*;
        if let Object(mut obj) = source {
            let requirements = if let Some(Array(requirements)) = obj.remove(&"requirements".to_owned()) {
                try!(Self::map(requirements, |req| {
                    Self::parse_requirement(req)
                }))
            } else {
                return Err(Error::Script(ScriptError::NoRequirements));
            };

            let allocations = if let Some(Array(allocations)) = obj.remove(&"allocations".to_owned()) {
                try!(Self::map(allocations, |alloc| {
                    Self::parse_resource(alloc)
                }))
            } else {
                return Err(Error::Script(ScriptError::NoAllocations));
            };

            let rules = if let Some(Array(rules)) = obj.remove(&"rules".to_owned()) {
                try!(Self::map(rules, |rule| {
                    Self::parse_trigger(rule)
                }))
            } else {
                return Err(Error::Script(ScriptError::NoRules));
            };

            Ok(Script {
                metadata: (),
                requirements: requirements,
                allocations: allocations,
                rules: rules
            })
        } else {
            Err(Error::Script(ScriptError::NotAnObject))
        }
    }

    pub fn parse_resource(source: Json) -> Result<Resource<UncheckedCtx, UncheckedEnv>, Error> {
        use self::serde_json::Value::*;
        if let Array(vec) = source {
            let devices = try!(Self::map(vec, |dev| {
                match dev {
                    String(name) => Ok(name),
                    _ => Err(Error::Resource(ResourceError::InvalidResource))
                }
            }));
            Ok(Resource {
                devices: devices,
                phantom: PhantomData,
            })
        } else {
            Err(Error::Resource(ResourceError::NotAnArray))
        }
    }

    pub fn parse_requirement(source: Json) -> Result<Requirement<UncheckedCtx, UncheckedEnv>, Error> {
        use self::serde_json::Value::*;
        if let Object(mut obj) = source {
            let kind = if let Some(String(kind)) = obj.remove(&"kind".to_owned()) {
                kind
            } else {
                return Err(Error::Requirement(RequirementError::NoKind))
            };
            let inputs = match obj.remove(&"inputs".to_owned()) {
                None => vec![],
                Some(Array(inputs)) =>
                    try!(Self::map(inputs, |input| {
                        match input {
                            String(x) => Ok(x),
                            _ => Err(Error::Requirement(RequirementError::InvalidInput))
                        }
                    })),
                _ => return Err(Error::Requirement(RequirementError::InvalidInput))
            };
            let outputs = match obj.remove(&"outputs".to_owned()) {
                None => vec![],
                Some(Array(outputs)) =>
                    try!(Self::map(outputs, |output| {
                        match output {
                            String(x) => Ok(x),
                            _ => Err(Error::Requirement(RequirementError::InvalidOutput))
                        }
                    })),
                _ => return Err(Error::Requirement(RequirementError::InvalidOutput))
            };
            Ok(Requirement {
                kind: kind,
                inputs: inputs,
                outputs: outputs,
                phantom: PhantomData
            })
        } else {
            Err(Error::Requirement(RequirementError::NotAnObject))
        }
    }

    pub fn parse_trigger(source: Json) -> Result<Trigger<UncheckedCtx, UncheckedEnv>, Error> {
        use self::serde_json::Value::*;
        if let Object(mut obj) = source {
            let condition = if let Some(condition) = obj.remove(&"condition".to_owned()) {
                try!(Self::parse_conjunction(condition))
            } else {
                return Err(Error::Trigger(TriggerError::NoCondition))
            };

            let execute = if let Some(Array(execute)) = obj.remove(&"action".to_owned()) {
                try!(Self::map(execute, |statement| {
                    Self::parse_statement(statement)
                }))
            } else {
                return Err(Error::Trigger(TriggerError::NoAction))
            };

            Ok(Trigger {
                condition: condition,
                execute: execute,
            })
        } else {
            Err(Error::Trigger(TriggerError::NotAnObject))
        }
    }


    pub fn parse_conjunction(source: Json) -> Result<Conjunction<UncheckedCtx, UncheckedEnv>, Error> {
        use self::serde_json::Value::*;
        if let Array(all) = source {
            let all = try!(Self::map(all, |condition| {
                Self::parse_condition(condition)
            }));
            Ok(Conjunction {
                all: all,
                state: ()
            })
        } else {
            Err(Error::Conjunction(ConjunctionError::NotAnArray))
        }
    }

    pub fn parse_condition(source: Json) -> Result<Condition<UncheckedCtx, UncheckedEnv>, Error> {
        use self::serde_json::Value::*;
        if let Object(mut obj) = source {
            let input = match obj.remove("input") {
                Some(U64(input)) => input as usize,
                _ => return Err(Error::Condition(ConditionError::InvalidInput))
            };
            let capability = match obj.remove("capability") {
                Some(String(capability)) => capability,
                _ => return Err(Error::Condition(ConditionError::InvalidCapability))
            };
            let range = match obj.remove("range") {
                None => Range::Any,
                Some(Bool(b)) => Range::EqBool(b),
                Some(String(s)) => Range::EqString(s),
                Some(Array(mut a)) =>
                // Unfortunately, no pattern-matching on arrays yet.
                    match a.len() {
                        2 => {
                            let max = a.pop().unwrap();
                            let min = a.pop().unwrap();
                            if min == Null {
                                Range::Leq(try!(Self::parse_number(max)))
                            } else if max == Null {
                                Range::Geq(try!(Self::parse_number(min)))
                            } else {
                                Range::BetweenEq {
                                    min: try!(Self::parse_number(min)),
                                    max: try!(Self::parse_number(max))
                                }
                            }
                        }
                        3 => {
                            let max = a.pop().unwrap();
                            let min = a.pop().unwrap();
                            let tag = a.pop().unwrap();
                            if let String(s) = tag {
                                if &*s == "notin" {
                                    Range::OutOfStrict {
                                        min: try!(Self::parse_number(min)),
                                        max: try!(Self::parse_number(max)),
                                    }
                                } else {
                                    return Err(Error::Condition(ConditionError::InvalidNotIn))
                                }
                            } else {
                                return Err(Error::Condition(ConditionError::InvalidNotIn))
                            }
                        }
                        _ => return Err(Error::Condition(ConditionError::InvalidRange))
                    },
                _ => return Err(Error::Condition(ConditionError::InvalidRange))
            };
            Ok(Condition {
                input: input,
                capability: capability,
                range: range,
                state: (),
            })
        } else {
            Err(Error::Condition(ConditionError::NotAnObject))
        }
    }


    pub fn parse_statement(source: Json) -> Result<Statement<UncheckedCtx, UncheckedEnv>, Error> {
        use self::serde_json::Value::*;
        if let Object(mut obj) = source {
            let destination = match obj.remove("output") {
                Some(U64(destination)) => destination as usize,
                _ => return Err(Error::Statement(StatementError::InvalidDestination))
            };
            let action = match obj.remove("capability") {
                Some(String(action)) => action,
                _ => return Err(Error::Statement(StatementError::InvalidAction))
            };
            let args = match obj.remove("args") {
                None => HashMap::new(),
                Some(Object(obj)) => {
                    let mut args = HashMap::new();
                    for (key, expr) in obj {
                        args.insert(key, try!(Self::parse_expression(expr)));
                    }
                    args
                }
                _ => {
                    return Err(Error::Statement(StatementError::InvalidArgs))
                }
            };
            Ok(Statement {
                destination: destination,
                action: action,
                arguments: args,
            })
        } else {
            Err(Error::Statement(StatementError::NotAnObject))
        }
    }


    pub fn parse_expression(source: Json) -> Result<Expression<UncheckedCtx, UncheckedEnv>, Error> {
        use self::serde_json::Value::*;
        let result = match source {
            String(s) => Expression::Value(Value::String(s)),
            Bool(b) => Expression::Value(Value::Bool(b)),
            Array(a) => {
                Expression::Vec(try!(Self::map(a, |expr| {
                    Self::parse_expression(expr)
                })))
            }
            Object(_) => {
                Expression::Value(Value::Num(try!(Self::parse_number(source))))
            }
            _ => return Err(Error::Expression(ExpressionError::InvalidStructure)),
        };
        Ok(result)
    }


    pub fn parse_number(source: Json) -> Result<Number, Error> {
        use self::serde_json::Value::*;
        if let Object(mut obj) = source {
            match (obj.remove("value"), obj.remove("unit")) {
                (Some(value), None) => {
                    let num = match value {
                        U64(num) => num as f64,
                        I64(num) => num as f64,
                        F64(num) => num,
                        _ => return Err(Error::Expression(ExpressionError::InvalidNumber))
                    };
                    Ok(Number::new(num, ()))
                }
                _ => Err(Error::Expression(ExpressionError::InvalidNumber))
            }
        } else {
            Err(Error::Expression(ExpressionError::InvalidNumber))
        }
    }

    // Utility function. A variant of `map` that stops in case of error.
    fn map<T, F, U>(vec: Vec<T>, cb: F) -> Result<Vec<U>, Error> where F: Fn(T) -> Result<U, Error> {
        let mut result = Vec::with_capacity(vec.len());
        for val in vec {
            result.push(try!(cb(val)));
        }
        Ok(result)
    }

}
