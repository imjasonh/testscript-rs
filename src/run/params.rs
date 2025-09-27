//! Configuration parameters for test execution

use crate::error::Result;
use crate::run::environment::TestEnvironment;
use std::collections::HashMap;

/// Type alias for a custom command function
pub type CommandFn = fn(&mut TestEnvironment, &[String]) -> Result<()>;

/// Type alias for a setup function
pub type SetupFn = Box<dyn Fn(&TestEnvironment) -> Result<()>>;

/// Configuration parameters for running tests
pub struct RunParams {
    /// Custom commands provided by the user
    pub commands: HashMap<String, CommandFn>,
    /// Setup function to run before the script executes
    pub setup: Option<SetupFn>,
    /// Conditions that can be checked in scripts
    pub conditions: HashMap<String, bool>,
    /// Whether to update test scripts with actual output
    pub update_scripts: bool,
}

impl RunParams {
    /// Create a new RunParams with default settings
    pub fn new() -> Self {
        let mut conditions = HashMap::new();

        // Add default conditions based on the current platform
        conditions.insert("unix".to_string(), cfg!(unix));
        conditions.insert("windows".to_string(), cfg!(windows));
        conditions.insert("linux".to_string(), cfg!(target_os = "linux"));
        conditions.insert("darwin".to_string(), cfg!(target_os = "macos"));
        conditions.insert("macos".to_string(), cfg!(target_os = "macos"));
        conditions.insert("mac".to_string(), cfg!(target_os = "macos"));

        // Add Rust-relevant conditions
        conditions.insert("debug".to_string(), cfg!(debug_assertions));
        conditions.insert("release".to_string(), !cfg!(debug_assertions));

        // Check for common programs
        conditions.insert("exec:cat".to_string(), Self::program_exists("cat"));
        conditions.insert("exec:echo".to_string(), Self::program_exists("echo"));
        conditions.insert("exec:ls".to_string(), Self::program_exists("ls"));
        conditions.insert("exec:mkdir".to_string(), Self::program_exists("mkdir"));
        conditions.insert("exec:rm".to_string(), Self::program_exists("rm"));

        // Check UPDATE_SCRIPTS environment variable
        let update_scripts = std::env::var("UPDATE_SCRIPTS")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        RunParams {
            commands: HashMap::new(),
            setup: None,
            conditions,
            update_scripts,
        }
    }

    /// Add a custom command
    pub fn command(mut self, name: &str, func: CommandFn) -> Self {
        self.commands.insert(name.to_string(), func);
        self
    }

    /// Set a setup function to run before each script
    pub fn setup<F>(mut self, func: F) -> Self
    where
        F: Fn(&TestEnvironment) -> Result<()> + 'static,
    {
        self.setup = Some(Box::new(func));
        self
    }

    /// Set a condition value
    pub fn condition(mut self, name: &str, value: bool) -> Self {
        self.conditions.insert(name.to_string(), value);
        self
    }

    /// Set whether to update scripts with actual output
    pub fn update_scripts(mut self, update: bool) -> Self {
        self.update_scripts = update;
        self
    }

    /// Check if a program exists in PATH
    fn program_exists(program: &str) -> bool {
        std::process::Command::new("which")
            .arg(program)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

impl Default for RunParams {
    fn default() -> Self {
        Self::new()
    }
}
