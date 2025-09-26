//! Example of how to use testscript-rs to run test scripts

use testscript_rs::testscript;

fn main() {
    // Change to the project root directory for the example
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        std::env::set_current_dir(manifest_dir).expect("Failed to change directory");
    }

    // Run all tests in testdata directory
    match testscript::run("testdata").execute() {
        Ok(()) => println!("All tests passed!"),
        Err(e) => {
            eprintln!("Test failed: {}", e);
            std::process::exit(1);
        }
    }
}