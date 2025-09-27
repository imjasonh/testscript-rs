//! Tests for network and advanced condition support

use std::fs;
use tempfile::TempDir;
use testscript_rs::{run::RunParams, testscript};

#[test]
fn test_env_conditions() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Set an environment variable for testing
    std::env::set_var("TEST_CONDITION", "true");

    let test_content = r#"[env:TEST_CONDITION] exec echo "env condition works"
stdout "env condition works"

[!env:NONEXISTENT_VAR] exec echo "negated env condition works"
stdout "negated env condition works"

[env:NONEXISTENT_VAR] exec echo "should be skipped"
! stdout "should be skipped"
"#;

    fs::write(testdata_dir.join("env_test.txt"), test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    assert!(result.is_ok(), "Environment condition test failed: {:?}", result);

    // Clean up
    std::env::remove_var("TEST_CONDITION");
}

#[test]
fn test_auto_detect_network() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Create a test that should work whether network is available or not
    let test_content = r#"[net] exec echo "network available"
[!net] exec echo "network not available"

# At least one should execute
exec echo "test completed"
stdout "test completed"
"#;

    fs::write(testdata_dir.join("network_test.txt"), test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy())
        .auto_detect_network()
        .execute();
    assert!(result.is_ok(), "Network condition test failed: {:?}", result);
}

#[test]
fn test_auto_detect_programs() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test with echo which should be available on all platforms
    let test_content = r#"[exec:echo] exec echo "echo is available"
stdout "echo is available"

[!exec:nonexistent_program_xyz] exec echo "nonexistent program not found"
stdout "nonexistent program not found"
"#;

    fs::write(testdata_dir.join("program_test.txt"), test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy())
        .auto_detect_programs(&["echo", "nonexistent_program_xyz"])
        .execute();
    assert!(result.is_ok(), "Program detection test failed: {:?}", result);
}

#[test]
fn test_runparams_condition_helpers() {
    // Test the helper functions directly
    std::env::set_var("HELPER_TEST", "value");

    assert!(RunParams::check_env_condition("env:HELPER_TEST"));
    assert!(!RunParams::check_env_condition("env:NONEXISTENT"));
    assert!(!RunParams::check_env_condition("not_env_condition"));

    std::env::remove_var("HELPER_TEST");
}

#[test]
fn test_combined_conditions() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Set environment for testing
    std::env::set_var("COMBINED_TEST", "true");

    let test_content = r#"# Test combining different condition types
# Use separate lines since parser doesn't support multiple conditions per line
[unix] exec echo "unix detected"
[windows] exec echo "windows detected"

# This should work on any platform with the env var
[env:COMBINED_TEST] exec echo "env var is set"
stdout "env var is set"
"#;

    fs::write(testdata_dir.join("combined_test.txt"), test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy())
        .auto_detect_network()
        .auto_detect_programs(&["echo"])
        .execute();
    assert!(result.is_ok(), "Combined condition test failed: {:?}", result);

    // Clean up
    std::env::remove_var("COMBINED_TEST");
}