//! Parser for .txtar format test scripts

use crate::error::{Error, Result};

/// Represents a single file block in the txtar archive
#[derive(Debug, Clone, PartialEq)]
pub struct TxtarFile {
    /// Name of the file (from the -- filename -- header)
    pub name: String,
    /// Contents of the file as bytes
    pub contents: Vec<u8>,
}

/// Represents a single command line in the script
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    /// The command name (first word)
    pub name: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Line number in the original script (for error reporting)
    pub line_num: usize,
    /// Optional condition prefix (e.g., "windows", "linux")
    pub condition: Option<String>,
    /// Whether this is a background command (ends with &)
    pub background: bool,
    /// Whether this command should be negated (starts with !)
    pub negated: bool,
}

/// Represents the parsed script and its associated files
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    /// List of commands to execute
    pub commands: Vec<Command>,
    /// List of files to create in the test environment
    pub files: Vec<TxtarFile>,
}

/// Parse a .txtar format string into a Script
///
/// The .txtar format consists of:
/// 1. Commands and comments at the top
/// 2. File blocks starting with "-- filename --" headers
///
/// # Arguments
/// * `content` - The raw content of the .txtar file
///
/// # Returns
/// A parsed Script containing commands and files
///
/// # Errors
/// Returns ParseError if the content is malformed
pub fn parse(content: &str) -> Result<Script> {
    let mut commands = Vec::new();
    let mut files = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    let mut current_file: Option<(String, Vec<u8>)> = None;

    while i < lines.len() {
        let line = lines[i];
        let line_num = i + 1; // 1-based line numbering

        // Skip empty lines
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Check for file header: -- filename --
        if let Some(filename) = parse_file_header(line) {
            // Save previous file if any
            if let Some((name, mut contents)) = current_file.take() {
                // Remove trailing newline if present (since we add one for every line)
                if contents.ends_with(b"\n") {
                    contents.pop();
                }
                files.push(TxtarFile { name, contents });
            }

            // Start new file
            current_file = Some((filename, Vec::new()));
            i += 1;
            continue;
        }

        // If we're inside a file block, add content to the current file
        if let Some((_, ref mut contents)) = current_file {
            // Add the line plus newline to file contents
            contents.extend_from_slice(line.as_bytes());
            contents.push(b'\n');
            i += 1;
            continue;
        }

        // Skip comment lines (outside of file blocks)
        if line.trim_start().starts_with('#') {
            i += 1;
            continue;
        }

        // Parse command line
        if let Some(command) = parse_command_line(line, line_num)? {
            commands.push(command);
        }

        i += 1;
    }

    // Save final file if any
    if let Some((name, mut contents)) = current_file.take() {
        // Remove trailing newline if present (since we add one for every line)
        if contents.ends_with(b"\n") {
            contents.pop();
        }
        files.push(TxtarFile { name, contents });
    }

    Ok(Script { commands, files })
}

/// Parse a file header line like "-- filename --"
fn parse_file_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("-- ") && trimmed.ends_with(" --") && trimmed.len() > 6 {
        let filename = &trimmed[3..trimmed.len() - 3];
        Some(filename.to_string())
    } else {
        None
    }
}

/// Parse a command line into a Command struct
fn parse_command_line(line: &str, line_num: usize) -> Result<Option<Command>> {
    let trimmed = line.trim();

    // Skip empty lines and comments
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }

    // Check for condition prefix like [windows] or [!unix]
    let (condition, command_part) = if trimmed.starts_with('[') {
        if let Some(end_bracket) = trimmed.find(']') {
            let condition_str = &trimmed[1..end_bracket];
            let remaining = trimmed[end_bracket + 1..].trim();
            (Some(condition_str.to_string()), remaining)
        } else {
            return Err(Error::parse_error(line_num, "Unclosed condition bracket"));
        }
    } else {
        (None, trimmed)
    };

    // Parse the command and arguments
    let tokens = parse_command_tokens(command_part)?;
    if tokens.is_empty() {
        return Ok(None);
    }

    // Check for negation (command starts with !)
    let (negated, command_name, args_start_idx) = if tokens[0] == "!" {
        if tokens.len() < 2 {
            return Err(Error::parse_error(line_num, "! requires a command"));
        }
        (true, tokens[1].clone(), 2)
    } else {
        (false, tokens[0].clone(), 1)
    };

    let mut args: Vec<String> = tokens.into_iter().skip(args_start_idx).collect();

    // Check for background command (ends with &)
    let background = if let Some(last_arg) = args.last() {
        if last_arg == "&" {
            args.pop(); // Remove the & from args
            true
        } else {
            false
        }
    } else {
        false
    };

    Ok(Some(Command {
        name: command_name,
        args,
        line_num,
        condition,
        background,
        negated,
    }))
}

