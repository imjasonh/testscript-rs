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
    /// Whether to preserve working directories when tests fail
    pub preserve_work_on_failure: bool,
    /// Optional root directory for test working directories
    pub workdir_root: Option<std::path::PathBuf>,
    /// Specific files to run (if None, discover all .txt files)
    pub files: Option<Vec<String>>,
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

        // Check UPDATE_SCRIPTS environment variable
        let update_scripts = std::env::var("UPDATE_SCRIPTS")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        RunParams {
            commands: HashMap::new(),
            setup: None,
            conditions,
            update_scripts,
            preserve_work_on_failure: false,
            workdir_root: None,
            files: None,
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

    /// Set whether to preserve working directories when tests fail
    pub fn preserve_work_on_failure(mut self, preserve: bool) -> Self {
        self.preserve_work_on_failure = preserve;
        self
    }

    /// Set the root directory for test working directories
    ///
    /// When specified, test directories will be created inside this root directory
    /// instead of the system default temporary directory.
    pub fn workdir_root<P: Into<std::path::PathBuf>>(mut self, root: P) -> Self {
        self.workdir_root = Some(root.into());
        self
    }

    /// Set specific files to run instead of discovering all .txt files
    ///
    /// When specified, only these files will be executed instead of discovering
    /// all .txt files in the directory.
    pub fn files<I, S>(mut self, files: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.files = Some(files.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Check if a program exists in PATH (cross-platform)
    pub fn program_exists(program: &str) -> bool {
        // TODO: Consider caching results for performance if needed

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
        // In CI environments, network checks can be flaky or restricted
        // Use a shorter timeout and more defensive approach

        // Try a quick TCP connection first (faster than ping in many environments)
        if Self::check_network_tcp() {
            return true;
        }

        // Fallback to ping with shorter timeout
        Self::check_network_ping()
    }

    /// Check network via TCP connection (faster and more reliable in CI)
    fn check_network_tcp() -> bool {
        use std::net::{TcpStream, ToSocketAddrs};
        use std::time::Duration;

        // Try to connect to DNS servers on port 53 (usually allowed in CI)
        let addresses = ["1.1.1.1:53", "8.8.8.8:53"];

        for addr in &addresses {
            if let Ok(mut socket_addrs) = addr.to_socket_addrs() {
                if let Some(socket_addr) = socket_addrs.next() {
                    // Use a very short timeout for CI compatibility
                    if TcpStream::connect_timeout(&socket_addr, Duration::from_millis(500)).is_ok()
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Fallback network check using ping
    fn check_network_ping() -> bool {
        let test_hosts = ["1.1.1.1"]; // Just try one host to be faster

        for host in &test_hosts {
            let result = std::process::Command::new("ping")
                .args(if cfg!(windows) {
                    vec!["-n", "1", "-w", "500", host] // Shorter timeout
                } else {
                    vec!["-c", "1", "-W", "1", host]
                })
                .output();

            if let Ok(output) = result {
                if output.status.success() {
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
