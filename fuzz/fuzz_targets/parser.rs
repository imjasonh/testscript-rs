#![no_main]

use libfuzzer_sys::fuzz_target;
use testscript_rs::parser;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, handling invalid UTF-8 gracefully
    let input = String::from_utf8_lossy(data);
    
    // Fuzz the main parser function
    // The parser should never panic and always return a proper Result
    let result = parser::parse(&input);
    
    // We don't care if parsing succeeds or fails, just that it doesn't panic
    // If it succeeds, verify the result is well-formed
    if let Ok(ref script) = result {
        // Basic sanity checks - these should never fail for valid parsed scripts
        // This helps catch internal consistency bugs
        for command in &script.commands {
            // Command name should not be empty if command exists
            assert!(!command.name.is_empty(), "Empty command name");
            // Line number should be positive
            assert!(command.line_num > 0, "Invalid line number");
        }
        
        for file in &script.files {
            // File name should not be empty if file exists
            assert!(!file.name.is_empty(), "Empty file name");
        }
    }
    
    // Test that the parser is deterministic - same input should produce same result
    let result2 = parser::parse(&input);
    match (result.is_ok(), result2.is_ok()) {
        (true, true) => {
            // Both succeeded - results should be identical
            if let (Ok(ref script1), Ok(ref script2)) = (&result, &result2) {
                assert_eq!(script1, script2, "Parser is not deterministic");
            }
        }
        (false, false) => {
            // Both failed - this is fine, errors don't need to be identical
        }
        _ => {
            panic!("Parser is not deterministic - one call succeeded, other failed");
        }
    }
});
