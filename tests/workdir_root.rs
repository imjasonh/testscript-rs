//! Tests for workdir root configuration functionality

use std::fs;
use tempfile::TempDir;
use testscript_rs::testscript;

#[test]
fn test_workdir_root_basic_functionality() {
    // Create a temporary directory to use as our custom root
    let custom_root = TempDir::new().unwrap();
    let custom_root_path = custom_root.path();

    // Create a test directory structure
    let testdata_dir = TempDir::new().unwrap();
    let testdata_path = testdata_dir.path().join("testdata");
    fs::create_dir(&testdata_path).unwrap();

    let test_content = r#"exec echo "hello from custom root"
stdout "hello from custom root"

-- test_file.txt --
Test content in custom root
"#;

    fs::write(testdata_path.join("custom_root_test.txt"), test_content).unwrap();

    // Run test with custom workdir root - this should succeed
    let result = testscript::run(testdata_path.to_string_lossy())
        .workdir_root(custom_root_path)
        .execute();

    assert!(result.is_ok(), "Test with custom workdir root failed: {:?}", result);
}

#[test]
fn test_workdir_root_nonexistent_directory() {
    let testdata_dir = TempDir::new().unwrap();
    let testdata_path = testdata_dir.path().join("testdata");
    fs::create_dir(&testdata_path).unwrap();

    let test_content = r#"exec echo "test"
stdout "test"
"#;

    fs::write(testdata_path.join("simple_test.txt"), test_content).unwrap();

    // Try to use a non-existent directory as workdir root
    let nonexistent_path = "/this/path/does/not/exist";
    let result = testscript::run(testdata_path.to_string_lossy())
        .workdir_root(nonexistent_path)
        .execute();

    assert!(result.is_err(), "Expected error for non-existent workdir root");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("does not exist"), "Error should mention directory doesn't exist");
}

#[test]
fn test_workdir_root_not_a_directory() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a file (not directory) to test the validation
    let file_path = temp_dir.path().join("not_a_directory.txt");
    fs::write(&file_path, "this is a file").unwrap();

    let testdata_dir = TempDir::new().unwrap();
    let testdata_path = testdata_dir.path().join("testdata");
    fs::create_dir(&testdata_path).unwrap();

    let test_content = r#"exec echo "test"
stdout "test"
"#;

    fs::write(testdata_path.join("simple_test.txt"), test_content).unwrap();

    // Try to use a file as workdir root
    let result = testscript::run(testdata_path.to_string_lossy())
        .workdir_root(&file_path)
        .execute();

    assert!(result.is_err(), "Expected error for file used as workdir root");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("not a directory"), "Error should mention path is not a directory");
}


#[test]
fn test_workdir_root_with_preserve_work() {
    // Create a temporary directory to use as our custom root
    let custom_root = TempDir::new().unwrap();
    let custom_root_path = custom_root.path();

    // Create a test directory structure
    let testdata_dir = TempDir::new().unwrap();
    let testdata_path = testdata_dir.path().join("testdata");
    fs::create_dir(&testdata_path).unwrap();

    let test_content = r#"exec echo "combined features test"
stdout "combined features test"

-- config.txt --
Configuration file for testing
"#;

    fs::write(testdata_path.join("combined_test.txt"), test_content).unwrap();

    // Test that workdir_root can be combined with other features
    let result = testscript::run(testdata_path.to_string_lossy())
        .workdir_root(custom_root_path)
        .preserve_work_on_failure(true)
        .execute();

    assert!(result.is_ok(), "Combined features test failed: {:?}", result);
}



#[test]
fn test_workdir_root_api_chaining() {
    // Test that the API method exists and can be chained
    let custom_root = TempDir::new().unwrap();
    let testdata_dir = TempDir::new().unwrap();
    let testdata_path = testdata_dir.path().join("testdata");
    fs::create_dir(&testdata_path).unwrap();

    let test_content = r#"exec echo "chaining test"
stdout "chaining test"
"#;

    fs::write(testdata_path.join("chain_test.txt"), test_content).unwrap();

    let result = testscript::run(testdata_path.to_string_lossy())
        .workdir_root(custom_root.path())
        .workdir_root(custom_root.path()) // Can be called multiple times
        .preserve_work_on_failure(false)
        .execute();

    assert!(result.is_ok(), "API chaining test failed: {:?}", result);
}