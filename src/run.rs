//! Test execution environment and runner

use crate::error::{Error, Result};
use crate::parser::{Command, TxtarFile};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as StdCommand, Output, Stdio};
use tempfile::TempDir;

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

        RunParams {
            commands: HashMap::new(),
            setup: None,
            conditions,
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

/// Test execution environment for a single script run
pub struct TestEnvironment {
    /// The root temporary directory for the test run
    pub work_dir: PathBuf,
    /// The underlying TempDir that cleans up on drop
    _temp_dir: TempDir,
    /// Environment variables for this specific run
    pub env_vars: HashMap<String, String>,
    /// Current working directory relative to work_dir
    pub current_dir: PathBuf,
    /// Output from the last executed command
    pub last_output: Option<Output>,
    /// Background processes indexed by name
    pub background_processes: HashMap<String, Child>,
    /// Standard input content for the next exec command
    pub next_stdin: Option<Vec<u8>>,
    /// Whether the test should be skipped
    pub should_skip: bool,
    /// Whether the test should stop early (but pass)
    pub should_stop: bool,
}

impl TestEnvironment {
    /// Create a new test environment with a temporary directory
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let work_dir = temp_dir.path().to_path_buf();

        Ok(TestEnvironment {
            work_dir: work_dir.clone(),
            _temp_dir: temp_dir,
            env_vars: HashMap::new(),
            current_dir: work_dir,
            last_output: None,
            background_processes: HashMap::new(),
            next_stdin: None,
            should_skip: false,
            should_stop: false,
        })
    }

    /// Set up files from the parsed script in the work directory
    pub fn setup_files(&mut self, files: &[TxtarFile]) -> Result<()> {
        for file in files {
            let file_path = self.work_dir.join(&file.name);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write the file contents
            fs::write(&file_path, &file.contents)?;
        }
        Ok(())
    }

    /// Execute a command in the current test environment
    pub fn execute_command(&mut self, cmd: &str, args: &[String]) -> Result<Output> {
        let mut command = StdCommand::new(cmd);
        command
            .args(args)
            .current_dir(&self.current_dir)
            .envs(&self.env_vars);

        let output = if let Some(stdin_content) = self.next_stdin.take() {
            command.stdin(Stdio::piped());
            let mut child = command.spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin.write_all(&stdin_content)?;
            }
            child.wait_with_output()?
        } else {
            command.output()?
        };

        self.last_output = Some(output.clone());
        Ok(output)
    }

    /// Execute a command in the background
    pub fn execute_background_command(
        &mut self,
        name: &str,
        cmd: &str,
        args: &[String],
    ) -> Result<()> {
        let mut command = StdCommand::new(cmd);
        command
            .args(args)
            .current_dir(&self.current_dir)
            .envs(&self.env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = command.spawn()?;
        self.background_processes.insert(name.to_string(), child);
        Ok(())
    }

    /// Wait for a background process to complete
    pub fn wait_for_background(&mut self, name: &str) -> Result<Output> {
        if let Some(child) = self.background_processes.remove(name) {
            let output = child.wait_with_output()?;
            self.last_output = Some(output.clone());
            Ok(output)
        } else {
            Err(Error::command_error(
                "wait",
                format!("No background process named '{}'", name),
            ))
        }
    }

    /// Change the current directory (relative to work_dir)
    pub fn change_directory(&mut self, path: &str) -> Result<()> {
        let new_dir = if path.starts_with('/') || path.contains(':') {
            // Absolute path - not allowed in tests for security
            return Err(Error::command_error("cd", "Absolute paths not allowed"));
        } else {
            self.work_dir.join(path)
        };

        if !new_dir.exists() {
            return Err(Error::command_error(
                "cd",
                format!("Directory '{}' does not exist", path),
            ));
        }

        if !new_dir.is_dir() {
            return Err(Error::command_error(
                "cd",
                format!("'{}' is not a directory", path),
            ));
        }

        self.current_dir = new_dir;
        Ok(())
    }

    /// Compare two files for equality
    pub fn compare_files(&self, file1: &str, file2: &str) -> Result<()> {
        let path1 = self.work_dir.join(file1);
        let path2 = self.work_dir.join(file2);

        let contents1 = fs::read(&path1)
            .map_err(|e| Error::command_error("cmp", format!("Cannot read '{}': {}", file1, e)))?;
        let contents2 = fs::read(&path2)
            .map_err(|e| Error::command_error("cmp", format!("Cannot read '{}': {}", file2, e)))?;

        if contents1 != contents2 {
            let content1_str = String::from_utf8_lossy(&contents1);
            let content2_str = String::from_utf8_lossy(&contents2);
            return Err(Error::FileCompare {
                message: format!(
                    "Files differ:\n'{}' contains:\n{}\n\n'{}' contains:\n{}",
                    file1, content1_str, file2, content2_str
                ),
            });
        }

        Ok(())
    }

    /// Compare command output with expected content
    pub fn compare_output(&self, output_type: &str, expected: &str) -> Result<()> {
        let actual = match self.last_output {
            Some(ref output) => match output_type {
                "stdout" => String::from_utf8_lossy(&output.stdout)
                    .trim_end()
                    .to_string(),
                "stderr" => String::from_utf8_lossy(&output.stderr)
                    .trim_end()
                    .to_string(),
                _ => return Err(Error::command_error(output_type, "Unknown output type")),
            },
            None => {
                return Err(Error::command_error(
                    output_type,
                    "No command output available",
                ))
            }
        };

        // Substitute environment variables in the expected pattern
        let expected_substituted = self.substitute_env_vars(expected);

        // Check if expected is a regex pattern (contains regex special characters)
        if expected_substituted.contains('^')
            || expected_substituted.contains('$')
            || expected_substituted.contains('[')
            || expected_substituted.contains('(')
            || expected_substituted.contains('*')
            || expected_substituted.contains('.')
        {
            let regex_pattern = format!("(?s){}", expected_substituted); // (?s) enables DOTALL mode
            let regex = Regex::new(&regex_pattern)
                .map_err(|e| Error::command_error(output_type, format!("Invalid regex: {}", e)))?;

            if !regex.is_match(&actual) {
                return Err(Error::OutputCompare {
                    expected: expected_substituted,
                    actual,
                });
            }
        } else {
            // Exact string match
            if actual != expected_substituted {
                return Err(Error::OutputCompare {
                    expected: expected_substituted,
                    actual,
                });
            }
        }

        Ok(())
    }

    /// Check if a file or directory exists
    pub fn file_exists(&self, path: &str) -> bool {
        let full_path = if path == "stdout" || path == "stderr" {
            // Special case: check if last command had output
            return self.last_output.is_some();
        } else {
            self.work_dir.join(path)
        };
        full_path.exists()
    }

    /// Create directories
    pub fn create_directories(&self, paths: &[String]) -> Result<()> {
        for path in paths {
            let full_path = self.work_dir.join(path);
            fs::create_dir_all(&full_path)?;
        }
        Ok(())
    }

    /// Copy files or directories
    pub fn copy_files(&mut self, args: &[String]) -> Result<()> {
        if args.len() < 2 {
            return Err(Error::command_error("cp", "At least 2 arguments required"));
        }

        let dest = &args[args.len() - 1];
        let sources = &args[..args.len() - 1];
        let dest_path = self.work_dir.join(dest);

        for source in sources {
            let source_content = if source == "stdout" {
                if let Some(ref output) = self.last_output {
                    output.stdout.clone()
                } else {
                    return Err(Error::command_error("cp", "No stdout available"));
                }
            } else if source == "stderr" {
                if let Some(ref output) = self.last_output {
                    output.stderr.clone()
                } else {
                    return Err(Error::command_error("cp", "No stderr available"));
                }
            } else {
                let source_path = self.work_dir.join(source);
                fs::read(&source_path).map_err(|e| {
                    Error::command_error("cp", format!("Cannot read '{}': {}", source, e))
                })?
            };

            // If dest exists and is a directory, copy into it
            let final_dest = if dest_path.is_dir() {
                let filename = if source == "stdout" || source == "stderr" {
                    source
                } else {
                    Path::new(source)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(source)
                };
                dest_path.join(filename)
            } else {
                dest_path.clone()
            };

            fs::write(&final_dest, source_content)?;
        }
        Ok(())
    }

    /// Remove files or directories
    pub fn remove_files(&self, paths: &[String]) -> Result<()> {
        for path in paths {
            let full_path = self.work_dir.join(path);
            if full_path.is_dir() {
                fs::remove_dir_all(&full_path)?;
            } else if full_path.exists() {
                fs::remove_file(&full_path)?;
            }
        }
        Ok(())
    }

    /// Move/rename files or directories
    pub fn move_file(&self, source: &str, dest: &str) -> Result<()> {
        let source_path = self.work_dir.join(source);
        let dest_path = self.work_dir.join(dest);

        if !source_path.exists() {
            return Err(Error::command_error(
                "mv",
                format!("Source '{}' does not exist", source),
            ));
        }

        fs::rename(&source_path, &dest_path).map_err(|e| {
            Error::command_error(
                "mv",
                format!("Cannot move '{}' to '{}': {}", source, dest, e),
            )
        })?;

        Ok(())
    }

    /// Set environment variable
    pub fn set_env_var(&mut self, key: &str, value: &str) {
        self.env_vars.insert(key.to_string(), value.to_string());
    }

    /// Set stdin content for next exec command
    pub fn set_stdin_content(&mut self, content: Vec<u8>) {
        self.next_stdin = Some(content);
    }

    /// Set stdin from file content
    pub fn set_stdin_from_file(&mut self, filename: &str) -> Result<()> {
        let content = if filename == "stdout" {
            if let Some(ref output) = self.last_output {
                output.stdout.clone()
            } else {
                return Err(Error::command_error("stdin", "No stdout available"));
            }
        } else if filename == "stderr" {
            if let Some(ref output) = self.last_output {
                output.stderr.clone()
            } else {
                return Err(Error::command_error("stdin", "No stderr available"));
            }
        } else {
            let file_path = self.work_dir.join(filename);
            fs::read(&file_path).map_err(|e| {
                Error::command_error("stdin", format!("Cannot read '{}': {}", filename, e))
            })?
        };

        self.next_stdin = Some(content);
        Ok(())
    }

    /// Kill a background process
    pub fn kill_background_process(&mut self, name: &str, _signal: Option<&str>) -> Result<()> {
        if let Some(mut child) = self.background_processes.remove(name) {
            // On Unix, we could send specific signals, but for cross-platform compatibility,
            // we'll just kill the process
            child.kill()?;
            let _ = child.wait()?; // Reap the process
            Ok(())
        } else {
            Err(Error::command_error(
                "kill",
                format!("No background process named '{}'", name),
            ))
        }
    }

    /// Compare files with environment variable substitution in the second file
    pub fn compare_files_with_env(&self, file1: &str, file2: &str) -> Result<()> {
        let path1 = self.work_dir.join(file1);
        let path2 = self.work_dir.join(file2);

        let contents1 = fs::read_to_string(&path1).map_err(|e| {
            Error::command_error("cmpenv", format!("Cannot read '{}': {}", file1, e))
        })?;

        let contents2_raw = fs::read_to_string(&path2).map_err(|e| {
            Error::command_error("cmpenv", format!("Cannot read '{}': {}", file2, e))
        })?;

        // Substitute environment variables in file2
        let contents2 = self.substitute_env_vars(&contents2_raw);

        if contents1.trim() != contents2.trim() {
            return Err(Error::FileCompare {
                message: format!(
                    "Files differ after environment substitution:\n'{}' contains:\n{}\n\n'{}' (after substitution) contains:\n{}",
                    file1, contents1, file2, contents2
                ),
            });
        }

        Ok(())
    }

    /// Substitute environment variables in a string
    fn substitute_env_vars(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Handle $VAR and ${VAR} patterns
        for (key, value) in &self.env_vars {
            let patterns = [
                format!("${{{}}}", key), // ${VAR}
                format!("${}", key),     // $VAR (but be careful with word boundaries)
            ];

            for pattern in &patterns {
                result = result.replace(pattern, value);
            }
        }

        // Handle special case: $$ -> $ (escape)
        result = result.replace("$$", "$");

        result
    }

    /// Change file permissions (chmod)
    pub fn change_permissions(&self, mode: &str, file: &str) -> Result<()> {
        let file_path = self.work_dir.join(file);

        // Parse the octal mode (e.g., "444", "755")
        let mode_int = u32::from_str_radix(mode, 8)
            .map_err(|_| Error::command_error("chmod", format!("Invalid mode: {}", mode)))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&file_path)
                .map_err(|e| {
                    Error::command_error(
                        "chmod",
                        format!("Cannot get metadata for '{}': {}", file, e),
                    )
                })?
                .permissions();
            perms.set_mode(mode_int);
            fs::set_permissions(&file_path, perms).map_err(|e| {
                Error::command_error(
                    "chmod",
                    format!("Cannot set permissions for '{}': {}", file, e),
                )
            })?;
        }

        #[cfg(windows)]
        {
            // On Windows, we can only set read-only status
            let mut perms = fs::metadata(&file_path)
                .map_err(|e| {
                    Error::command_error(
                        "chmod",
                        format!("Cannot get metadata for '{}': {}", file, e),
                    )
                })?
                .permissions();

            // If mode doesn't include write permissions for owner (e.g., 444), make it read-only
            let readonly = (mode_int & 0o200) == 0;
            perms.set_readonly(readonly);

            fs::set_permissions(&file_path, perms).map_err(|e| {
                Error::command_error(
                    "chmod",
                    format!("Cannot set permissions for '{}': {}", file, e),
                )
            })?;
        }

        Ok(())
    }

    /// Unquote a file by removing leading ">" characters from each line
    pub fn unquote_file(&self, file: &str) -> Result<()> {
        let file_path = self.work_dir.join(file);

        let contents = fs::read_to_string(&file_path).map_err(|e| {
            Error::command_error("unquote", format!("Cannot read '{}': {}", file, e))
        })?;

        let unquoted_contents: String = contents
            .lines()
            .map(|line| line.strip_prefix('>').unwrap_or(line))
            .collect::<Vec<&str>>()
            .join("\n");

        fs::write(&file_path, unquoted_contents).map_err(|e| {
            Error::command_error("unquote", format!("Cannot write '{}': {}", file, e))
        })?;

        Ok(())
    }

    /// Search for a pattern in files (like grep)
    pub fn grep_files(&mut self, pattern: &str, files: &[String]) -> Result<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| Error::command_error("grep", format!("Invalid regex: {}", e)))?;

        let mut _found_matches = false;
        let mut output = String::new();

        for file in files {
            let file_path = self.work_dir.join(file);
            let contents = fs::read_to_string(&file_path).map_err(|e| {
                Error::command_error("grep", format!("Cannot read '{}': {}", file, e))
            })?;

            for (line_num, line) in contents.lines().enumerate() {
                if regex.is_match(line) {
                    output.push_str(&format!("{}:{}: {}\n", file, line_num + 1, line));
                    _found_matches = true;
                }
            }
        }

        // Set the output as if it came from an exec command
        // Create a fake successful command output
        let fake_output = std::process::Command::new("echo")
            .arg("")
            .output()
            .unwrap_or_else(|_| std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });

        self.last_output = Some(std::process::Output {
            status: fake_output.status,
            stdout: output.trim_end().as_bytes().to_vec(),
            stderr: Vec::new(),
        });

        Ok(())
    }

    /// Check if a file is read-only
    pub fn is_readonly(&self, file: &str) -> bool {
        let file_path = self.work_dir.join(file);

        if let Ok(metadata) = fs::metadata(&file_path) {
            metadata.permissions().readonly()
        } else {
            false
        }
    }

    /// Create a symbolic link
    pub fn create_symlink(&self, target: &str, link_name: &str) -> Result<()> {
        let target_path = self.work_dir.join(target);
        let link_path = self.work_dir.join(link_name);

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_path, &link_path).map_err(|e| {
                Error::command_error(
                    "symlink",
                    format!(
                        "Cannot create symlink '{}' -> '{}': {}",
                        link_name, target, e
                    ),
                )
            })?;
        }

        #[cfg(windows)]
        {
            // On Windows, try to create a symlink but fall back gracefully
            use std::os::windows::fs::symlink_file;

            if target_path.is_file() {
                symlink_file(&target_path, &link_path).map_err(|e| {
                    Error::command_error(
                        "symlink",
                        format!("Cannot create symlink '{}' -> '{}': {} (Note: Windows symlinks may require administrator privileges)", link_name, target, e),
                    )
                })?;
            } else {
                return Err(Error::command_error(
                    "symlink",
                    "Symlinks on Windows are only supported for files, not directories",
                ));
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            return Err(Error::command_error(
                "symlink",
                "Symlinks are not supported on this platform",
            ));
        }

        Ok(())
    }
}

