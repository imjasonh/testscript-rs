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

    // Create a test that will fail due to mismatched stdout expectation
    let test_content = r#"exec echo "hello"
stdout "goodbye"

-- test_file.txt --
Test content in custom root
"#;

    fs::write(testdata_path.join("custom_root_test.txt"), test_content).unwrap();

    // Run test with custom workdir root and preserve work on failure
    let result = testscript::run(testdata_path.to_string_lossy())
        .workdir_root(custom_root_path)
        .preserve_work_on_failure(true)
        .execute();

    // The test should fail due to the mismatched stdout expectation
    assert!(
        result.is_err(),
        "Test should fail due to mismatched stdout expectation. Got: {:?}",
        result
    );

    // Verify that the workdir was created in our custom root and contains expected files
    let entries: Vec<_> = fs::read_dir(custom_root_path)
        .expect("Custom root should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("Should be able to read directory entries");

    assert!(
        !entries.is_empty(),
        "Custom root should contain test working directories"
    );

    // Find the test working directory (should start with a temp directory name)
    let work_dir = entries
        .iter()
        .find(|entry| entry.file_type().unwrap().is_dir())
        .expect("Should find at least one working directory");

    let work_dir_path = work_dir.path();

    // Verify that the test file was created in the working directory
    let test_file_path = work_dir_path.join("test_file.txt");
    assert!(
        test_file_path.exists(),
        "test_file.txt should exist in the working directory"
    );

    let test_file_content =
        fs::read_to_string(&test_file_path).expect("Should be able to read test_file.txt");
    assert_eq!(
        test_file_content.trim(),
        "Test content in custom root",
        "Test file should contain expected content"
    );
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

    assert!(
        result.is_err(),
        "Expected error for non-existent workdir root"
    );
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("does not exist"),
        "Error should mention directory doesn't exist"
    );
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

    assert!(
        result.is_err(),
        "Expected error for file used as workdir root"
    );
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("not a directory"),
        "Error should mention path is not a directory"
    );
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
