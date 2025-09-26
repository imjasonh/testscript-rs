//! Error types for testscript-rs

use thiserror::Error;

/// Result type alias for testscript operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for testscript operations
#[derive(Error, Debug)]
pub enum Error {
    /// Regex error
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    /// IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// WalkDir error
    #[error("Directory walk error: {0}")]
    WalkDir(#[from] walkdir::Error),

    /// Parse error in test script
    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    /// Command execution error
    #[error("Command '{command}' failed: {message}")]
    Command { command: String, message: String },

    /// File comparison error
    #[error("File comparison failed: {message}")]
    FileCompare { message: String },

    /// Output comparison error
    #[error("Output comparison failed: expected {expected}, got {actual}")]
    OutputCompare { expected: String, actual: String },

    /// Unknown command error
    #[error("Unknown command: {command}")]
    UnknownCommand { command: String },

    /// Unknown condition error
    #[error("Unknown condition: {condition}")]
    UnknownCondition { condition: String },

    /// Generic error with message
    #[error("{0}")]
    Generic(String),

    /// Script execution error with context
    #[error("Error in {script_file} at line {line_num}:\n{context}")]
    ScriptError {
        script_file: String,
        line_num: usize,
        context: String,
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    /// Create a parse error
    pub fn parse_error(line: usize, message: impl Into<String>) -> Self {
        Error::Parse {
            line,
            message: message.into(),
        }
    }

    /// Create a command error
    pub fn command_error(command: impl Into<String>, message: impl Into<String>) -> Self {
        Error::Command {
            command: command.into(),
            message: message.into(),
        }
    }

    /// Create a script error with context
    pub fn script_error(
        script_file: impl Into<String>,
        line_num: usize,
        script_content: &str,
        source: Error,
    ) -> Self {
        let context = generate_error_context(script_content, line_num);
        Error::ScriptError {
            script_file: script_file.into(),
            line_num,
            context,
            source: Box::new(source),
        }
    }
}

/// Generate error context showing surrounding lines
fn generate_error_context(script_content: &str, error_line: usize) -> String {
    let lines: Vec<&str> = script_content.lines().collect();
    let mut context = String::new();

    let start = error_line.saturating_sub(3).max(1);
    let end = (error_line + 2).min(lines.len());

    for line_num in start..=end {
        if line_num == 0 {
            continue;
        }

        let line_content = lines.get(line_num - 1).unwrap_or(&"");

        if line_num == error_line {
            context.push_str(&format!("> {} | {}\n", line_num, line_content));
        } else {
            context.push_str(&format!("  {} | {}\n", line_num, line_content));
        }
    }

    context.trim_end().to_string()
}
