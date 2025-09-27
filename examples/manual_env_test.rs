//! Manual test for environment variable functionality

use std::fs;
use testscript_rs::testscript;

fn main() {
    println!("Creating test file...");
    fs::write("test_env.txt", r#"exec echo "Environment variable test!"
stdout "Wrong output again"
"#).unwrap();
    
    println!("Before update:");
    println!("{}", fs::read_to_string("test_env.txt").unwrap());
    
    println!("\nRunning with UPDATE_SCRIPTS environment variable...");
    let result = testscript::run(".").execute();
    
    match result {
        Ok(()) => {
            println!("Update succeeded!");
            println!("\nAfter update:");
            println!("{}", fs::read_to_string("test_env.txt").unwrap());
        }
        Err(e) => {
            println!("Update failed: {}", e);
        }
    }
    
    // Clean up
    let _ = fs::remove_file("test_env.txt");
}