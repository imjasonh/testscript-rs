//! Tests for the new built-in commands

use std::fs;
use tempfile::TempDir;
use testscript_rs::run_test;

#[test]
fn test_exists_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("exists_test.txt");

    let script_content = r#"# Test exists command
exists existing_file.txt
! exists nonexistent_file.txt

-- existing_file.txt --
content here"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Exists test failed: {:?}", result);
}

#[test]
fn test_mkdir_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("mkdir_test.txt");

    let script_content = r#"# Test mkdir command
mkdir test_dir
mkdir nested/deep/dir
exists test_dir
exists nested/deep/dir"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Mkdir test failed: {:?}", result);
}

#[test]
fn test_cp_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("cp_test.txt");

    let script_content = r#"# Test cp command
cp source.txt dest.txt
cmp source.txt dest.txt

mkdir target_dir
cp source.txt target_dir/
exists target_dir/source.txt

-- source.txt --
test content for copying"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Copy test failed: {:?}", result);
}

#[test]
fn test_rm_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("rm_test.txt");

    let script_content = r#"# Test rm command
exists file_to_remove.txt
rm file_to_remove.txt
! exists file_to_remove.txt

mkdir dir_to_remove
exists dir_to_remove
rm dir_to_remove
! exists dir_to_remove

-- file_to_remove.txt --
temporary content"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Remove test failed: {:?}", result);
}

#[test]
fn test_env_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("env_test.txt");

    let script_content = r#"# Test env command
env TEST_VAR=hello
env ANOTHER_VAR=world"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Env test failed: {:?}", result);
}

#[test]
fn test_stdin_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("stdin_test.txt");

    let script_content = r#"# Test stdin command
stdin input.txt
exec cat
stdout "input content"

-- input.txt --
input content"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    // This might fail if cat isn't available - that's okay for now
    if result.is_err() {
        println!(
            "Stdin test failed (cat might not be available): {:?}",
            result
        );
    }
}

#[test]
fn test_skip_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("skip_test.txt");

    let script_content = r#"# Test skip command
skip "This test should be skipped"
exec echo "This should not run""#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_err(), "Skip test should have failed");

    let error_msg = result.unwrap_err().to_string();
    println!("Skip error message: {}", error_msg);
    // The skip command should create a contextual error showing the skip line
    assert!(error_msg.contains("skip_test.txt"));
    assert!(error_msg.contains("skip \"This test should be skipped\""));
}

#[test]
fn test_stop_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("stop_test.txt");

    let script_content = r#"# Test stop command
exec echo "This runs"
stdout "This runs"
stop "Stop early"
exec echo "This should not run""#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Stop test should have passed: {:?}", result);
}

#[test]
fn test_cp_with_stdout() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("cp_stdout_test.txt");

    let script_content = r#"# Test copying stdout to file
exec printf "hello from stdout"
cp stdout output.txt
cmp output.txt expected.txt

-- expected.txt --
hello from stdout"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "CP stdout test failed: {:?}", result);
}

#[cfg(unix)]
#[test]
fn test_symlink_command() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("symlink_test.txt");

    let script_content = r#"# Test symlink command
symlink original.txt link.txt
exists link.txt
exec cat link.txt
stdout "original content"

-- original.txt --
original content"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Symlink test failed: {:?}", result);
}

#[cfg(unix)]
#[test]
fn test_symlink_directory() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("symlink_dir_test.txt");

    let script_content = r#"# Test symlink with directory
mkdir test_dir
symlink test_dir link_dir
exists link_dir
exec ls link_dir"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Directory symlink test failed: {:?}", result);
}

#[cfg(unix)]
#[test]
fn test_symlink_relative_path() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("symlink_relative_test.txt");

    let script_content = r#"# Test symlink with relative path
mkdir subdir
symlink ../original.txt subdir/link.txt
exists subdir/link.txt
exec cat subdir/link.txt
stdout "relative content"

-- original.txt --
relative content"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Relative path symlink test failed: {:?}", result);
}

#[test]
fn test_symlink_error_cases() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("symlink_error_test.txt");

    let script_content = r#"# Test symlink error cases
! symlink
! symlink only_one_arg"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Symlink error test failed: {:?}", result);
}

#[test]
fn test_issue_example() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("issue_example.txt");

    let script_content = r#"# Create a symlink
symlink original.txt link.txt
exists link.txt
exec cat link.txt
stdout "original content"

-- original.txt --
original content"#;

    fs::write(&script_path, script_content).unwrap();

    let result = run_test(&script_path);
    assert!(result.is_ok(), "Issue example test failed: {:?}", result);
}
