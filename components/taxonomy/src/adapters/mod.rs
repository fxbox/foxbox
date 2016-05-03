//! Defining and using adapters.
//!
//! The Foxbox can be extended by implementing new Adapters, which teach it to talk to new devices
//! or classes of devices. This module contains all the code required to implement adapters, as
//! well as the code that internally talks to adapters.

/// Defining new Adapters.
pub mod adapter;

/// The code that handles all Adapters behind the scenes.
mod backend;


/// A pure software adapter, designed for testing.
pub mod fake_adapter;

/// The Adapter manager. Used by Adapters to (un)register themselves and their services.
pub mod manager;

/// Persisting tags to disk.
mod tag_storage;

/// Utilities.
pub mod utils;
