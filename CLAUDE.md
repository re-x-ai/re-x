# re-x

AI-native regex CLI tool written in Rust. Dual engine: `regex` (linear time) + `fancy-regex` (backtracking) with automatic selection.

## Build & Test

```bash
cargo build              # Debug build
cargo test               # All tests (unit + integration)
cargo clippy -- -D warnings  # Lint (zero warnings required)
cargo fmt --check        # Format check
```

## Code Conventions

- Edition 2021, standard `rustfmt`
- Error types: `thiserror`; CLI errors: `anyhow`
- Output structs in `src/output/types.rs` with `serde` serialization
- Feature gates: `#[cfg(feature = "cli")]`, `#[cfg(feature = "mcp")]`
- Tests: inline `#[cfg(test)]` modules + `tests/cli_test.rs` (assert_cmd)

## re-x Commands

Use `re-x` to test, validate, and apply regex:

```bash
re-x test 'PAT' 'INPUT'            # Test matching
re-x validate 'PAT'                # Syntax + portability (8 languages)
re-x explain 'PAT'                 # Explain structure
re-x replace 'PAT' 'REPL' 'INPUT'  # Preview replacement
re-x apply 'PAT' 'REPL' --file F   # Apply to file (with backup)
re-x from-examples 'EX1' 'EX2'     # Infer pattern from examples
re-x benchmark 'PAT'               # ReDoS check
re-x --mcp                         # Start MCP server (JSON-RPC over stdio)
```
