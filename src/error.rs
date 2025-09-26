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
}