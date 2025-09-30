use testscript_rs::testscript;
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing workdir_root functionality...");
    
    // Create a custom root directory
    let custom_root = "/tmp/test_workdir_root";
    fs::create_dir_all(custom_root)?;
    
    // Create test data
    let testdata = TempDir::new()?;
    let testdata_path = testdata.path().join("testdata");
    fs::create_dir(&testdata_path)?;
    
    let test_content = r#"exec echo "Testing workdir_root feature"
stdout "Testing workdir_root feature"
"#;
    
    fs::write(testdata_path.join("test.txt"), test_content)?;
    
    // Test with custom workdir root
    println!("Running test with custom workdir root: {}", custom_root);
    match testscript::run(testdata_path.to_string_lossy())
        .workdir_root(custom_root)
        .execute() {
        Ok(()) => println!("✓ Test passed with custom workdir root!"),
        Err(e) => {
            eprintln!("✗ Test failed: {:?}", e);
            return Err(e.into());
        }
    }
    
    println!("All tests passed! The workdir_root feature is working correctly.");
    Ok(())
}