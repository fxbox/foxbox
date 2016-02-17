use dependencies::{DevEnv, ExecutableDevEnv};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use ast::{Script, Requirement, Resource, Trigger, Statement, Conjunction, Condition, Expression, Context, UncheckedCtx, UncheckedEnv};
use values::Value;
use util::map;

extern crate chrono;
use self::chrono::{DateTime, UTC};

///
/// # Precompilation
///

/// Data, labelled with its latest update.
pub struct DatedData {
    pub updated: DateTime<UTC>,
    pub data: Value,
}


pub struct CompiledCtx<DevEnv> {
    phantom: PhantomData<DevEnv>,
}

pub struct CompiledInput<Env> where Env: DevEnv {
    pub device: Env::Device,
    pub state: RwLock<Option<DatedData>>,
}

pub struct CompiledOutput<Env> where Env: DevEnv {
    pub device: Env::Device,
}

pub type CompiledInputSet<Env> = Vec<Arc<CompiledInput<Env>>>;
pub type CompiledOutputSet<Env> = Vec<Arc<CompiledOutput<Env>>>;
pub struct CompiledConditionState {
    pub is_met: bool
}

impl<Env> Context for CompiledCtx<Env> where Env: DevEnv {
    type ConditionState = CompiledConditionState; // FIXME: We could share this
    type OutputSet = CompiledOutputSet<Env>;
    type InputSet = CompiledInputSet<Env>;
}


#[derive(Debug)]
pub enum SourceError {
    AllocationLengthError { allocations: usize, requirements: usize},
    NoCapability, // FIXME: Add details
    NoSuchInput, // FIXME: Add details
    NoSuchOutput, // FIXME: Add details
}

#[derive(Debug)]
pub enum DevAccessError {
    DeviceNotFound, // FIXME: Add details
    DeviceKindNotFound, // FIXME: Add details
    DeviceCapabilityNotFound, // FIXME: Add details
}

#[derive(Debug)]
pub enum Error {
    SourceError(SourceError),
    DevAccessError(DevAccessError),
}

pub struct Precompiler<Env> where Env: ExecutableDevEnv {
    inputs: Vec<Option<CompiledInputSet<Env>>>,
    outputs: Vec<Option<CompiledOutputSet<Env>>>,
    phantom: PhantomData<Env>,
}

impl<Env> Precompiler<Env> where Env: ExecutableDevEnv {
    pub fn new(source: &Script<UncheckedCtx, UncheckedEnv>) -> Result<Self, Error> {

        use self::Error::*;
        use self::SourceError::*;
        use self::DevAccessError::*;

        // In an UncheckedCtx, inputs and outputs are (unchecked)
        // indices towards the vector of allocations. In this step,
        // we 1/ check the indices, to make sure that they actually
        // point inside the vector;
        // 2/ prepare arrays `inputs` and `outputs`, which will later
        // serve to replace the indices by pointers to the Arc containing
        // details on the device and its state.

        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        if source.allocations.len() != source.requirements.len() {
            return Err(SourceError(AllocationLengthError {
                allocations: source.allocations.len(),
                requirements: source.requirements.len()
            }));
        }

        for (alloc, req) in source.allocations.iter().zip(&source.requirements) {
            let mut input = None;
            let mut output = None;

            let has_inputs = req.inputs.len() > 0;
            let has_outputs = req.outputs.len() > 0;
            if  !has_inputs && !has_outputs {
                // An empty resource? This doesn't make sense.
                return Err(SourceError(NoCapability));
            }

            if has_inputs {
                let mut resolved = Vec::with_capacity(alloc.devices.len());
                for dev in &alloc.devices {
                    match Env::get_device(&dev) {
                        None => return Err(DevAccessError(DeviceNotFound)),
                        Some(d) => resolved.push(Arc::new(CompiledInput {
                            device: d,
                            state: RwLock::new(None)
                        }))
                    }
                }
                input = Some(resolved);
            }
            if has_outputs {
                let mut resolved = Vec::with_capacity(alloc.devices.len());
                for dev in &alloc.devices {
                    match Env::get_device(&dev) {
                        None => return Err(DevAccessError(DeviceNotFound)),
                        Some(d) => resolved.push(Arc::new(CompiledOutput {
                            device: d,
                        }))
                    }
                }
                output = Some(resolved);
            }
            inputs.push(input);
            outputs.push(output);
        }

        Ok(Precompiler {
            inputs: inputs,
            outputs: outputs,
            phantom: PhantomData
        })
    }

    pub fn rebind_script(&self, script: Script<UncheckedCtx, UncheckedEnv>) -> Result<Script<CompiledCtx<Env>, Env>, Error>
    {
        let rules = try!(map(script.rules, |rule| self.rebind_trigger(rule)));

        let allocations = try!(map(script.allocations, |res| {
            let devices = try!(map(res.devices, |dev| {
                self.rebind_device(dev)
            }));
            Ok(Resource {
                devices: devices,
                phantom: PhantomData,
            })
        }));

        let requirements = try!(map(script.requirements, |req| {
            let inputs = try!(map(req.inputs, |input| {
                self.rebind_input_capability(input)
            }));
            let outputs = try!(map(req.outputs, |output| {
                self.rebind_output_capability(output)
            }));
            Ok(Requirement {
                kind: try!(self.rebind_device_kind(req.kind)),
                inputs: inputs,
                outputs: outputs,
                phantom: PhantomData
            })
        }));

        Ok(Script {
            metadata: (),
            requirements: requirements,
            allocations: allocations,
            rules: rules
        })
    }

