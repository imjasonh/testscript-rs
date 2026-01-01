//! Error types for testscript-rs

use thiserror::Error;

#[cfg(feature = "colors")]
use termcolor::Color;

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
    #[error("{}", format_output_comparison(.expected, .actual))]
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
    #[error("Error in {script_file} at line {line_num}:\n{context}\n\n{source}")]
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
            #[cfg(feature = "colors")]
            {
                if atty::is(atty::Stream::Stderr) {
                    context.push_str(&format!(
                        "\x1b[31m> {} | {}\x1b[0m\n",
                        line_num, line_content
                    ));
                } else {
                    context.push_str(&format!("> {} | {}\n", line_num, line_content));
                }
            }
            #[cfg(not(feature = "colors"))]
            context.push_str(&format!("> {} | {}\n", line_num, line_content));
        } else {
            context.push_str(&format!("  {} | {}\n", line_num, line_content));
        }
    }

    context.trim_end().to_string()
}

/// Format output comparison error for better readability
fn format_output_comparison(expected: &str, actual: &str) -> String {
    #[cfg(feature = "colors")]
    {
        if atty::is(atty::Stream::Stderr) {
            return format_output_comparison_colored(expected, actual);
        }
    }

    format_output_comparison_plain(expected, actual)
}

#[cfg(feature = "colors")]
fn format_output_comparison_colored(expected: &str, actual: &str) -> String {
    // Handle empty strings specially
    if expected.is_empty() && actual.is_empty() {
        return "Both expected and actual output are empty".to_string();
    }
    if expected.is_empty() {
        return format!(
            "Expected empty output, but got:\n{}",
            colorize_output_value(actual, Color::Red)
        );
    }
    if actual.is_empty() {
        return format!(
            "Expected output:\n{}\nBut got empty output",
            colorize_output_value(expected, Color::Green)
        );
    }

    // For short outputs, show inline
    if expected.len() <= 50
        && actual.len() <= 50
        && !expected.contains('\n')
        && !actual.contains('\n')
    {
        return format!(
            "Expected: {}\n  Actual: {}",
            colorize_text(format!("'{}'", expected), Color::Green),
            colorize_text(format!("'{}'", actual), Color::Red)
        );
    }

    // For longer outputs, show formatted comparison
    format!(
        "Output mismatch:\n\n{}:\n{}\n\n{}:\n{}",
        colorize_text("Expected".to_string(), Color::Green),
        colorize_output_value(expected, Color::Green),
        colorize_text("Actual".to_string(), Color::Red),
        colorize_output_value(actual, Color::Red)
    )
}

#[cfg(feature = "colors")]
fn colorize_text(text: String, color: Color) -> String {
    // Simple ANSI color codes for basic coloring
    let color_code = match color {
        Color::Red => "\x1b[31m",
        Color::Green => "\x1b[32m",
        _ => "",
    };
    let reset = "\x1b[0m";

    if color_code.is_empty() {
        text
    } else {
        format!("{}{}{}", color_code, text, reset)
    }
}

#[cfg(feature = "colors")]
fn colorize_output_value(value: &str, color: Color) -> String {
    let formatted = format_output_value_plain(value);
    colorize_text(formatted, color)
}

fn format_output_comparison_plain(expected: &str, actual: &str) -> String {
    // Handle empty strings specially
    if expected.is_empty() && actual.is_empty() {
        return "Both expected and actual output are empty".to_string();
    }
    if expected.is_empty() {
        return format!(
            "Expected empty output, but got:\n{}",
            format_output_value(actual)
        );
    }
    if actual.is_empty() {
        return format!(
            "Expected output:\n{}\nBut got empty output",
            format_output_value(expected)
        );
    }

    // For short outputs, show inline
    if expected.len() <= 50
        && actual.len() <= 50
        && !expected.contains('\n')
        && !actual.contains('\n')
    {
        return format!("Expected: '{}'\n  Actual: '{}'", expected, actual);
    }

    // For longer outputs, show formatted comparison
    format!(
        "Output mismatch:\n\nExpected:\n{}\n\nActual:\n{}",
        format_output_value(expected),
        format_output_value(actual)
    )
}

/// Format a single output value for display
fn format_output_value(value: &str) -> String {
    format_output_value_plain(value)
}

