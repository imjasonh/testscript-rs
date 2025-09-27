//! Tests for update scripts functionality

use std::fs;
use tempfile::TempDir;
use testscript_rs::testscript;

#[test]
fn test_update_scripts_stdout() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Create a test script with incorrect expected output
    let test_content = r#"exec echo "actual output"
stdout "expected output"
"#;

    let test_file = testdata_dir.join("update_test.txt");
    fs::write(&test_file, test_content).unwrap();

    // Run with update_scripts enabled - should update the file instead of failing
    let result = testscript::run(testdata_dir.to_string_lossy())
        .update_scripts(true)
        .execute();

    assert!(result.is_ok(), "Update should succeed: {:?}", result);

    // Check that the file was updated with the actual output
    let updated_content = fs::read_to_string(&test_file).unwrap();
    assert!(
        updated_content.contains("stdout \"actual output\""),
        "File should be updated with actual output, got: {}",
        updated_content
    );
}

#[test]
fn test_update_scripts_stderr() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Create a test script with incorrect expected stderr output
    let test_content = r#"exec sh -c "echo 'error message' >&2"
stderr "wrong error message"
"#;

    let test_file = testdata_dir.join("stderr_update_test.txt");
    fs::write(&test_file, test_content).unwrap();

    // Run with update_scripts enabled
    let result = testscript::run(testdata_dir.to_string_lossy())
        .update_scripts(true)
        .execute();

    assert!(result.is_ok(), "Update should succeed: {:?}", result);

    // Check that the file was updated
    let updated_content = fs::read_to_string(&test_file).unwrap();
    assert!(
        updated_content.contains("stderr \"error message\""),
        "File should be updated with actual stderr output, got: {}",
        updated_content
    );
}

#[test]
fn test_update_scripts_via_env_var() {
    // Ensure clean state
    std::env::remove_var("UPDATE_SCRIPTS");

    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Create a test script with incorrect expected output
    let test_content = r#"exec echo "env test output"
stdout "wrong output"
"#;

    let test_file = testdata_dir.join("env_update_test.txt");
    fs::write(&test_file, test_content).unwrap();

    // Set the environment variable
    std::env::set_var("UPDATE_SCRIPTS", "1");

    // Run without explicitly setting update_scripts - should read from env var
    let result = testscript::run(testdata_dir.to_string_lossy()).execute();

    // Clean up the env var immediately after the test
    std::env::remove_var("UPDATE_SCRIPTS");

    assert!(
        result.is_ok(),
        "Update via env var should succeed: {:?}",
        result
    );

    // Check that the file was updated
    let updated_content = fs::read_to_string(&test_file).unwrap();
    assert!(
        updated_content.contains("stdout \"env test output\""),
        "File should be updated via env var, got: {}",
        updated_content
    );
}

#[test]
fn test_normal_mode_still_fails() {
    // Ensure UPDATE_SCRIPTS env var is not set
    std::env::remove_var("UPDATE_SCRIPTS");

    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Create a test script with incorrect expected output
    let test_content = r#"exec echo "actual output"
stdout "expected output"
"#;

    let test_file = testdata_dir.join("normal_test.txt");
    fs::write(&test_file, test_content).unwrap();

    // Explicitly set update_scripts to false to override any env var
    let result = testscript::run(testdata_dir.to_string_lossy())
        .update_scripts(false)
        .execute();

    assert!(
        result.is_err(),
        "Normal mode should still fail on mismatch: {:?}",
        result
    );

    // Check that the file was NOT updated
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(
        content, test_content,
        "File should not be modified in normal mode"
    );
}
