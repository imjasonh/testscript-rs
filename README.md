# testscript-rs

[![CI](https://github.com/imjasonh/testscript-rs/workflows/CI/badge.svg)](https://github.com/imjasonh/testscript-rs/actions)

A Rust crate for testing command-line tools using filesystem-based script files.

testscript-rs provides a framework for writing integration tests for CLI applications using the `.txtar` format, where test scripts and file contents are combined in a single file.

This crate is inspired by and aims to be compatible with Go's [`github.com/rogpeppe/go-internal/testscript`](https://github.com/rogpeppe/go-internal/tree/master/testscript) package.

Testscript is primarily useful for describing testing scenarios involving executing commands and dealing with files. This makes it a good choice for testing CLI applications in a succinct and human-readable way.

## Quick Example

```rust
use testscript_rs::testscript;

#[test]
fn test_my_cli() {
    testscript::run("testdata")
        .setup(|env| {
            // Compile your CLI tool
            std::process::Command::new("cargo")
                .args(["build", "--bin", "my-cli"])
                .status()?;

            // Copy binary to test environment
            std::fs::copy("target/debug/my-cli", env.work_dir.join("my-cli"))?;
            Ok(())
        })
        .execute()
        .unwrap();
}
```

With a test script in `testdata/basic.txt`:

```
# Test basic functionality
exec ./my-cli --version
stdout "my-cli 1.0"

exec ./my-cli process input.txt
cmp output.txt expected.txt

-- input.txt --
test content

-- expected.txt --
processed: test content
```

Running the test will compile the CLI program, make it available to the testscript environment, run the specified commands, and check its output.

## Installation

Add testscript-rs to your `Cargo.toml`:

```toml
[dev-dependencies]
testscript-rs = "0.1"
```

Requires Rust 1.70 or later.

## Usage

### Basic Usage

```rust
use testscript_rs::testscript;

#[test]
fn test_cli() {
    testscript::run("testdata").execute().unwrap();
}
```

### With Setup Hook

```rust
testscript::run("testdata")
    .setup(|env| {
        // Compile your binary before each test
        std::process::Command::new("cargo")
            .args(["build", "--bin", "my-tool"])
            .status()?;
        Ok(())
    })
    .execute()
    .unwrap();
```

### With Custom Commands

```rust
testscript::run("testdata")
    .command("custom-cmd", |env, args| {
        // Your custom command implementation
        println!("Running custom command with args: {:?}", args);
        Ok(())
    })
    .condition("feature-enabled", true)
    .execute()
    .unwrap();
```

To call the custom command, in your testscript file:

```
custom-cmd arg1 arg2 arg3
```

## Test Script Format

Test scripts use the `.txtar` format. For complete format documentation, see the [original Go testscript documentation]([https://github.com/rogpeppe/go-internal/tree/master/testscript](https://pkg.go.dev/github.com/rogpeppe/go-internal@v1.14.1/testscript#Params)).

### Built-in Commands

- **exec** - Execute external commands
- **cmp** - Compare two files
- **stdout/stderr** - Check command output (supports regex)
- **exists** - Check file existence
- **mkdir** - Create directories
- **cp** - Copy files (supports stdout/stderr as source)
- **mv** - Move/rename files
- **rm** - Remove files/directories
- **chmod** - Change file permissions
- **env** - Set environment variables
- **cmpenv** - Compare files with environment variable substitution
- **stdin** - Set stdin for next command
- **cd** - Change working directory
- **wait** - Wait for background processes
- **kill** - Kill background processes
- **skip** - Skip test execution
- **stop** - Stop test early (pass)
- **unquote** - Remove leading `>` from file lines
- **grep** - Search files with regex

Commands can be prefixed with conditions (`[unix]`) or negated (`!`).

> Note: Some features of `testscript` in Go are not supported in this Rust port:
> 
> - `[gc]` for whether Go was built with gc
> - `[gccgo]` for whether Go was built with gccgo
> - `[go1.x]` for whether the Go version is 1.x or later

## Examples

See [`examples/sample-cli/`](./examples/sample-cli/) and its `testdata` directory for more examples.

There are also more tests in [`testdata`](./testdata/) that demonstrate and check this implementations behavior.
