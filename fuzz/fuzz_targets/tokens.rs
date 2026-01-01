#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, handling invalid UTF-8 gracefully
    let input = String::from_utf8_lossy(data);

    // We need to test the token parsing function, but it's private
    // So we test it indirectly through the public parser interface
    // Create minimal txtar content with the fuzzed command line
    let txtar_content = format!("{}\n", input);

    // Fuzz through the main parser - this will exercise parse_command_tokens
    let result = testscript_rs::parser::parse(&txtar_content);

    // The parser should never panic
    // If parsing succeeds, verify basic properties
    if let Ok(script) = result {
        for command in &script.commands {
            // If we have a command, it should have at least a name
            assert!(!command.name.is_empty(), "Command with empty name");
            // Arguments should be valid strings (no invalid UTF-8)
            for arg in &command.args {
                // Just accessing the string should not panic
                let _len = arg.len();
            }
        }
    }

    // Test with various command prefixes to exercise condition parsing
    if !input.trim().is_empty() && !input.starts_with('#') {
        let prefixes = ["", "[unix] ", "[!windows] ", "! "];
        for prefix in &prefixes {
            let prefixed_content = format!("{}{}\n", prefix, input);
            let _result = testscript_rs::parser::parse(&prefixed_content);
            // Should not panic regardless of result
        }
    }
});
