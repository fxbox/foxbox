#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

//! This create defines mechanisms for executing simple scripts on the
//! server.
//!
//! By design, these scripts have limited capabilities. Each script
//! takes the form of a set of rules: "when any of the input services
//! of foo matches some condition, send some value to all of the onput
//! services of bar".
//!
//! See module `ast` for more details on the grammar of scripts.

extern crate foxbox_adapters;
extern crate foxbox_taxonomy;

extern crate transformable_channels;

extern crate chrono;
extern crate serde;
extern crate serde_json;


/// Definition of the AST.
pub mod ast;

/// Parsing JSON into an AST.
pub mod parse;

/// Compiling an AST into something runnable.
pub mod compile;

/// Actually executing code.
pub mod run;

/// Miscellaneous internal utilities.
pub mod util;

/// An implementation of Thinkerbell on top of simulated devices.
pub mod simulator;