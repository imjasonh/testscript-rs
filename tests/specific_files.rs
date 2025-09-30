//! Tests for running specific test files functionality

use std::fs;
use tempfile::TempDir;
use testscript_rs::testscript;

#[test]
fn test_files_basic_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Create multiple test files
    let hello_content = r#"exec echo "Hello"
stdout "Hello"
"#;

    let exists_content = r#"exists test_file.txt

-- test_file.txt --
content"#;

    let skip_content = r#"skip 'This test should be skipped'
exec echo "This should not run"
exec exit 1
"#;

    fs::write(testdata_dir.join("hello.txt"), hello_content).unwrap();
    fs::write(testdata_dir.join("exists.txt"), exists_content).unwrap();
    fs::write(testdata_dir.join("skip.txt"), skip_content).unwrap();

    // Test running specific files (should only run hello.txt and exists.txt)
    let result = testscript::run(testdata_dir.to_string_lossy())
        .files(["hello.txt", "exists.txt"])
        .execute();

    assert!(result.is_ok(), "Specific files test failed: {:?}", result);
}

#[test]
fn test_files_with_vec_string() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    let test_content = r#"exec echo "Test"
stdout "Test"
"#;

    fs::write(testdata_dir.join("test1.txt"), test_content).unwrap();
    fs::write(testdata_dir.join("test2.txt"), test_content).unwrap();

    // Test with Vec<String>
    let test_files = vec!["test1.txt".to_string(), "test2.txt".to_string()];
    let result = testscript::run(testdata_dir.to_string_lossy())
        .files(test_files)
        .execute();

    assert!(
        result.is_ok(),
        "Vec<String> files test failed: {:?}",
        result
    );
}

#[test]
fn test_files_with_relative_paths() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    let subdir = testdata_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();

    let test_content = r#"exec echo "Relative test"
stdout "Relative test"
"#;

    fs::write(testdata_dir.join("test.txt"), test_content).unwrap();
    fs::write(subdir.join("nested.txt"), test_content).unwrap();

    // Test with relative paths including subdirectories
    let result = testscript::run(testdata_dir.to_string_lossy())
        .files(["test.txt", "subdir/nested.txt"])
        .execute();

    assert!(result.is_ok(), "Relative paths test failed: {:?}", result);
}

#[test]
fn test_files_with_absolute_paths() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    let test_content = r#"exec echo "Absolute test"
stdout "Absolute test"
"#;

    let test_file = testdata_dir.join("absolute_test.txt");
    fs::write(&test_file, test_content).unwrap();

    // Test with absolute path
    let result = testscript::run(testdata_dir.to_string_lossy())
        .files([test_file.to_string_lossy().to_string()])
        .execute();

    assert!(result.is_ok(), "Absolute paths test failed: {:?}", result);
}

#[test]
fn test_files_nonexistent_file_error() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test with nonexistent file
    let result = testscript::run(testdata_dir.to_string_lossy())
        .files(["nonexistent.txt"])
        .execute();

    assert!(result.is_err(), "Expected error for nonexistent file");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("Test file not found"),
        "Error message should mention file not found: {}",
        error_msg
    );
}

#[test]
fn test_files_empty_list_error() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test with empty file list
    let empty_files: Vec<String> = vec![];
    let result = testscript::run(testdata_dir.to_string_lossy())
        .files(empty_files)
        .execute();

    assert!(result.is_err(), "Expected error for empty file list");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("No test files specified"),
        "Error message should mention no files specified: {}",
        error_msg
    );
}

#[test]
fn test_files_directory_instead_of_file_error() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    let subdir = testdata_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();

    // Test with directory instead of file
    let result = testscript::run(testdata_dir.to_string_lossy())
        .files(["subdir"])
        .execute();

    assert!(
        result.is_err(),
        "Expected error for directory instead of file"
    );
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("Path is not a file"),
        "Error message should mention path not a file: {}",
        error_msg
    );
}
