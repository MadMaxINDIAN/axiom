// This module re-exports action execution utilities.
// Core logic lives in evaluator.rs; this module is the public interface
// for action-related helpers used by server/CLI layers.

pub use crate::evaluator::set_path;
