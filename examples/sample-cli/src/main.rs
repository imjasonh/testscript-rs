//! Sample CLI tool to demonstrate testscript-rs usage
//!
//! This is a simple file manipulation tool that supports:
//! - Counting lines, words, and characters in files
//! - Searching for patterns in files
//! - Creating and removing files
//! - Directory operations

use clap::{Parser, Subcommand};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Parser)]
#[command(name = "sample-cli")]
#[command(about = "A sample CLI tool for demonstrating testscript-rs")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Count lines, words, or characters in files
    Count {
        /// What to count: lines, words, chars
        #[arg(short, long, default_value = "lines")]
        mode: CountMode,
        /// Files to process
        files: Vec<String>,
    },
    /// Search for a pattern in files
    Grep {
        /// Pattern to search for
        pattern: String,
        /// Files to search in
        files: Vec<String>,
        /// Case insensitive search
        #[arg(short, long)]
        ignore_case: bool,
    },
    /// Create a new file with optional content
    Create {
        /// File to create
        file: String,
        /// Content to write (optional)
        #[arg(short, long)]
        content: Option<String>,
    },
    /// Remove files
    Remove {
        /// Files to remove
        files: Vec<String>,
    },
    /// List directory contents
    List {
        /// Directory to list (default: current)
        #[arg(default_value = ".")]
        dir: String,
    },
}

#[derive(Clone)]
enum CountMode {
    Lines,
    Words,
    Chars,
}

impl std::str::FromStr for CountMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lines" | "line" | "l" => Ok(CountMode::Lines),
            "words" | "word" | "w" => Ok(CountMode::Words),
            "chars" | "char" | "c" => Ok(CountMode::Chars),
            _ => Err(format!(
                "Invalid count mode: {}. Use lines, words, or chars",
                s
            )),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Count { mode, files }) => count_command(mode, files),
        Some(Commands::Grep {
            pattern,
            files,
            ignore_case,
        }) => grep_command(pattern, files, ignore_case),
        Some(Commands::Create { file, content }) => create_command(file, content),
        Some(Commands::Remove { files }) => remove_command(files),
        Some(Commands::List { dir }) => list_command(dir),
        None => {
            println!("sample-cli - A demonstration CLI tool");
            println!("Use --help for usage information");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn count_command(mode: CountMode, files: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if files.is_empty() {
        return Err("No files specified".into());
    }

    for file_path in files {
        let content = fs::read_to_string(&file_path)?;

        let count = match mode {
            CountMode::Lines => content.lines().count(),
            CountMode::Words => content.split_whitespace().count(),
            CountMode::Chars => content.chars().count(),
        };

        println!("{}: {}", file_path, count);
    }

    Ok(())
}

fn grep_command(
    pattern: String,
    files: Vec<String>,
    ignore_case: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if files.is_empty() {
        return Err("No files specified".into());
    }

    let search_pattern = if ignore_case {
        pattern.to_lowercase()
    } else {
        pattern
    };

    for file_path in files {
        let file = fs::File::open(&file_path)?;
        let reader = BufReader::new(file);

        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            let search_line = if ignore_case {
                line.to_lowercase()
            } else {
                line.clone()
            };

            if search_line.contains(&search_pattern) {
                println!("{}:{}: {}", file_path, line_num + 1, line);
            }
        }
    }

    Ok(())
}

fn create_command(file: String, content: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let content = content.unwrap_or_else(|| String::new());
    fs::write(&file, content)?;
    println!("Created file: {}", file);
    Ok(())
}

fn remove_command(files: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if files.is_empty() {
        return Err("No files specified".into());
    }

    for file_path in files {
        if Path::new(&file_path).exists() {
            fs::remove_file(&file_path)?;
            println!("Removed: {}", file_path);
        } else {
            eprintln!("File not found: {}", file_path);
        }
    }

    Ok(())
}

fn list_command(dir: String) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_type = if entry.file_type()?.is_dir() {
            "DIR"
        } else {
            "FILE"
        };
        println!("{}\t{}", file_type, file_name.to_string_lossy());
    }

    Ok(())
}