/// Parse command tokens, handling quoted arguments
fn parse_command_tokens(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut just_closed_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' | '\'' => {
                if in_quotes && ch == quote_char {
                    // End of quoted string
                    in_quotes = false;
                    just_closed_quotes = true;
                } else if !in_quotes {
                    // Start of quoted string
                    in_quotes = true;
                    quote_char = ch;
                    just_closed_quotes = false;
                } else {
                    // Different quote type inside quotes - treat as literal
                    current_token.push(ch);
                    just_closed_quotes = false;
                }
            }
            ' ' | '\t' => {
                if in_quotes {
                    current_token.push(ch);
                    just_closed_quotes = false;
                } else if !current_token.is_empty() || just_closed_quotes {
                    tokens.push(current_token.clone());
                    current_token.clear();
                    just_closed_quotes = false;
                }
            }
            '\\' => {
                // Handle escape sequences
                if let Some(next_ch) = chars.next() {
                    if in_quotes && quote_char == '\'' {
                        // In single quotes, only process \\ and \'
                        match next_ch {
                            '\\' => current_token.push('\\'),
                            '\'' => current_token.push('\''),
                            'n' => current_token.push('\n'), // Still process \n in single quotes for Go compat
                            't' => current_token.push('\t'), // Still process \t in single quotes for Go compat
                            'r' => current_token.push('\r'), // Still process \r in single quotes for Go compat
                            _ => {
                                current_token.push('\\');
                                current_token.push(next_ch);
                            }
                        }
                    } else {
                        // In double quotes or outside quotes, process all escapes
                        match next_ch {
                            'n' => current_token.push('\n'),
                            't' => current_token.push('\t'),
                            'r' => current_token.push('\r'),
                            '\\' => current_token.push('\\'),
                            '"' => current_token.push('"'),
                            '\'' => current_token.push('\''),
                            _ => {
                                current_token.push('\\');
                                current_token.push(next_ch);
                            }
                        }
                    }
                } else {
                    current_token.push('\\');
                }
            }
            _ => {
                current_token.push(ch);
                just_closed_quotes = false;
            }
        }
    }

    // Add final token if any (including empty tokens that were quoted)
    if !current_token.is_empty() || just_closed_quotes {
        tokens.push(current_token);
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_header() {
        assert_eq!(
            parse_file_header("-- hello.txt --"),
            Some("hello.txt".to_string())
        );
        assert_eq!(
            parse_file_header("-- sub/dir/file.go --"),
            Some("sub/dir/file.go".to_string())
        );
        assert_eq!(parse_file_header("--hello--"), None);
        assert_eq!(parse_file_header("hello"), None);
    }

    #[test]
    fn test_parse_command_tokens() {
        let tokens = parse_command_tokens("exec echo hello").unwrap();
        assert_eq!(tokens, vec!["exec", "echo", "hello"]);

        let tokens = parse_command_tokens("exec echo \"hello world\"").unwrap();
        assert_eq!(tokens, vec!["exec", "echo", "hello world"]);

        let tokens = parse_command_tokens("exec echo \"escaped\\\"quote\"").unwrap();
        assert_eq!(tokens, vec!["exec", "echo", "escaped\"quote"]);

        let tokens = parse_command_tokens("stdout 'hello world\\n'").unwrap();
        assert_eq!(tokens, vec!["stdout", "hello world\n"]);

        // Test empty quoted strings
        let tokens = parse_command_tokens("stdout \"\"").unwrap();
        assert_eq!(tokens, vec!["stdout", ""]);

        let tokens = parse_command_tokens("stdout ''").unwrap();
        assert_eq!(tokens, vec!["stdout", ""]);

        // Test mixed empty and non-empty tokens
        let tokens = parse_command_tokens("exec echo \"\" \"hello\" \"\"").unwrap();
        assert_eq!(tokens, vec!["exec", "echo", "", "hello", ""]);
    }

    #[test]
    fn test_parse_command_line() {
        let cmd = parse_command_line("exec echo hello", 1).unwrap().unwrap();
        assert_eq!(cmd.name, "exec");
        assert_eq!(cmd.args, vec!["echo", "hello"]);
        assert_eq!(cmd.line_num, 1);
        assert_eq!(cmd.condition, None);
        assert!(!cmd.background);
        assert!(!cmd.negated);

        let cmd = parse_command_line("[windows] exec echo hello", 2)
            .unwrap()
            .unwrap();
        assert_eq!(cmd.condition, Some("windows".to_string()));

        let cmd = parse_command_line("exec echo hello &", 3).unwrap().unwrap();
        assert!(cmd.background);
        assert_eq!(cmd.args, vec!["echo", "hello"]);

        let cmd = parse_command_line("! exists missing_file", 4)
            .unwrap()
            .unwrap();
        assert!(cmd.negated);
        assert_eq!(cmd.name, "exists");
        assert_eq!(cmd.args, vec!["missing_file"]);
    }

    #[test]
    fn test_parse_basic_script() {
        let content = r#"# This is a comment
exec echo hello
cmp stdout expected.txt

-- expected.txt --
hello
"#;

        let script = parse(content).unwrap();

        assert_eq!(script.commands.len(), 2);
        assert_eq!(script.commands[0].name, "exec");
        assert_eq!(script.commands[0].args, vec!["echo", "hello"]);
        assert_eq!(script.commands[1].name, "cmp");
        assert_eq!(script.commands[1].args, vec!["stdout", "expected.txt"]);

        assert_eq!(script.files.len(), 1);
        assert_eq!(script.files[0].name, "expected.txt");
        assert_eq!(script.files[0].contents, b"hello");
    }

    #[test]
    fn test_parse_multiple_files() {
        let content = r#"exec cat file1.txt file2.txt
cmp stdout expected.txt

-- file1.txt --
first file
content

-- file2.txt --
second file

-- expected.txt --
first file
content
second file
"#;

        let script = parse(content).unwrap();

        assert_eq!(script.files.len(), 3);
        assert_eq!(script.files[0].name, "file1.txt");
        assert_eq!(script.files[0].contents, b"first file\ncontent");
        assert_eq!(script.files[1].name, "file2.txt");
        assert_eq!(script.files[1].contents, b"second file");
        assert_eq!(script.files[2].name, "expected.txt");
        assert_eq!(
            script.files[2].contents,
            b"first file\ncontent\nsecond file"
        );
    }
}