fn format_output_value_plain(value: &str) -> String {
    if value.is_empty() {
        return "<empty>".to_string();
    }

    // Add line numbers for multi-line output
    if value.contains('\n') {
        let lines: Vec<&str> = value.lines().collect();
        if lines.len() > 1 {
            return lines
                .iter()
                .enumerate()
                .map(|(i, line)| format!("  {} | {}", i + 1, line))
                .collect::<Vec<_>>()
                .join("\n");
        }
    }

    // For single line, add quotes if it contains special characters
    if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        format!("'{}'", value)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_output_comparison_simple() {
        let result = format_output_comparison("expected", "actual");
        assert!(result.contains("Expected: 'expected'\n  Actual: 'actual'"));
    }

    #[test]
    fn test_format_output_comparison_empty() {
        let result = format_output_comparison("", "");
        assert_eq!(result, "Both expected and actual output are empty");

        let result = format_output_comparison("", "something");
        assert!(result.contains("Expected empty output, but got:"));
        assert!(result.contains("something"));

        let result = format_output_comparison("expected", "");
        assert!(result.contains("Expected output:"));
        assert!(result.contains("But got empty output"));
    }

    #[test]
    fn test_format_output_comparison_multiline() {
        let expected = "line1\nline2\nline3";
        let actual = "line1\nwrong\nline3";
        let result = format_output_comparison(expected, actual);

        assert!(result.contains("Output mismatch:"));
        assert!(result.contains("Expected:"));
        assert!(result.contains("Actual:"));
        assert!(result.contains("1 | line1"));
        assert!(result.contains("2 | line2"));
        assert!(result.contains("2 | wrong"));
        assert!(result.contains("3 | line3"));
    }

    #[test]
    fn test_format_output_value_empty() {
        assert_eq!(format_output_value(""), "<empty>");
    }

    #[test]
    fn test_format_output_value_simple() {
        assert_eq!(format_output_value("simple"), "simple");
        assert_eq!(format_output_value("with spaces"), "'with spaces'");
        assert_eq!(format_output_value("with\ttabs"), "'with\ttabs'");
    }

    #[test]
    fn test_format_output_value_multiline() {
        let result = format_output_value("line1\nline2\nline3");
        assert!(result.contains("1 | line1"));
        assert!(result.contains("2 | line2"));
        assert!(result.contains("3 | line3"));
    }

    #[test]
    fn test_format_output_value_single_line_with_newline() {
        let result = format_output_value("single line\n");
        assert_eq!(result, "'single line\n'");
    }

    #[test]
    fn test_error_creation() {
        let parse_err = Error::parse_error(42, "test message");
        match parse_err {
            Error::Parse { line, message } => {
                assert_eq!(line, 42);
                assert_eq!(message, "test message");
            }
            _ => panic!("Wrong error type"),
        }

        let cmd_err = Error::command_error("test_cmd", "test failure");
        match cmd_err {
            Error::Command { command, message } => {
                assert_eq!(command, "test_cmd");
                assert_eq!(message, "test failure");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_script_error_creation() {
        let source = Error::command_error("test", "failed");
        let script_err = Error::script_error(
            "test.txt",
            5,
            "line1\nline2\nline3\nline4\nline5\nline6",
            source,
        );

        match script_err {
            Error::ScriptError {
                script_file,
                line_num,
                context,
                ..
            } => {
                assert_eq!(script_file, "test.txt");
                assert_eq!(line_num, 5);
                assert!(context.contains("> 5 | line5"));
                assert!(context.contains("  3 | line3"));
                assert!(context.contains("  4 | line4"));
                assert!(context.contains("  6 | line6"));
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_generate_error_context() {
        let script = "line1\nline2\nline3\nline4\nline5\nline6\nline7";
        let context = generate_error_context(script, 4);

        assert!(context.contains("  1 | line1"));
        assert!(context.contains("  2 | line2"));
        assert!(context.contains("  3 | line3"));
        assert!(context.contains("> 4 | line4"));
        assert!(context.contains("  5 | line5"));
        assert!(context.contains("  6 | line6"));
        assert!(!context.contains("line7"));
    }

    #[test]
    fn test_generate_error_context_edge_cases() {
        // Test first line error
        let script = "line1\nline2\nline3";
        let context = generate_error_context(script, 1);
        assert!(context.contains("> 1 | line1"));
        assert!(context.contains("  2 | line2"));
        assert!(context.contains("  3 | line3"));

        // Test last line error
        let context = generate_error_context(script, 3);
        assert!(context.contains("  1 | line1"));
        assert!(context.contains("  2 | line2"));
        assert!(context.contains("> 3 | line3"));

        // Test empty script
        let context = generate_error_context("", 1);
        assert!(context.is_empty());
    }

    #[test]
    fn test_output_compare_error_display() {
        let error = Error::OutputCompare {
            expected: "test".to_string(),
            actual: "different".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Expected: 'test'"));
        assert!(display.contains("Actual: 'different'"));
    }
}
