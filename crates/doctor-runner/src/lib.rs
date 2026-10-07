//! Direct subprocess execution and tool-version detection. No target scripts
//! or contract builds are executed automatically.

mod environment;
mod execution;

pub use environment::{
    detect_environment, inspect_tool, parse_version, Environment, EnvironmentError, ToolDetection,
    ToolState,
};
pub use execution::{execute, CapturedCommand, CommandSpec, RunnerError};
