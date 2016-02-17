/// An abstraction on top of the APIs that will need to be implemented
/// at lower-level.
pub mod dependencies;

/// Dealing with values provided by the devices.
pub mod values;

/// Definition of the AST.
pub mod ast;

/// Parsing JSON into an AST.
pub mod parse;

/// Compiling an AST into something runnable.
pub mod compile;

/// Actually executing code.
pub mod run;

/// Miscellaneous internal utilities.
mod util;

