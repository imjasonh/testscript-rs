# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

testscript-rs is a Rust crate for testing CLI tools using filesystem-based script files in the `.txtar` format. It mirrors the functionality of Go's `github.com/rogpeppe/go-internal/testscript` package.

## Development Commands

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration
cargo test --test builtin_commands
cargo test parser::tests::

# Run single test
cargo test test_basic_api

# Check code quality
cargo clippy --all-targets -- -D warnings
cargo check

# Build and run examples
cargo run --example test_runner

# Test the sample CLI example
cd examples/sample-cli && cargo test
```

## Architecture

The crate is organized into three main modules that work together:

### Core Modules

**`parser.rs`** - Parses `.txtar` format files into structured representations:
- `Script` struct contains commands and files from a test script
- `Command` struct represents individual command lines with args, conditions, negation
- `TxtarFile` represents file blocks embedded in the script
- Handles quoted arguments, escape sequences, conditions (`[unix]`), negation (`!`), and background processes (`&`)

**`run.rs`** - Execution environment and command dispatch:
- `TestEnvironment` manages isolated temporary directories for each test
- `RunParams` configures the test runner (internal, used by Builder)
- Built-in command implementations (exec, cmp, stdout, stderr, exists, mkdir, cp, mv, rm, chmod, env, cmpenv, stdin, etc.)
- Custom command dispatch system
- Background process management

**`error.rs`** - Error types using `thiserror` for structured error handling

### Public API

The main public API is the `testscript` module with a builder pattern:

```rust
testscript::run("testdata")
    .setup(|env| { /* compile CLI tool */ })
    .command("custom", |env, args| { /* custom logic */ })
    .condition("feature", true)
    .execute()
```

## Key Implementation Details

**Script Parsing**: The parser handles complex `.txtar` format with file blocks, command parsing with proper quoting, and conditional execution markers.

**Test Isolation**: Each test script runs in a completely isolated temporary directory with its own environment variables and file system.

**Command System**: Built-in commands are implemented as methods on `TestEnvironment`. Custom commands are function pointers stored in a HashMap and checked before built-ins during dispatch.

**Background Processes**: Commands ending with `&` spawn processes stored in a HashMap by name, managed with `wait` and `kill` commands.

**Environment Variables**: Full support including `$WORK` (working directory), custom variables, and substitution in file comparisons (`cmpenv`) and output patterns.

## Test Structure

- `tests/integration.rs` - Main API tests including custom command usage
- `tests/builtin_commands.rs` - Tests for all built-in commands
- `tests/edge_cases.rs` - Parser edge cases and error conditions
- `tests/integration_tests.rs` - Original integration tests
- `tests/setup_hook.rs` - Setup functionality tests
- `testdata/` - Go testscript compatibility test files (40 files from upstream)

## Built-in Commands

Core: `exec`, `cmp`, `stdout`, `stderr`, `cd`, `wait`, `exists`, `mkdir`, `cp`, `mv`, `rm`, `chmod`, `env`, `cmpenv`, `stdin`, `skip`, `stop`, `kill`, `unquote`, `grep`

The command dispatch checks custom commands first, then falls back to built-ins, enabling easy extension.
