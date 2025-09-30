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

    let mut test_files = Vec::new();

    // If specific files are provided, use them directly
    if let Some(ref files) = params.files {
        // Parse the base directory from the glob pattern
        let base_dir = if let Some(slash_pos) = test_data_glob.rfind('/') {
            &test_data_glob[..slash_pos]
        } else {
            "."
        };

        for file in files {
            let file_path = if file.starts_with('/') {
                // Absolute path - use as-is
                std::path::PathBuf::from(file)
            } else if file.contains('/') {
                // Relative path - resolve relative to base directory
                std::path::PathBuf::from(base_dir).join(file)
            } else {
                // Just a filename - look in the base directory
                std::path::PathBuf::from(base_dir).join(file)
            };

            // Validate that the file exists
            if !file_path.exists() {
                return Err(Error::Generic(format!(
                    "Test file not found: {}",
                    file_path.display()
                )));
            }

            if !file_path.is_file() {
                return Err(Error::Generic(format!(
                    "Path is not a file: {}",
                    file_path.display()
                )));
            }

            test_files.push(file_path);
        }
    } else {
        // Use the original glob-based discovery
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
    }

    // Sort test files for consistent execution order
    test_files.sort();

    if test_files.is_empty() {
        if params.files.is_some() {
            return Err(Error::Generic("No test files specified".to_string()));
        } else {
            return Err(Error::Generic(format!(
                "No test files found matching pattern: {}",
                test_data_glob
            )));
        }
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
/// ## Automatic Condition Detection
///
/// The following conditions are automatically available without any setup:
///
/// - **Platform conditions**: `[unix]`, `[windows]`, `[linux]`, `[darwin]`, `[macos]`
/// - **Network condition**: `[net]` - Tests network connectivity by pinging reliable hosts
/// - **Build conditions**: `[debug]`, `[release]` - Based on compilation flags
/// - **Program conditions**: `[exec:program]` - Checks if a program is available in PATH
/// - **Environment conditions**: `[env:VAR]` - Dynamic checking of environment variables
/// - **Program existence**: `[exec:program]` - Checks if a program is available in PATH
/// - **Negation**: Use `!` to negate any condition, e.g. `[!windows]`, `[!env:CI]`, `[!exec:git]`
///
/// ## Examples
///
/// ### Basic Usage
/// ```no_run
/// use testscript_rs::testscript;
///
/// // Simple usage - all conditions detected automatically
/// testscript::run("testdata").execute().unwrap();
/// ```
///
/// ### Running Specific Test Files
/// ```no_run
/// use testscript_rs::testscript;
///
/// // Run only specific test files instead of all .txt files
/// testscript::run("testdata")
///     .files(["hello.txt", "exists.txt"])
///     .execute()
///     .unwrap();
/// ```
///
/// ### With Custom Setup and Work Directory Preservation
/// ```no_run
/// use testscript_rs::testscript;
///
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
///     .condition("custom", my_custom_check())
///     .preserve_work_on_failure(true)  // Preserve work directory on test failure
///     .execute()
///     .unwrap();
///
/// fn my_custom_check() -> bool {
///     // Your custom condition logic
///     true
/// }
/// ```
///
/// ### With Custom Work Directory Root
/// ```no_run
/// use testscript_rs::testscript;
///
/// // Use a custom location for test working directories
/// testscript::run("testdata")
///     .workdir_root("/tmp/my-app-tests")
///     .preserve_work_on_failure(true)  // Combine features
///     .execute()
///     .unwrap();
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
    /// Use this to add custom conditions beyond the built-in ones.
    /// Many common conditions are automatically detected (see Builder docs for details).
    ///
    /// # Arguments
    /// * `name` - The condition name (use in scripts as `[name]`)
    /// * `value` - Whether the condition is met
    ///
    /// # Built-in Conditions (automatically available)
    /// - `net` - Network connectivity
    /// - `unix`, `windows`, `linux`, `darwin` - Platform detection
    /// - `debug`, `release` - Build type  
    /// - `exec:program` - Program availability (35+ programs)
    /// - `env:VAR` - Environment variables (dynamic)
    ///
    /// # Examples
    /// ```no_run
    /// use testscript_rs::testscript;
    ///
    /// testscript::run("testdata")
    ///     .condition("feature_enabled", cfg!(feature = "advanced"))
    ///     .condition("has_gpu", check_gpu_available())
    ///     .execute()
    ///     .unwrap();
    ///
    /// fn check_gpu_available() -> bool {
    ///     // Your GPU detection logic
    ///     false
    /// }
    /// ```
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

    /// Enable or disable preserving working directories when tests fail
    ///
    /// When enabled, if a test fails, the working directory will be preserved
    /// and its path will be printed to stderr for debugging purposes.
    /// This matches the behavior of Go's testscript TestWork functionality.
    ///
    /// # Examples
    /// ```no_run
    /// use testscript_rs::testscript;
    ///
    /// testscript::run("testdata")
    ///     .preserve_work_on_failure(true)
    ///     .execute()
    ///     .unwrap();
    /// ```
    ///
    /// When a test fails, you'll see output like:
    /// ```text
    /// Test failed. Work directory preserved at: /tmp/testscript-work-abc123
    /// You can inspect the test environment:
    ///   cd /tmp/testscript-work-abc123
    ///   ls -la
    /// ```
    pub fn preserve_work_on_failure(mut self, preserve: bool) -> Self {
        self.params = self.params.preserve_work_on_failure(preserve);
        self
    }

    /// Set the root directory for test working directories
    ///
    /// When specified, test directories will be created inside this root directory
    /// instead of the system default temporary directory. This is useful for:
    /// - **Debugging**: Use a known location for test directories
    /// - **Performance**: Use faster storage (e.g., tmpfs on Linux)
    /// - **CI environments**: Use specific temp locations
    /// - **Security**: Isolate test directories to specific paths
    ///
    /// # Arguments
    /// * `root` - The directory path where test working directories should be created
    ///
    /// # Examples
    /// ```no_run
    /// use testscript_rs::testscript;
    ///
    /// // Use custom location for test directories
    /// testscript::run("testdata")
    ///     .workdir_root("/tmp/my-app-tests")
    ///     .preserve_work_on_failure(true)  // Combine with other features
    ///     .execute()
    ///     .unwrap();
    /// ```
    ///
    /// This creates test directories like `/tmp/my-app-tests/testscript-abc123/`.
    ///
    /// # Notes
    /// - The root directory must exist and be writable
    /// - If not specified, uses the system default temporary directory
    /// - Each test still gets its own isolated subdirectory
    pub fn workdir_root<P: Into<std::path::PathBuf>>(mut self, root: P) -> Self {
        self.params = self.params.workdir_root(root);
        self
    }

    /// Run only specific test files instead of discovering all .txt files
    ///
    /// When specified, only these files will be executed instead of discovering
    /// all .txt files in the directory. Files can be specified as:
    /// - Relative paths (relative to the test directory): `"hello.txt"`
    /// - Absolute paths: `"/path/to/test.txt"`
    /// - Just filenames: `"test.txt"` (looked up in the test directory)
    ///
    /// # Arguments
    /// * `files` - An iterator of file paths to run
    ///
    /// # Examples
    /// ```no_run
    /// use testscript_rs::testscript;
    ///
    /// // Run specific test files
    /// testscript::run("testdata")
    ///     .files(["hello.txt", "exists.txt"])
    ///     .execute()
    ///     .unwrap();
    ///
    /// // Using Vec<String>
    /// let test_files = vec!["hello.txt".to_string(), "exists.txt".to_string()];
    /// testscript::run("testdata")
    ///     .files(test_files)
    ///     .execute()
    ///     .unwrap();
    /// ```
    ///
    /// # Benefits
    /// - **Selective testing**: Run only specific test scenarios
    /// - **Faster development**: Skip unrelated tests during development
    /// - **CI optimization**: Run subsets of tests in different jobs
    /// - **Go compatibility**: Match Go testscript functionality
    pub fn files<I, S>(mut self, files: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params = self.params.files(files);
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
///
/// // Run only specific test files
/// testscript::run("testdata")
///     .files(["hello.txt", "exists.txt"])
///     .execute()
///     .unwrap();
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
