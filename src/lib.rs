//! # testscript-rs
//!
//! A Rust crate for testing command-line tools using filesystem-based script files,
//! mirroring the functionality of Go's `rogpeppe/go-internal/testscript`.
//!
//! This crate provides a framework for writing integration tests for CLI tools
//! using `.txtar` format files that contain both test scripts and file contents.

pub mod error;
pub mod parser;
pub mod run;

pub use error::{Error, Result};
pub use parser::{Command, Script, TxtarFile};
pub use run::{CommandFn, RunParams, SetupFn, TestEnvironment};

// Re-export for advanced users who need direct access
pub use run::run_test;

// Internal function used by the Builder - not part of public API
fn run(params: &mut RunParams, test_data_glob: &str) -> Result<()> {
    use walkdir::WalkDir;

    // Simple glob pattern matching - for now just handle basic patterns like "testdata/*.txt"
    let (base_dir, pattern) = if let Some(slash_pos) = test_data_glob.rfind('/') {
        let base_dir = &test_data_glob[..slash_pos];
        let pattern = &test_data_glob[slash_pos + 1..];
        (base_dir, pattern)
    } else {
        (".", test_data_glob)
    };

    // Convert glob pattern to a simple matcher
    let pattern_regex = pattern.replace("*", ".*");
    let regex = regex::Regex::new(&format!("^{}$", pattern_regex))?;

    let mut test_files = Vec::new();

    // Walk the directory and collect matching files
    for entry in WalkDir::new(base_dir).min_depth(1).max_depth(1) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(file_name) = entry.file_name().to_str() {
                if regex.is_match(file_name) {
                    test_files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    // Sort test files for consistent execution order
    test_files.sort();

    if test_files.is_empty() {
        return Err(Error::Generic(format!(
            "No test files found matching pattern: {}",
            test_data_glob
        )));
    }

    // Run each test file
    for test_file in test_files {
        run::run_script(&test_file, params)
            .map_err(|e| Error::Generic(format!("Test '{}' failed: {}", test_file.display(), e)))?;
    }

    Ok(())
}

/// Builder for configuring and running testscript tests
///
/// This provides a fluent interface for setting up and executing test scripts.
///
/// # Examples
///
/// ```no_run
/// use testscript_rs::testscript;
///
/// // Simple usage
/// testscript::run("testdata").execute().unwrap();
///
/// // With customization and advanced conditions
/// testscript::run("testdata")
///     .setup(|env| {
///         // Compile your CLI tool
///         std::process::Command::new("cargo")
///             .args(["build", "--bin", "my-cli"])
///             .status()
///             .expect("Failed to build");
///         Ok(())
///     })
///     .command("my-cmd", |_env, _args| {
///         // Custom command implementation
///         Ok(())
///     })
///     .condition("net", check_network_available())
///     .condition("docker", command_exists("docker"))
///     .condition("env:CI", std::env::var("CI").is_ok())
///     .execute()
///     .unwrap();
///
/// // Basic usage - all common conditions detected automatically
/// testscript::run("testdata")
///     .execute()
///     .unwrap();
///
/// fn check_network_available() -> bool {
///     // Your network check implementation
///     true
/// }
///
/// fn command_exists(cmd: &str) -> bool {
///     // Your command existence check
///     false
/// }
/// ```
pub struct Builder {
    dir: String,
    params: RunParams,
}

impl Builder {
    /// Create a new builder for the given test directory
    fn new(dir: impl Into<String>) -> Self {
        Self {
            dir: dir.into(),
            params: RunParams::new(),
        }
    }

    /// Add a setup function that runs before each test script
    ///
    /// The setup function receives a reference to the test environment and can
    /// perform actions like compiling binaries or setting up test data.
    pub fn setup<F>(mut self, func: F) -> Self
    where
        F: Fn(&TestEnvironment) -> Result<()> + 'static,
    {
        self.params = self.params.setup(func);
        self
    }

    /// Add a custom command that can be used in test scripts
    ///
    /// # Arguments
    /// * `name` - The command name as it will appear in test scripts
    /// * `func` - The function to execute when the command is called
    pub fn command(mut self, name: &str, func: CommandFn) -> Self {
        self.params = self.params.command(name, func);
        self
    }

    /// Set a condition value for conditional command execution
    ///
    /// Conditions can be used in test scripts like `[mycondition] exec echo hello`
    pub fn condition(mut self, name: &str, value: bool) -> Self {
        self.params = self.params.condition(name, value);
        self
    }

    /// Enable or disable updating test scripts with actual output
    ///
    /// When enabled, instead of failing on output mismatches, the test files
    /// will be updated with the actual command output.
    pub fn update_scripts(mut self, update: bool) -> Self {
        self.params = self.params.update_scripts(update);
        self
    }

    /// Execute all test scripts in the configured directory
    ///
    /// This will discover all `.txt` files in the directory and run them as test scripts.
    /// Each test runs in isolation with its own temporary directory.
    ///
    /// # Returns
    /// `Ok(())` if all tests pass, or the first error encountered.
    pub fn execute(mut self) -> Result<()> {
        let pattern = format!("{}/*.txt", self.dir);
        run(&mut self.params, &pattern)
    }
}

/// Create a new testscript builder for the given directory
///
/// This is the main entry point for running testscript tests.
///
/// # Examples
///
/// ```no_run
/// use testscript_rs::testscript;
///
/// // Run all tests in testdata directory
/// testscript::run("testdata").execute().unwrap();
/// ```
pub mod testscript {
    use super::*;

    /// Create a new testscript builder for the given directory
    pub fn run(dir: impl Into<String>) -> Builder {
        Builder::new(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_integration_test() {
        // Test the parser directly with a simple script
        let script_content = r#"exec echo hello
stdout hello

-- file.txt --
content"#;

        let script = crate::parser::parse(script_content).unwrap();
        assert_eq!(script.commands.len(), 2);
        assert_eq!(script.files.len(), 1);
    }

    #[test]
    fn test_example() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let testdata_dir = temp_dir.path().join("testdata");
        fs::create_dir(&testdata_dir).unwrap();

        let test_content = r#"exec echo "API works!"
stdout "API works!"

-- test.txt --
content"#;

        fs::write(testdata_dir.join("api_test.txt"), test_content).unwrap();

        let result = testscript::run(testdata_dir.to_string_lossy()).execute();
        assert!(result.is_ok(), "API example failed: {:?}", result);
    }
}
