//! Plugins for Proto-Vulcan Constraint Processing
//!
//! This module contains plugin implementations that provide both substitution
//! extension processing and constraint block compilation capabilities.

pub mod clpfd;
pub mod clpz;

pub use clpfd::ClpfdPlugin;
pub use clpz::ClpzPlugin;