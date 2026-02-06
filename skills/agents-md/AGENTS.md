# AGENTS.md

## Project overview

re-x is an AI-native regex CLI tool written in Rust. It provides structured JSON output for testing, validating, explaining, benchmarking, and applying regular expressions. Designed for integration with coding agents and CI pipelines.

- **Language**: Rust (edition 2021)
- **Dual regex engine**: `regex` crate (linear time) + `fancy-regex` (backtracking with lookahead/lookbehind)
- **Engine auto-selection**: Patterns with advanced features (lookahead, backreference, etc.) automatically use fancy-regex
- **Output**: JSON by default, human-readable text with `--format text`

## Build and test commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests (unit + integration)
cargo clippy -- -D warnings  # Lint (zero warnings required)
cargo fmt                    # Format
cargo bench              # Run benchmarks
```

## Commands

```bash
re-x test 'PATTERN' 'INPUT'                         # Test matching
re-x test 'PATTERN' --file FILE                      # Test against file
re-x replace 'PATTERN' 'REPLACEMENT' 'INPUT'         # Preview replacement
re-x validate 'PATTERN'                              # Check syntax + portability
re-x validate 'PATTERN' --target-lang javascript     # Check language compatibility
re-x explain 'PATTERN'                               # Explain pattern structure
re-x from-examples 'EX1' 'EX2' 'EX3'                # Infer pattern from examples
re-x apply 'PAT' 'REPL' --file FILE                  # Apply replacement to file
re-x apply 'PAT' 'REPL' --file FILE --dry-run        # Preview without writing
re-x benchmark 'PATTERN'                             # Performance + ReDoS check
```

## Code style

- Follow standard `rustfmt` formatting
- Use `thiserror` for error types, `anyhow` for CLI error handling
- All public functions return structured types from `src/output/types.rs`
- Feature-gate CLI with `#[cfg(feature = "cli")]` and MCP with `#[cfg(feature = "mcp")]`

## Testing

- Unit tests: inline `#[cfg(test)]` modules in each source file
- Integration tests: `tests/cli_test.rs` using `assert_cmd` + `predicates`
- Temp files for apply/replace tests: use `tempfile` crate
- All tests must pass with `cargo test`
- CI runs `cargo clippy -- -D warnings` (zero warnings required)

## Project structure

```
src/
  main.rs          # Entry point, CLI dispatch
  cli.rs           # Command handlers
  mcp.rs           # MCP (Model Context Protocol) server
  core/
    engine.rs      # Dual regex engine (regex + fancy-regex)
    benchmark.rs   # Performance measurement + ReDoS detection
    explain.rs     # Pattern explanation
    portability.rs # Cross-language compatibility (AST-based)
    replace.rs     # Replacement logic + file apply
    validate.rs    # Syntax validation
    test.rs        # Match testing
    from_examples.rs # Pattern inference
    templates.rs   # Common pattern templates
  output/
    types.rs       # Output data structures (serde)
    json.rs        # JSON formatter
    text.rs        # Human-readable formatter
tests/
  cli_test.rs      # End-to-end CLI tests
benches/
  engine_bench.rs  # Criterion benchmarks
```

## Regex workflow

When generating or modifying regex in this project or any project using re-x:

1. `re-x validate 'pattern'` -- check syntax
2. `re-x test 'pattern' 'test data'` -- verify behavior
3. If issues, fix and return to step 1
4. `re-x validate 'pattern' --target-lang <lang>` -- check portability
5. `re-x apply 'pat' 'repl' --file path --dry-run` -- preview file changes
6. Commit
