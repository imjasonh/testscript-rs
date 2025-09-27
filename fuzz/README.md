# Fuzz Testing for testscript-rs

This directory contains fuzz tests for ensuring the parser in testscript-rs is resilient to malformed, malicious, or unexpected input.

## Setup

1. Install cargo-fuzz and switch to nightly Rust:
```bash
cargo install cargo-fuzz
rustup default nightly
```

2. Build all fuzz targets:
```bash
cargo fuzz build
```

## Fuzz Targets

### `parser`
Tests the main `parser::parse()` function with completely random input.

**Focus**: 
- Parser never panics on any input
- Always returns proper Result types
- Deterministic parsing (same input produces same result)
- Basic sanity checks on parsed output

```bash
cargo fuzz run parser
```

### `tokens` 
Tests command token parsing through the public parser interface.

**Focus**:
- Command line token parsing with various quoting
- Condition parsing (`[unix]`, `[!windows]`)
- Negation and background process indicators

```bash
cargo fuzz run tokens
```

### `env_substitution`
Tests environment variable substitution functionality.

**Focus**:
- Variable substitution (`$VAR`, `${VAR}`)
- Edge cases with special characters
- Idempotent behavior
- Escape sequences (`$$`)

```bash
cargo fuzz run env_substitution  
```

### `structured`
Uses `arbitrary` crate to generate structured (but potentially malformed) txtar content.

**Focus**:
- Well-formed but edge-case txtar structures
- Large inputs with many commands/files
- Boundary conditions

```bash
cargo fuzz run structured
```

## Security Testing

The fuzz tests specifically look for:

- **Path traversal attempts**: `../../etc/passwd` in filenames
- **Command injection**: Malicious command patterns
- **Memory safety**: No crashes, infinite loops, or excessive memory usage  
- **Input validation**: Proper error handling for malformed input

## Running Tests

### Quick Test
```bash
# Run for 30 seconds with timeout protection
timeout 30 cargo fuzz run parser -- -timeout=5
```

### Continuous Fuzzing
```bash
# Run until manually stopped
cargo fuzz run parser

# Run with specific options
cargo fuzz run parser -- -runs=10000 -max_len=8192
```

### All Targets
```bash
for target in parser tokens env_substitution structured; do
    echo "Testing $target..."
    timeout 60 cargo fuzz run "$target" -- -timeout=10 -runs=1000
done
```

## Corpus

The `corpus/` directory contains seed inputs that help guide fuzzing:

- `corpus/parser/`: Basic txtar structures and edge cases
- `corpus/tokens/`: Command parsing scenarios
- `corpus/env_substitution/`: Variable substitution patterns

## Artifacts

If crashes are found, they will be stored in `artifacts/` directory. Each crash can be reproduced with:

```bash
cargo fuzz run parser artifacts/parser/crash-<hash>
```

## Success Criteria

- ✅ Parser never panics on any input
- ✅ All errors use proper Error types (no unwrap/panic)
- ✅ No infinite loops or excessive memory usage
- ✅ No path traversal vulnerabilities in file creation
- ✅ Deterministic parsing behavior
- ✅ Proper UTF-8 handling (graceful degradation)

## Integration with CI

For CI environments, run bounded fuzzing:

```bash
# Quick smoke test (suitable for CI)
cargo fuzz run parser -- -runs=1000 -max_total_time=30

# More thorough (nightly CI)  
cargo fuzz run parser -- -runs=100000 -max_total_time=300
```