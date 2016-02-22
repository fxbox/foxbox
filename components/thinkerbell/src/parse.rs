//! Parsing scripts.
use ast::{Script, UncheckedCtx};

extern crate serde_json;

/// A structure dedicated to parsing scripts from JSON strings.
pub struct Parser;

impl Parser {
    /// Attempt to parse a string to an unchecked script.
    pub fn parse(str: String) -> Result<Script<UncheckedCtx>, serde_json::error::Error> {
        self::serde_json::from_str(&str)
    }
}