/// Run a single test script
pub fn run_test(script_path: &Path) -> Result<()> {
    let params = RunParams::new();
    run_script(script_path, &params)
}

/// Run a single script with the given parameters
pub fn run_script(script_path: &Path, params: &RunParams) -> Result<()> {
    // Read and parse the script
    let content = fs::read_to_string(script_path)?;
    let script = crate::parser::parse(&content).map_err(|e| {
        // Enhance parse errors with script context
        match e {
            Error::Parse { line, message } => Error::script_error(
                script_path.to_string_lossy().to_string(),
                line,
                &content,
                Error::Parse { line, message },
            ),
            other => other,
        }
    })?;

    // Create script context for better error reporting
    let script_file = script_path.to_string_lossy().to_string();

    // Create test environment
    let mut env = TestEnvironment::new()?;

    // Set up files from the script
    env.setup_files(&script.files)?;

    // Set the $WORK environment variable to the working directory
    let work_dir_str = env.work_dir.to_string_lossy().to_string();
    env.set_env_var("WORK", &work_dir_str);

    // Run setup hook if provided
    if let Some(setup) = &params.setup {
        setup(&env)?;
    }

    // Execute commands
    for command in &script.commands {
        if let Err(e) = execute_command(&mut env, command, params) {
            // Wrap error with script context
            return Err(Error::script_error(
                &script_file,
                command.line_num,
                &content,
                e,
            ));
        }

        // Check for early termination
        if env.should_skip {
            return Err(Error::Generic("Test skipped".to_string()));
        }
        if env.should_stop {
            break; // Stop early but don't fail
        }
    }

    // Wait for any remaining background processes
    let background_names: Vec<String> = env.background_processes.keys().cloned().collect();
    for name in background_names {
        env.wait_for_background(&name)?;
    }

    Ok(())
}

