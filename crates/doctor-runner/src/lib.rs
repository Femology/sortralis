//! Direct subprocess execution and tool-version detection. No target scripts
//! or contract builds are executed automatically.

mod environment;
mod execution;

pub use environment::{
    detect_environment, detect_environment_with, inspect_tool, parse_version, Environment,
    EnvironmentError, ToolDetection, ToolState,
};
pub use execution::{execute, CapturedCommand, CommandSpec, RunnerError};

/// Injectable boundary for explicitly requested subprocesses.
pub trait CommandRunner {
    fn execute(&self, request: &CommandSpec) -> Result<CapturedCommand, RunnerError>;
}
pub struct SystemRunner;
impl CommandRunner for SystemRunner {
    fn execute(&self, request: &CommandSpec) -> Result<CapturedCommand, RunnerError> {
        execute(request)
    }
}
