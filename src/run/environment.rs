//! Test execution environment management

use crate::error::{Error, Result};
use crate::parser::TxtarFile;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command as StdCommand, Output, Stdio};
use tempfile::TempDir;

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
        Self::new_with_root(None)
    }

    /// Create a new test environment with a temporary directory in the specified root
    pub fn new_with_root(workdir_root: Option<&std::path::Path>) -> Result<Self> {
        let temp_dir = match workdir_root {
            Some(root) => {
                // Validate that the root directory exists and is writable
                if !root.exists() {
                    return Err(Error::Generic(format!(
                        "Workdir root directory does not exist: {}",
                        root.display()
                    )));
                }
                if !root.is_dir() {
                    return Err(Error::Generic(format!(
                        "Workdir root path is not a directory: {}",
                        root.display()
                    )));
                }
                // Test if the directory is writable by creating a temp directory
                tempfile::TempDir::new_in(root).map_err(|e| {
                    Error::Generic(format!(
                        "Cannot create temporary directory in workdir root {}: {}",
                        root.display(),
                        e
                    ))
                })?
            }
            None => tempfile::tempdir()?,
        };
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
            // Enable DOTALL mode (?s) for . to match newlines
            // Enable Unicode mode (?u) for proper Unicode character matching
            let regex_pattern = format!("(?su){}", expected_substituted);
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

    /// Compare command output with expected content and count matches
    pub fn compare_output_with_count(
        &self,
        output_type: &str,
        expected: &str,
        expected_count: usize,
    ) -> Result<()> {
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

        // Use regex to count matches
        let regex_pattern = if expected_substituted.contains('^')
            || expected_substituted.contains('$')
            || expected_substituted.contains('[')
            || expected_substituted.contains('(')
            || expected_substituted.contains('*')
            || expected_substituted.contains('.')
        {
            // Already a regex pattern
            format!("(?su){}", expected_substituted)
        } else {
            // Treat as literal string and escape for regex
            format!("(?su){}", regex::escape(&expected_substituted))
        };

        let regex = Regex::new(&regex_pattern)
            .map_err(|e| Error::command_error(output_type, format!("Invalid regex: {}", e)))?;

        let match_count = regex.find_iter(&actual).count();

        if match_count != expected_count {
            return Err(Error::OutputCompare {
                expected: format!("{} (count: {})", expected_substituted, expected_count),
                actual: format!("{} (count: {})", actual, match_count),
            });
        }

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

    /// Substitute environment variables in a string
    pub fn substitute_env_vars(&self, input: &str) -> String {
        let mut result = input.to_string();

        // First handle ${VAR@R} patterns for regex quoting
        for (key, value) in &self.env_vars {
            let regex_quoted_pattern = format!("${{{}@R}}", key);
            if result.contains(&regex_quoted_pattern) {
                // Escape regex metacharacters in the value
                let escaped_value = regex::escape(value);
                result = result.replace(&regex_quoted_pattern, &escaped_value);
            }
        }

        // Then handle normal $VAR and ${VAR} patterns
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

    /// Preserve the work directory by preventing TempDir cleanup
    /// Returns the path to the preserved directory
    pub fn preserve_work_dir(self) -> std::path::PathBuf {
        // Use the idiomatic way to preserve a TempDir
        self._temp_dir.keep()
    }
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
