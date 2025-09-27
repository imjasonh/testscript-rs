//! Test execution module

pub mod commands;
pub mod environment;
pub mod execution;
pub mod params;

// Re-export public types
pub use environment::TestEnvironment;
pub use params::{CommandFn, RunParams, SetupFn};

use crate::error::Result;
use std::path::Path;

/// Run a single test script
pub fn run_test(script_path: &Path) -> Result<()> {
    let params = RunParams::new();
    run_script(script_path, &params)
}

/// Run a single script with the given parameters
pub fn run_script(script_path: &Path, params: &RunParams) -> Result<()> {
    execution::run_script_impl(script_path, params)
}