    fn rebind_trigger(&self, trigger: Trigger<UncheckedCtx, UncheckedEnv>) -> Result<Trigger<CompiledCtx<Env>, Env>, Error>
    {
        let execute = try!(map(trigger.execute, |statement| {
            self.rebind_statement(statement)
        }));
        Ok(Trigger {
            execute: execute,
            condition: try!(self.rebind_conjunction(trigger.condition))
        })
    }

    fn rebind_conjunction(&self, conjunction: Conjunction<UncheckedCtx, UncheckedEnv>) -> Result<Conjunction<CompiledCtx<Env>, Env>, Error>
    {
        let all = try!(map(conjunction.all, |condition| {
            self.rebind_condition(condition)
        }));
        Ok(Conjunction {
            all: all,
            state: try!(self.rebind_condition_state(conjunction.state))
        })
    }

    fn rebind_condition(&self, condition: Condition<UncheckedCtx, UncheckedEnv>) -> Result<Condition<CompiledCtx<Env>, Env>, Error>
    {
        Ok(Condition {
            range: condition.range,
            capability: try!(self.rebind_input_capability(condition.capability)),
            input: try!(self.rebind_input(condition.input)),
            state: try!(self.rebind_condition_state(condition.state))
        })
    }

    fn rebind_statement(&self, statement: Statement<UncheckedCtx, UncheckedEnv>) -> Result<Statement<CompiledCtx<Env>, Env>, Error>
    {
        let mut arguments = HashMap::with_capacity(statement.arguments.len());
        for (key, expr) in statement.arguments {
            arguments.insert(key.clone(), try!(self.rebind_expression(expr)));
        }
        Ok(Statement {
            destination: try!(self.rebind_output(statement.destination)),
            action: try!(self.rebind_output_capability(statement.action)),
            arguments: arguments
        })
    }

    fn rebind_expression(&self, expression: Expression<UncheckedCtx, UncheckedEnv>) -> Result<Expression<CompiledCtx<Env>, Env>, Error>
    {
        let expression = match expression {
            Expression::Value(v) => Expression::Value(v),
            Expression::Vec(v) => {
                Expression::Vec(try!(map(v, |expr| {
                    self.rebind_expression(expr)
                })))
            }
            Expression::Input(_) => panic!("Not implemented yet")
        };
        Ok(expression)
    }

    fn rebind_device(&self, dev: <UncheckedEnv as DevEnv>::Device) -> Result<Env::Device, Error>
    {
        match Env::get_device(&dev) {
            None => Err(Error::DevAccessError(DevAccessError::DeviceNotFound)),
            Some(found) => Ok(found.clone())
        }
    }


    fn rebind_device_kind(&self, kind: <UncheckedEnv as DevEnv>::DeviceKind) ->
        Result<Env::DeviceKind, Error>
    {
        match Env::get_device_kind(&kind) {
            None => Err(Error::DevAccessError(DevAccessError::DeviceKindNotFound)),
            Some(found) => Ok(found.clone())
        }
    }
    
    fn rebind_input_capability(&self, cap: <UncheckedEnv as DevEnv>::InputCapability) ->
        Result<Env::InputCapability, Error>
    {
        match Env::get_input_capability(&cap) {
            None => Err(Error::DevAccessError(DevAccessError::DeviceCapabilityNotFound)),
            Some(found) => Ok(found.clone())
        }
    }

    fn rebind_output_capability(&self, cap: <UncheckedEnv as DevEnv>::OutputCapability) ->
        Result<Env::OutputCapability, Error>
    {
        match Env::get_output_capability(&cap) {
            None => Err(Error::DevAccessError(DevAccessError::DeviceCapabilityNotFound)),
            Some(found) => Ok(found.clone())
        }
    }

    // Rebinding the context
    fn rebind_condition_state(&self, (): <UncheckedCtx as Context>::ConditionState) ->
        Result<<CompiledCtx<Env> as Context>::ConditionState, Error>
    {
        // By default, conditions are not met.
        Ok(CompiledConditionState {
            is_met: false
        })
    }

    fn rebind_input(&self, index: <UncheckedCtx as Context>::InputSet) ->
        Result<<CompiledCtx<Env> as Context>::InputSet, Error>
    {
        match self.inputs[index] {
            None => Err(Error::SourceError(SourceError::NoSuchInput)),
            Some(ref input) => Ok(input.clone())
        }
    }


    fn rebind_output(&self, index: <UncheckedCtx as Context>::OutputSet) ->
        Result<<CompiledCtx<Env> as Context>::OutputSet, Error>
    {
        match self.outputs[index] {
            None => Err(Error::SourceError(SourceError::NoSuchOutput)),
            Some(ref output) => Ok(output.clone())
        }
    }
}
