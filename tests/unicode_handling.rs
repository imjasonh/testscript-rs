//! Tests for Unicode handling in stdout/stderr pattern matching

use std::fs;
use tempfile::TempDir;
use testscript_rs::testscript;

#[test]
fn test_unicode_stdout_exact_match() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test with Unicode checkmark
    let test_content = r#"exec echo "Success ✓"
stdout "Success ✓"
"#;

    let test_file = testdata_dir.join("unicode_stdout_test.txt");
    fs::write(&test_file, test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    
    assert!(
        result.is_ok(),
        "Unicode stdout exact match should work: {:?}",
        result
    );
}

#[test]
fn test_unicode_stderr_exact_match() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test with Unicode checkmark in stderr
    let test_content = r#"exec sh -c 'echo "Error ✗" >&2'
stderr "Error ✗"
"#;

    let test_file = testdata_dir.join("unicode_stderr_test.txt");
    fs::write(&test_file, test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    
    assert!(
        result.is_ok(),
        "Unicode stderr exact match should work: {:?}",
        result
    );
}

#[test]
fn test_unicode_regex_match() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test with Unicode in regex pattern
    let test_content = r#"exec echo "Success ✓ completed"
stdout "Success .* completed"
"#;

    let test_file = testdata_dir.join("unicode_regex_test.txt");
    fs::write(&test_file, test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    
    assert!(
        result.is_ok(),
        "Unicode in regex pattern should work: {:?}",
        result
    );
}

#[test]
fn test_multiple_unicode_chars() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test with multiple Unicode characters
    let test_content = r#"exec echo "Tests: ✓ ✗ ⚠ ℹ"
stdout "Tests: ✓ ✗ ⚠ ℹ"
"#;

    let test_file = testdata_dir.join("multi_unicode_test.txt");
    fs::write(&test_file, test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    
    assert!(
        result.is_ok(),
        "Multiple Unicode characters should work: {:?}",
        result
    );
}

#[test]
fn test_emoji_in_output() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test with emoji
    let test_content = r#"exec echo "Status: 🎉 Success!"
stdout "Status: 🎉 Success!"
"#;

    let test_file = testdata_dir.join("emoji_test.txt");
    fs::write(&test_file, test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    
    assert!(
        result.is_ok(),
        "Emoji in output should work: {:?}",
        result
    );
}

#[test]
fn test_unicode_with_special_chars() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test Unicode with characters that trigger regex mode
    let test_content = r#"exec echo "Result: ✓ (success)"
stdout "Result: ✓ \(success\)"
"#;

    let test_file = testdata_dir.join("unicode_special_test.txt");
    fs::write(&test_file, test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    
    assert!(
        result.is_ok(),
        "Unicode with regex special chars should work: {:?}",
        result
    );
}

#[test]
fn test_dot_matches_unicode_in_regex() {
    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    // Test if . in regex matches Unicode characters
    let test_content = r#"exec echo "A✓B"
stdout "A.B"
"#;

    let test_file = testdata_dir.join("dot_unicode_test.txt");
    fs::write(&test_file, test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    
    assert!(
        result.is_ok(),
        "Dot in regex should match Unicode character: {:?}",
        result
    );
}
