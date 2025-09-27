//! Example of how to use testscript-rs to run test scripts

use testscript_rs::testscript;

fn main() {
    // Change to the project root directory for the example
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        std::env::set_current_dir(manifest_dir).expect("Failed to change directory");
    }

    // Check for test directory from environment for test purposes
    let test_dir = std::env::var("TESTSCRIPT_TEST_DIR").unwrap_or_else(|_| "testdata".to_string());
    
    // Check if preserve work should be enabled
    let preserve_work = std::env::var("TESTSCRIPT_PRESERVE_WORK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    // Run all tests in the specified directory
    let mut builder = testscript::run(test_dir);
    
    if preserve_work {
        builder = builder.preserve_work_on_failure(true);
    }

    match builder.execute() {
        Ok(()) => println!("All tests passed!"),
        Err(e) => {
            eprintln!("Test failed: {}", e);
            std::process::exit(1);
        }
    }
}
