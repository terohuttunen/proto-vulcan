//! Runtime execution for the Proto-Vulcan interpreter
//!
//! This module contains the runtime execution system for the IR-based interpreter.
//! It converts compiled IR to runtime goals for execution by the Proto-Vulcan engine.

pub mod context;

pub use crate::goal::Goal;
pub use context::{ExecutionContext, VariableValue};
