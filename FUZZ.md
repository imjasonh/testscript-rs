# Running Fuzz Tests

This document describes how to run fuzz tests for testscript-rs to ensure parser resilience.

## Quick Start

```bash
# Install fuzzing tools
cargo install cargo-fuzz

# Switch to nightly Rust (required for fuzzing)
rustup default nightly

# Run parser fuzzing for 30 seconds
timeout 30 cargo fuzz run parser

# Run all fuzz targets with time limit
for target in parser tokens env_substitution structured; do
    echo "Fuzzing $target..."
    timeout 60 cargo fuzz run "$target" -- -timeout=10 -runs=10000
done
```

## What Gets Tested

The fuzz tests target potential security and stability issues:

### Security Concerns
- **Path traversal**: `../../etc/passwd` in filenames
- **Command injection**: Malicious command patterns  
- **Memory safety**: No crashes, infinite loops, excessive memory usage
- **Input validation**: Proper error handling for malformed input

### Parser Resilience  
- **Never panics**: All inputs should return proper Result types
- **Deterministic**: Same input produces same output
- **UTF-8 handling**: Graceful degradation for invalid UTF-8
- **Edge cases**: Empty files, malformed headers, unclosed quotes

## Fuzz Targets

| Target | Purpose |
|--------|---------|
| `parser` | Main parser with random input |
| `tokens` | Command token parsing edge cases |
| `env_substitution` | Environment variable handling |  
| `structured` | Well-formed but boundary-case inputs |

## Example Output

```bash
$ cargo fuzz run parser -- -runs=1000
INFO: Loaded 1 modules   (1968 inline 8-bit counters)
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
#1000	DONE   cov: 249 ft: 413 corp: 23/70b lim: 4 exec/s: 0 rss: 44Mb
Done 1000 runs in 1 second(s)
```

No crashes = success! 🎉

## Switch Back to Stable

```bash
# Return to stable Rust for normal development
rustup default stable
```

See `fuzz/README.md` for detailed documentation.