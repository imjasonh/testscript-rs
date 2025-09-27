//! Manual test for update scripts functionality

use std::fs;
use testscript_rs::testscript;

fn main() {
    println!("Creating test file...");
    fs::write("test_manual.txt", r#"exec echo "Hello, UpdateScripts!"
stdout "Wrong output"
"#).unwrap();
    
    println!("Before update:");
    println!("{}", fs::read_to_string("test_manual.txt").unwrap());
    
    println!("\nRunning with update_scripts enabled...");
    let result = testscript::run(".")
        .update_scripts(true)
        .execute();
    
    match result {
        Ok(()) => {
            println!("Update succeeded!");
            println!("\nAfter update:");
            println!("{}", fs::read_to_string("test_manual.txt").unwrap());
        }
        Err(e) => {
            println!("Update failed: {}", e);
        }
    }
    
    // Clean up
    let _ = fs::remove_file("test_manual.txt");
}