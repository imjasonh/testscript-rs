//! Built-in command implementations

use crate::error::{Error, Result};
use crate::run::environment::TestEnvironment;
use regex::Regex;
use std::fs;
use std::path::Path;

impl TestEnvironment {
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
