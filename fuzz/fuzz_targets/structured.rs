#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use testscript_rs::parser;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    commands: Vec<FuzzCommand>,
    files: Vec<FuzzFile>,
}

#[derive(Arbitrary, Debug)]
struct FuzzCommand {
    condition: Option<String>,
    negated: bool,
    background: bool,
    name: String,
    args: Vec<String>,
}

#[derive(Arbitrary, Debug)]
struct FuzzFile {
    name: String,
    contents: Vec<u8>,
}

impl FuzzInput {
    fn to_txtar(&self) -> String {
        let mut result = String::new();
        
        // Add commands
        for cmd in &self.commands {
            // Add condition prefix
            if let Some(ref condition) = cmd.condition {
                result.push_str(&format!("[{}] ", condition));
            }
            
            // Add negation
            if cmd.negated {
                result.push_str("! ");
            }
            
            // Add command name and args
            result.push_str(&cmd.name);
            for arg in &cmd.args {
                // Simple quoting - add quotes if arg contains spaces
                if arg.contains(' ') || arg.contains('\t') {
                    result.push_str(&format!(" \"{}\"", arg.replace('"', "\\\"")));
                } else {
                    result.push_str(&format!(" {}", arg));
                }
            }
            
            // Add background indicator
            if cmd.background {
                result.push_str(" &");
            }
            
            result.push('\n');
        }
        
        // Add files
        for file in &self.files {
            result.push_str(&format!("-- {} --\n", file.name));
            // Convert bytes to string, replacing invalid UTF-8
            let content = String::from_utf8_lossy(&file.contents);
            result.push_str(&content);
            if !content.ends_with('\n') {
                result.push('\n');
            }
        }
        
        result
    }
}

fuzz_target!(|data: &[u8]| {
    // Create structured input using arbitrary
    let mut unstructured = Unstructured::new(data);
    if let Ok(fuzz_input) = FuzzInput::arbitrary(&mut unstructured) {
        // Convert to txtar format
        let txtar_content = fuzz_input.to_txtar();
        
        // Test the parser
        let result = parser::parse(&txtar_content);
        
        // Verify parser doesn't panic and produces consistent results
        if let Ok(script) = result {
            // Validate the parsed structure
            // Note: parsed commands may be fewer than input due to empty lines, comments, etc.
            assert!(script.commands.len() <= fuzz_input.commands.len().max(1000), "Too many commands parsed");
            
            for command in &script.commands {
                assert!(!command.name.is_empty(), "Empty command name");
                assert!(command.line_num > 0, "Invalid line number");
            }
            
            for file in &script.files {
                assert!(!file.name.is_empty(), "Empty file name");
            }
        }
    }
    
    // Also test with raw bytes as backup for edge cases
    let raw_input = String::from_utf8_lossy(data);
    let _result = parser::parse(&raw_input);
});