/// Execute a single command
fn execute_command(env: &mut TestEnvironment, command: &Command, params: &RunParams) -> Result<()> {
    // For negated commands, we expect them to fail
    let result = execute_command_inner(env, command, params);

    if command.negated {
        match result {
            Ok(_) => Err(Error::command_error(
                &command.name,
                "Command was expected to fail but succeeded",
            )),
            Err(_) => Ok(()), // Negated command failed as expected
        }
    } else {
        result
    }
}

/// Inner command execution logic
fn execute_command_inner(
    env: &mut TestEnvironment,
    command: &Command,
    params: &RunParams,
) -> Result<()> {
    // Check condition if present
    if let Some(ref condition) = command.condition {
        let condition_met = if let Some(value) = params.conditions.get(condition) {
            *value
        } else if let Some(base_condition) = condition.strip_prefix('!') {
            // Handle negated conditions
            if let Some(value) = params.conditions.get(base_condition) {
                !value
            } else {
                return Err(Error::UnknownCondition {
                    condition: base_condition.to_string(),
                });
            }
        } else {
            return Err(Error::UnknownCondition {
                condition: condition.clone(),
            });
        };

        if !condition_met {
            return Ok(()); // Skip this command
        }
    }

    // Check for custom commands first
    if let Some(custom_fn) = params.commands.get(&command.name) {
        return custom_fn(env, &command.args);
    }

    // Handle built-in commands
    match command.name.as_str() {
        "exec" => {
            if command.args.is_empty() {
                return Err(Error::command_error("exec", "No command specified"));
            }

            let cmd = &command.args[0];
            let args = &command.args[1..];

            if command.background {
                // Use the command name as the background process name
                let process_name = cmd.to_string();
                env.execute_background_command(&process_name, cmd, args)?;
            } else {
                let output = env.execute_command(cmd, args)?;

                // Check exit status
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(Error::command_error(
                        "exec",
                        format!(
                            "Command '{}' failed with exit code {}: {}",
                            cmd,
                            output.status.code().unwrap_or(-1),
                            stderr.trim()
                        ),
                    ));
                }
            }
        }
        "cmp" => {
            if command.args.len() != 2 {
                return Err(Error::command_error("cmp", "Expected exactly 2 arguments"));
            }
            env.compare_files(&command.args[0], &command.args[1])?;
        }
        "cmpenv" => {
            if command.args.len() != 2 {
                return Err(Error::command_error(
                    "cmpenv",
                    "Expected exactly 2 arguments",
                ));
            }
            env.compare_files_with_env(&command.args[0], &command.args[1])?;
        }
        "stdout" | "stderr" => {
            if command.args.len() != 1 {
                return Err(Error::command_error(
                    &command.name,
                    "Expected exactly 1 argument",
                ));
            }

            let expected = &command.args[0];

            // Check if argument is a filename or literal text
            let expected_content = if expected == "-" {
                // Empty string
                "".to_string()
            } else if let Ok(file_content) = fs::read_to_string(env.work_dir.join(expected)) {
                // It's a file - use its contents
                file_content.trim_end().to_string()
            } else {
                // It's literal text
                expected.clone()
            };

            env.compare_output(&command.name, &expected_content)?;
        }
        "cd" => {
            if command.args.len() != 1 {
                return Err(Error::command_error("cd", "Expected exactly 1 argument"));
            }
            env.change_directory(&command.args[0])?;
        }
        "wait" => {
            if command.args.len() != 1 {
                return Err(Error::command_error("wait", "Expected exactly 1 argument"));
            }
            env.wait_for_background(&command.args[0])?;
        }
        "exists" => {
            if command.args.is_empty() {
                return Err(Error::command_error(
                    "exists",
                    "Expected at least 1 argument",
                ));
            }

            // Check for -readonly flag
            let (check_readonly, files) = if command.args[0] == "-readonly" {
                if command.args.len() < 2 {
                    return Err(Error::command_error(
                        "exists",
                        "Expected file argument after -readonly",
                    ));
                }
                (true, &command.args[1..])
            } else {
                (false, &command.args[..])
            };

            for path in files {
                if !env.file_exists(path) {
                    return Err(Error::command_error(
                        "exists",
                        format!("File '{}' does not exist", path),
                    ));
                }

                if check_readonly && !env.is_readonly(path) {
                    return Err(Error::command_error(
                        "exists",
                        format!("File '{}' is not read-only", path),
                    ));
                }
            }
        }
        "mkdir" => {
            if command.args.is_empty() {
                return Err(Error::command_error(
                    "mkdir",
                    "Expected at least 1 argument",
                ));
            }
            env.create_directories(&command.args)?;
        }
        "cp" => {
            if command.args.len() < 2 {
                return Err(Error::command_error("cp", "Expected at least 2 arguments"));
            }
            env.copy_files(&command.args)?;
        }
        "rm" => {
            if command.args.is_empty() {
                return Err(Error::command_error("rm", "Expected at least 1 argument"));
            }
            env.remove_files(&command.args)?;
        }
        "mv" => {
            if command.args.len() != 2 {
                return Err(Error::command_error("mv", "Expected exactly 2 arguments"));
            }
            env.move_file(&command.args[0], &command.args[1])?;
        }
        "env" => {
            if command.args.is_empty() {
                // Print current environment for debugging
                for (key, value) in &env.env_vars {
                    println!("{}={}", key, value);
                }
            } else {
                // Set environment variables
                for arg in &command.args {
                    if let Some(eq_pos) = arg.find('=') {
                        let key = &arg[..eq_pos];
                        let value = &arg[eq_pos + 1..];
                        env.set_env_var(key, value);
                    } else {
                        return Err(Error::command_error(
                            "env",
                            format!("Invalid env format: {}", arg),
                        ));
                    }
                }
            }
        }
        "stdin" => {
            if command.args.len() != 1 {
                return Err(Error::command_error("stdin", "Expected exactly 1 argument"));
            }
            env.set_stdin_from_file(&command.args[0])?;
        }
        "skip" => {
            env.should_skip = true;
            let message = if command.args.is_empty() {
                "Test skipped".to_string()
            } else {
                command.args.join(" ")
            };
            return Err(Error::Generic(format!("SKIP: {}", message)));
        }
        "stop" => {
            env.should_stop = true;
            let _message = if command.args.is_empty() {
                "Test stopped early".to_string()
            } else {
                command.args.join(" ")
            };
            // For stop, we don't return an error - the test passes but stops
            return Ok(());
        }
        "kill" => {
            let (signal, target) = if command.args.len() == 2 && command.args[0].starts_with('-') {
                (Some(command.args[0].as_str()), &command.args[1])
            } else if command.args.len() == 1 {
                (None, &command.args[0])
            } else {
                return Err(Error::command_error("kill", "Expected 1 or 2 arguments"));
            };
            env.kill_background_process(target, signal)?;
        }
        "chmod" => {
            if command.args.len() != 2 {
                return Err(Error::command_error(
                    "chmod",
                    "Expected exactly 2 arguments",
                ));
            }
            env.change_permissions(&command.args[0], &command.args[1])?;
        }
        "symlink" => {
            if command.args.len() != 2 {
                return Err(Error::command_error(
                    "symlink",
                    "Expected exactly 2 arguments: target link_name",
                ));
            }
            env.create_symlink(&command.args[0], &command.args[1])?;
        }
        "unquote" => {
            if command.args.len() != 1 {
                return Err(Error::command_error(
                    "unquote",
                    "Expected exactly 1 argument",
                ));
            }
            env.unquote_file(&command.args[0])?;
        }
        "grep" => {
            if command.args.len() < 2 {
                return Err(Error::command_error(
                    "grep",
                    "Expected at least 2 arguments",
                ));
            }
            let pattern = &command.args[0];
            let files = &command.args[1..];
            env.grep_files(pattern, files)?;
        }
        _ => {
            return Err(Error::UnknownCommand {
                command: command.name.clone(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_test_environment_creation() {
        let env = TestEnvironment::new().unwrap();
        assert!(env.work_dir.exists());
        assert!(env.work_dir.is_dir());
    }

    #[test]
    fn test_setup_files() {
        let mut env = TestEnvironment::new().unwrap();
        let files = vec![
            TxtarFile {
                name: "hello.txt".to_string(),
                contents: b"Hello, world!".to_vec(),
            },
            TxtarFile {
                name: "sub/dir/nested.txt".to_string(),
                contents: b"Nested content".to_vec(),
            },
        ];

        env.setup_files(&files).unwrap();

        let hello_path = env.work_dir.join("hello.txt");
        let nested_path = env.work_dir.join("sub/dir/nested.txt");

        assert!(hello_path.exists());
        assert!(nested_path.exists());

        assert_eq!(fs::read(&hello_path).unwrap(), b"Hello, world!");
        assert_eq!(fs::read(&nested_path).unwrap(), b"Nested content");
    }

    #[test]
    fn test_compare_files() {
        let env = TestEnvironment::new().unwrap();

        fs::write(env.work_dir.join("file1.txt"), "same content").unwrap();
        fs::write(env.work_dir.join("file2.txt"), "same content").unwrap();
        fs::write(env.work_dir.join("file3.txt"), "different content").unwrap();

        // Should succeed
        env.compare_files("file1.txt", "file2.txt").unwrap();

        // Should fail
        assert!(env.compare_files("file1.txt", "file3.txt").is_err());
    }
}
