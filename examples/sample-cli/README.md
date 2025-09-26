# Sample CLI Tool Example

This directory demonstrates how to use `testscript-rs` to test a CLI application.

## What's Here

- **`src/main.rs`** - A sample CLI tool with various subcommands
- **`tests/integration_test.rs`** - Integration test using testscript-rs
- **`testdata/`** - Test scripts in `.txtar` format

## The CLI Tool

The sample CLI tool supports:
- `count` - Count lines, words, or characters in files
- `grep` - Search for patterns in files
- `create` - Create files with optional content
- `remove` - Remove files
- `list` - List directory contents

## The Tests

The integration test demonstrates key testscript-rs features:

1. **Setup Hook** - Compiles the CLI binary before running tests
2. **Binary Testing** - Tests the actual compiled binary
3. **File Operations** - Creates files, checks existence, compares content
4. **Output Validation** - Checks stdout/stderr with regex patterns
5. **Error Testing** - Verifies error conditions with negated commands

## Running the Tests

```bash
cargo test
```

This will:
1. Compile the `sample-cli` binary
2. Copy it to each test's temporary directory
3. Run all `.txt` test scripts in `testdata/`
4. Report results

## Example Test Script

```
# Test basic functionality
exec ./sample-cli --version
stdout "sample-cli 1.0"

# Test file operations
exec ./sample-cli create hello.txt --content "Hello, World!"
exists hello.txt
exec ./sample-cli count hello.txt
stdout "hello.txt: 1"

# Test cleanup
exec ./sample-cli remove hello.txt
! exists hello.txt
```

This demonstrates how testscript-rs makes CLI testing both powerful and readable!