//! Tests for enhanced error messages

use std::fs;
use tempfile::TempDir;
use testscript_rs::run_test;

#[test]
fn test_error_messages() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("failing_test.txt");

    let script_content = r#"# Test that should fail with good error message
exec echo "this works"
stdout "this works"

# This command will fail
exec nonexistent-command arg1 arg2
stdout "should not get here"

# More context
exec echo "after failure""#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);

    // This should fail, but with enhanced error messages
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    println!("Enhanced error message:\n{}", error_msg);

    // Verify the error contains context
    assert!(error_msg.contains("failing_test.txt"));
    assert!(error_msg.contains("line"));
    assert!(error_msg.contains("nonexistent-command"));
}

#[test]
fn test_parse_error_context() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("parse_error.txt");

    let script_content = r#"# Test parse error
exec echo "good command"
[unclosed condition exec echo "bad"
exec echo "after error""#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);

    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    println!("Parse error message:\n{}", error_msg);

    // Should show context around the parse error
    assert!(error_msg.contains("parse_error.txt"));
    assert!(error_msg.contains("unclosed"));
}
