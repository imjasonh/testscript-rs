//! Integration tests

use testscript_rs::testscript;

#[test]
fn test_basic_api() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    let test_content = r#"exec echo hello
stdout hello

-- file.txt --
content"#;

    fs::write(testdata_dir.join("basic.txt"), test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy()).execute();
    assert!(result.is_ok());
}

#[test]
fn test_custom_commands() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let testdata_dir = temp_dir.path().join("testdata");
    fs::create_dir(&testdata_dir).unwrap();

    let test_content = r#"greet Alice
greet Bob
count-args one two three"#;

    fs::write(testdata_dir.join("custom.txt"), test_content).unwrap();

    let result = testscript::run(testdata_dir.to_string_lossy())
        .command("greet", |_env, args| {
            if args.is_empty() {
                return Err(testscript_rs::Error::Generic("greet requires a name".to_string()));
            }
            // Custom command logic would go here
            Ok(())
        })
        .command("count-args", |_env, args| {
            if args.len() != 3 {
                return Err(testscript_rs::Error::Generic(format!("expected 3 args, got {}", args.len())));
            }
            Ok(())
        })
        .execute();

    assert!(result.is_ok());
}
