# Contributing to re-x

## Quick Start

```bash
git clone https://github.com/re-x-ai/re-x.git
cd re-x
cargo build
cargo test
```

## Development Workflow

1. Create a branch from `main`
2. Make your changes
3. Run all checks:

```bash
cargo fmt           # Format code
cargo clippy -- -D warnings  # Lint (must be zero warnings)
cargo test          # Run all tests
```

4. Open a pull request against `main`

## Code Structure

```
src/
  main.rs          # Entry point, CLI dispatch
  cli.rs           # Command handlers
  mcp.rs           # MCP server (JSON-RPC over stdio)
  core/            # Business logic (engine, validate, explain, etc.)
  output/          # Output formatting (JSON, text, types)
tests/
  cli_test.rs      # End-to-end CLI tests
benches/
  engine_bench.rs  # Criterion benchmarks
```

## Adding a New Command

1. Add the CLI argument to `Commands` enum in `src/cli.rs`
2. Implement the core logic in a new file under `src/core/`
3. Add the output type to `src/output/types.rs`
4. Add JSON formatter in `src/output/json.rs` and text formatter in `src/output/text.rs`
5. Wire up the command handler in `src/cli.rs` and dispatch in `src/main.rs`
6. Add MCP tool definition in `src/mcp.rs`
7. Add unit tests in the core module and E2E tests in `tests/cli_test.rs`
8. Update README.md, CLAUDE.md, AGENTS.md, and skill files

## Testing Guidelines

- **Unit tests**: Add `#[cfg(test)]` module in the same source file
- **E2E tests**: Add to `tests/cli_test.rs` using `assert_cmd` + `predicates`
- **Temp files**: Use `tempfile` crate for file-based tests
- All tests must pass before merging

## Commit Messages

Use concise, descriptive messages:

```
Add explain command for pattern breakdown
Fix false positive in portability lookahead detection
Update README with apply command documentation
```

## CI Requirements

All PRs must pass:
- Build on Linux, macOS, Windows (stable + beta Rust)
- `cargo clippy -- -D warnings`
- `cargo fmt -- --check`
- `cargo doc` with `-D warnings`

## Release Process

Releases are automated via GitHub Actions. Push a version tag to trigger:

```bash
git tag v0.2.0
git push origin v0.2.0
```

This builds binaries for all platforms, creates a GitHub Release, and publishes to crates.io.
