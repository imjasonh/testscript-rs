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

        // Add network condition - check if network is available by default
        conditions.insert("net".to_string(), Self::check_network_available());

        // Check for common programs - comprehensive set like Go testscript
        let common_programs = [
            // Basic Unix/Windows commands
            "cat",
            "echo",
            "ls",
            "mkdir",
            "rm",
            "cp",
            "mv",
            "chmod",
            "pwd",
            "cd",
            "grep",
            "sed",
            "awk",
            "sort",
            "uniq",
            "head",
            "tail",
            "wc",
            "find",
            "tar",
            "gzip",
            "gunzip",
            "zip",
            "unzip",
            // Development tools
            "git",
            "make",
            "cmake",
            "docker",
            "node",
            "npm",
            "yarn",
            "python",
            "python3",
            "pip",
            "pip3",
            "go",
            "cargo",
            "rustc",
            "java",
            "javac",
            "mvn",
            "gradle",
            // Build/test tools
            "curl",
            "wget",
            "ssh",
            "rsync",
            "diff",
            "patch",
            // Platform-specific variations
            "sh",
            "bash",
            "zsh",
            "ps",
            "kill",
            "sleep",
            "true",
            "false",
            // Test programs (for testing purposes - these likely don't exist)
            "nonexistent_rare_program_xyz123",
        ];

        for program in &common_programs {
            let condition_name = format!("exec:{}", program);
            conditions.insert(condition_name, Self::program_exists(program));
        }

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

    /// Check if a program exists in PATH (cross-platform)
    fn program_exists(program: &str) -> bool {
        // Use different commands based on platform
        #[cfg(windows)]
        let check_cmd = "where";
        #[cfg(not(windows))]
        let check_cmd = "which";

        std::process::Command::new(check_cmd)
            .arg(program)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check if network is available by attempting to reach a reliable host
    fn check_network_available() -> bool {
        // Try multiple reliable hosts to increase reliability
        let test_hosts = ["1.1.1.1", "8.8.8.8", "9.9.9.9"];

        for host in &test_hosts {
            let output = std::process::Command::new("ping")
                .args(if cfg!(windows) {
                    vec!["-n", "1", "-w", "1000", host]
                } else {
                    vec!["-c", "1", "-W", "1", host]
                })
                .output();

            if let Ok(result) = output {
                if result.status.success() {
                    return true;
                }
            }
        }
        false
    }

    /// Check environment variable condition
    pub fn check_env_condition(condition: &str) -> bool {
        if let Some(env_var) = condition.strip_prefix("env:") {
            std::env::var(env_var).is_ok()
        } else {
            false
        }
    }
}

impl Default for RunParams {
    fn default() -> Self {
        Self::new()
    }
}
