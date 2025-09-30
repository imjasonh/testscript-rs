---
description: Instructions for GitHub Copilot when working with testscript-rs, a Rust crate for testing CLI tools using filesystem-based script files
globs: "**/*.rs,**/*.toml,**/*.md,testdata/**/*.txt"
---

# GitHub Copilot Instructions for testscript-rs

## Project Overview

testscript-rs is a Rust crate for testing CLI tools using filesystem-based script files in the `.txtar` format. It mirrors the functionality of Go's `github.com/rogpeppe/go-internal/testscript` package, providing a powerful testing framework for command-line applications.

## Development Workflow

### Common Commands

Always use these commands for development and testing:

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test integration
cargo test --test builtin_commands
cargo test parser::tests::

# Run a single test
cargo test test_basic_api

# Format code (always run before committing)
cargo fmt

# Check code formatting
cargo fmt --check

# Check code quality
cargo clippy --all-targets -- -D warnings

# Build and run examples
cargo run --example test_runner

# Test the sample CLI example
cd examples/sample-cli && cargo test
```

### Testing Strategy

- **Unit tests**: Located in individual modules with `#[cfg(test)]`
- **Integration tests**: In `tests/` directory
- **Compatibility tests**: Use files from `testdata/` (Go testscript format)
- **Examples**: In `examples/` directory with their own test suites

## Architecture and Code Organization

### Core Modules (in `src/`)

**`parser.rs`** - Parses `.txtar` format files:
- `Script` struct: Contains commands and files from test scripts
- `Command` struct: Individual command lines with args, conditions, negation
- `TxtarFile` struct: File blocks embedded in scripts
- Handles quoted arguments, escape sequences, conditions (`[unix]`), negation (`!`), background processes (`&`)

**`run.rs`** - Execution environment and command dispatch:
- `TestEnvironment`: Manages isolated temporary directories
- `RunParams`: Configuration for the test runner (internal)
- Built-in command implementations
- Custom command dispatch system
- Background process management

**`error.rs`** - Error types using `thiserror` for structured error handling

### Public API (`src/testscript.rs`)

The main public API uses a builder pattern:

```rust
testscript::run("testdata")
    .setup(|env| { /* compile CLI tool */ })
    .command("custom", |env, args| { /* custom logic */ })
    .condition("feature", true)
    .execute()
```

## Coding Guidelines

### Rust Style
- Follow standard Rust formatting (`cargo fmt`)
- Use `clippy` suggestions (`cargo clippy`)
- Prefer `Result<T, Error>` over panicking
- Use `thiserror` for error types
- Implement `Debug` for all public types

### Error Handling
- Use the custom `Error` enum defined in `error.rs`
- Propagate errors with `?` operator
- Provide meaningful error messages with context
- Never use `unwrap()` or `panic!()` in production code

### Testing Patterns
- Use descriptive test names: `test_basic_functionality`
- Create isolated test environments with `TestEnvironment::new()`
- Test both success and failure cases
- Use `assert_eq!` and `assert!` with descriptive messages

### File Structure
- Parser logic stays in `parser.rs`
- Execution logic stays in `run/` module
- Tests in appropriate `tests/` files
- Examples in `examples/` with their own `Cargo.toml`

## Built-in Commands

When working with built-in commands, understand these core commands:

**Essential**: `exec`, `cmp`, `stdout`, `stderr`, `exists`, `mkdir`, `cp`, `mv`, `rm`
**Advanced**: `chmod`, `env`, `cmpenv`, `stdin`, `cd`, `wait`, `kill`, `grep`, `symlink`
**Control**: `skip`, `stop`, `unquote`

### Command Dispatch Order
1. Custom commands (registered with `.command()`)
2. Built-in commands (implemented in `TestEnvironment`)
3. Error if command not found

## Test Script Format (`.txtar`)

### Command Syntax
```
# Comment
[condition] [!] command arg1 arg2 [&]
```

- `[condition]`: Optional condition like `[unix]` or `[windows]`
- `[!]`: Optional negation prefix
- `command`: Built-in or custom command name
- `arg1 arg2`: Arguments (support quoted strings)
- `[&]`: Optional background execution

### File Blocks
```
-- filename --
file content
```

### Environment Variables
- `$WORK`: Test working directory
- Custom variables set with `env` command
- Substitution in `cmpenv` and output patterns

## Common Patterns

### CLI Tool Testing
```rust
testscript::run("testdata")
    .setup(|env| {
        // Compile the CLI tool
        std::process::Command::new("cargo")
            .args(["build", "--bin", "my-tool"])
            .status()?;
        
        // Copy to test environment
        std::fs::copy("target/debug/my-tool", env.work_dir.join("my-tool"))?;
        Ok(())
    })
    .execute()
```

### Custom Commands
```rust
testscript::run("testdata")
    .command("mytool", |env, args| {
        // Custom command implementation
        env.run_command("my-binary", args)
    })
    .execute()
```

### Background Processes
```
# Start background process
exec my-server --port 8080 &

# Do some work
exec curl http://localhost:8080/api/test

# Wait for completion
wait my-server
```

## Security Considerations

### Fuzz Testing
- Located in `fuzz/` directory
- Targets: `parser`, `tokens`, `env_substitution`, `structured`
- Run with: `cargo fuzz run <target>`
- Security focus: path traversal, command injection, memory safety

### Safe Patterns
- Always validate file paths (no `../` traversal)
- Sanitize command arguments
- Use temporary directories for test isolation
- Never execute user input directly

## Common Issues and Solutions

### Test Failures
- Check working directory with `$WORK` variable
- Verify file permissions (especially on Unix)
- Ensure proper environment variable setup
- Use `cargo test -- --nocapture` for debugging

### Parser Issues
- Validate `.txtar` format (proper `-- filename --` headers)
- Check for proper command syntax
- Handle edge cases with empty lines and comments

### Cross-platform Compatibility
- Use conditions: `[unix]`, `[windows]`
- Handle path separators correctly
- Test file permissions on different platforms

## Contributing Guidelines

### Before Submitting
1. Run `cargo fmt` and `cargo clippy`
2. Ensure all tests pass: `cargo test`
3. Add tests for new functionality
4. Update documentation for API changes
5. Test examples still work

### Adding Built-in Commands
1. Add method to `TestEnvironment` in `run/environment.rs`
2. Add to command dispatch in `run/mod.rs`
3. Add tests in `tests/builtin_commands.rs`
4. Update documentation

### Adding Examples
1. Create new directory in `examples/`
2. Include `Cargo.toml` and test suite
3. Add to CI workflows if needed
4. Document in example's `README.md`

## Resources

- [Go testscript documentation](https://pkg.go.dev/github.com/rogpeppe/go-internal/testscript) (reference implementation)
- [txtar format specification](https://pkg.go.dev/github.com/rogpeppe/go-internal/txtar)
- Project documentation in `README.md`, `CLAUDE.md`, `FUZZ.md`
- Examples in `examples/` directory

When generating code, prioritize correctness, safety, and compatibility with the existing codebase. Always consider cross-platform implications and follow Rust best practices.