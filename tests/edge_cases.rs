//! Edge case tests for testscript-rs parsing and execution

use std::fs;
use tempfile::TempDir;
use testscript_rs::{parser, run_test};

#[test]
fn test_parse_edge_cases() {
    // Test empty script
    let empty = parser::parse("").unwrap();
    assert_eq!(empty.commands.len(), 0);
    assert_eq!(empty.files.len(), 0);

    // Test script with only comments
    let comments_only = parser::parse("# Just a comment\n# Another comment").unwrap();
    assert_eq!(comments_only.commands.len(), 0);
    assert_eq!(comments_only.files.len(), 0);

    // Test script with only files
    let files_only = parser::parse("-- file.txt --\ncontent").unwrap();
    assert_eq!(files_only.commands.len(), 0);
    assert_eq!(files_only.files.len(), 1);

    // Test commands with complex quoting
    let complex_quotes = r#"exec echo "hello \"world\" with spaces"
exec echo 'single quotes'
exec echo "newline\nand\ttab""#;
    let script = parser::parse(complex_quotes).unwrap();
    assert_eq!(script.commands.len(), 3);
    assert_eq!(script.commands[0].args[1], r#"hello "world" with spaces"#);
    assert_eq!(script.commands[2].args[1], "newline\nand\ttab");
}

#[test]
fn test_parse_malformed_conditions() {
    // Test unclosed condition bracket
    let malformed = parser::parse("[unclosed exec echo hello");
    assert!(malformed.is_err());

    // Test empty condition
    let empty_condition = parser::parse("[] exec echo hello");
    let script = empty_condition.unwrap();
    assert_eq!(script.commands[0].condition, Some("".to_string()));
}

#[test]
fn test_files_with_special_names() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("special_names.txt");

    let script_content = r#"# Test files with special characters in names
cmp file-with-dashes.txt file_with_underscores.txt
cmp path/to/nested/file.txt another/nested/file.txt

-- file-with-dashes.txt --
content

-- file_with_underscores.txt --
content

-- path/to/nested/file.txt --
nested content

-- another/nested/file.txt --
nested content"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(
        result.is_ok(),
        "Special file names test failed: {:?}",
        result
    );
}

#[test]
fn test_commands_with_many_args() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("many_args.txt");

    // Test command with many arguments
    let script_content = r#"# Test commands with many arguments
exec echo arg1 arg2 arg3 "arg with spaces" arg5 arg6
stdout "arg1 arg2 arg3 arg with spaces arg5 arg6""#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Many args test failed: {:?}", result);
}

#[test]
fn test_binary_file_content() {
    // Test that binary content is preserved correctly
    let content_with_binary = "-- binary.dat --\nhello\x00world\x7f\x00";
    let script = parser::parse(content_with_binary).unwrap();

    assert_eq!(script.files.len(), 1);
    assert_eq!(script.files[0].name, "binary.dat");
    assert_eq!(script.files[0].contents, b"hello\x00world\x7f\x00");
}

#[test]
fn test_file_with_dashes_in_content() {
    // Test that content with dashes (but not file headers) doesn't confuse parser
    let content = r#"-- config.txt --
--this is not a file header because no spaces--
just content with dashes
--more dashes without spaces--
- single dash line"#;

    let script = parser::parse(content).unwrap();
    assert_eq!(script.files.len(), 1);
    assert_eq!(script.files[0].name, "config.txt");
    let expected_content = "--this is not a file header because no spaces--\njust content with dashes\n--more dashes without spaces--\n- single dash line";
    assert_eq!(script.files[0].contents, expected_content.as_bytes());
}

#[test]
fn test_command_line_numbers() {
    let content = r#"# Comment line 1
exec echo line2
# Comment line 3
stdout line4
# Comment line 5
exec echo line6"#;

    let script = parser::parse(content).unwrap();
    assert_eq!(script.commands.len(), 3);
    assert_eq!(script.commands[0].line_num, 2); // exec echo line2
    assert_eq!(script.commands[1].line_num, 4); // stdout line4
    assert_eq!(script.commands[2].line_num, 6); // exec echo line6
}

#[test]
fn test_mixed_conditions() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("mixed_conditions.txt");

    let script_content = r#"# Test mixed condition types
[unix] exec echo "unix"
[!windows] exec echo "not windows"
exec echo "always runs"
[linux] exec echo "linux only"
[!release] exec echo "not release mode""#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Mixed conditions test failed: {:?}", result);
}

#[test]
fn test_empty_command_args() {
    let content = r#"exec
cmp file1.txt file2.txt

-- file1.txt --
-- file2.txt --"#;

    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("empty_args.txt");
    fs::write(&script_path, content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_err(), "Empty exec command should fail");
}
