//! Command execution logic

use crate::error::{Error, Result};
use crate::parser::Command;
use crate::run::{environment::TestEnvironment, params::RunParams};
use std::fs;
use std::path::Path;

/// Information about a script update needed when in update mode
#[derive(Debug, Clone)]
pub struct ScriptUpdate {
    /// The line number where the stdout/stderr command appears
    pub line_num: usize,
    /// The command name (stdout or stderr)
    pub command_name: String,
    /// The new expected output
    pub new_output: String,
}

/// Run a single script with the given parameters - main implementation
pub fn run_script_impl(script_path: &Path, params: &RunParams) -> Result<()> {
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

    // Track script updates if we're in update mode
    let mut updates = Vec::new();

    // Execute commands
    for command in &script.commands {
        let result = execute_command(&mut env, command, params);

        if let Err(e) = result {
            // If we're in update mode and this is an output comparison error, capture the update
            if params.update_scripts {
                if let Error::OutputCompare {
                    expected: _,
                    actual,
                } = &e
                {
                    if command.name == "stdout" || command.name == "stderr" {
                        updates.push(ScriptUpdate {
                            line_num: command.line_num,
                            command_name: command.name.clone(),
                            new_output: actual.clone(),
                        });
                        // Continue instead of failing
                        continue;
                    }
                }
            }

            // Handle work directory preservation on failure
            if params.preserve_work_on_failure {
                let preserved_path = env.preserve_work_dir();
                eprintln!("Test failed. Work directory preserved at: {}", preserved_path.display());
                eprintln!("You can inspect the test environment:");
                eprintln!("  cd {}", preserved_path.display());
                eprintln!("  ls -la");
            }

            // Wrap error with script context for non-update cases or non-output errors
            return Err(Error::script_error(
                &script_file,
                command.line_num,
                &content,
                e,
            ));
        }

        // Check for early termination
        if env.should_skip {
            // Handle work directory preservation for skipped tests if configured
            if params.preserve_work_on_failure {
                let preserved_path = env.preserve_work_dir();
                eprintln!("Test skipped. Work directory preserved at: {}", preserved_path.display());
                eprintln!("You can inspect the test environment:");
                eprintln!("  cd {}", preserved_path.display());
                eprintln!("  ls -la");
            }
            return Err(Error::Generic("Test skipped".to_string()));
        }
        if env.should_stop {
            break; // Stop early but don't fail
        }
    }

    // Apply updates if any were collected
    if !updates.is_empty() && params.update_scripts {
        apply_script_updates(script_path, &content, &updates)?;
    }

    // Wait for any remaining background processes - handle failures here too
    let background_names: Vec<String> = env.background_processes.keys().cloned().collect();
    for name in background_names {
        if let Err(e) = env.wait_for_background(&name) {
            if params.preserve_work_on_failure {
                let preserved_path = env.preserve_work_dir();
                eprintln!("Test failed during background process cleanup. Work directory preserved at: {}", preserved_path.display());
                eprintln!("You can inspect the test environment:");
                eprintln!("  cd {}", preserved_path.display());
                eprintln!("  ls -la");
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Apply script updates to the actual file
fn apply_script_updates(script_path: &Path, content: &str, updates: &[ScriptUpdate]) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let mut updated_lines = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line_num = i + 1; // Line numbers are 1-based

        // Check if this line needs to be updated
        if let Some(update) = updates.iter().find(|u| u.line_num == line_num) {
            // This is a stdout/stderr command that needs updating
            let line = lines[i];
            let cmd_part = format!("{} ", update.command_name);

            if line.trim_start().starts_with(&cmd_part) {
                // Extract the indentation from the original line
                let indent = line.len() - line.trim_start().len();
                let indent_str = " ".repeat(indent);

                // Create the updated line with proper quoting
                let quoted_output = if update.new_output.contains(' ')
                    || update.new_output.contains('\n')
                    || update.new_output.contains('"')
                {
                    // Use proper shell quoting for complex strings
                    format!("\"{}\"", update.new_output.replace('"', "\\\""))
                } else if update.new_output.is_empty() {
                    "\"-\"".to_string()
                } else {
                    update.new_output.clone()
                };

                updated_lines.push(format!(
                    "{}{} {}",
                    indent_str, update.command_name, quoted_output
                ));
            } else {
                // Shouldn't happen, but preserve the original line if it doesn't match
                updated_lines.push(line.to_string());
            }
        } else {
            // Keep the original line
            updated_lines.push(lines[i].to_string());
        }
        i += 1;
    }

    // Write the updated content back to the file
    let updated_content = updated_lines.join("\n");
    if updated_content != content {
        fs::write(script_path, updated_content)?;
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
        } else if condition.starts_with("env:") {
            RunParams::check_env_condition(condition)
        } else if let Some(program) = condition.strip_prefix("exec:") {
            RunParams::program_exists(program)
        } else if let Some(base_condition) = condition.strip_prefix('!') {
            // Handle negated conditions
            if let Some(value) = params.conditions.get(base_condition) {
                !value
            } else if base_condition.starts_with("env:") {
                // Dynamic negated environment variable condition
                !RunParams::check_env_condition(base_condition)
            } else if let Some(program) = base_condition.strip_prefix("exec:") {
                !RunParams::program_exists(program)
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
