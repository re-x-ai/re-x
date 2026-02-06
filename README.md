<p align="center">
  <h1 align="center">re-x 🔍⚡</h1>
  <p align="center">
    <strong>AI-native regex CLI — Test, validate, explain. Built for coding agents.</strong>
  </p>
  <p align="center">
    <a href="https://crates.io/crates/re-x"><img src="https://img.shields.io/crates/v/re-x.svg" alt="crates.io"></a>
    <a href="https://github.com/re-x-ai/re-x/actions"><img src="https://github.com/re-x-ai/re-x/workflows/CI/badge.svg" alt="CI"></a>
    <a href="https://github.com/sponsors/re-x-ai"><img src="https://img.shields.io/github/sponsors/re-x-ai?color=ea4aaa" alt="Sponsors"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  </p>
</p>

---

**re-x** is a regex testing tool designed for AI coding agents. JSON output by default. Single binary. Zero dependencies.

AI agents generate regex patterns constantly — but they can't **test** them. re-x fixes that.

```bash
$ re-x test '(\d{3})-(\d{4})' 'Call 123-4567 now'
{
  "matched": true,
  "match_count": 1,
  "matches": [
    {
      "text": "123-4567",
      "start": 5,
      "end": 13,
      "captures": [
        {"group": 1, "text": "123", "start": 5, "end": 8},
        {"group": 2, "text": "4567", "start": 9, "end": 13}
      ]
    }
  ]
}
```

## Why re-x?

| Problem | Before re-x | With re-x |
|---|---|---|
| AI generates regex | Writes Python script to test → wastes tokens | `re-x test 'pattern' 'input'` → instant JSON |
| "Does this regex work in JS?" | AI guesses | `re-x validate 'pattern'` → portability report |
| Complex regex in legacy code | AI tries to read it | `re-x explain 'pattern'` → structured breakdown |
| ReDoS vulnerability check | No easy way | `re-x benchmark 'pattern'` → backtracking detection |
| regex101.com | Web only, no CLI | re-x = regex101 for your terminal |

## Install

```bash
# Cargo (Rust)
cargo install re-x

# Homebrew (macOS / Linux)
brew install re-x

# Download binary
curl -fsSL https://github.com/re-x-ai/re-x/releases/latest/download/install.sh | sh
```

## Commands

### `re-x test` — Test a pattern

```bash
# Against a string
re-x test '\b\w+@\w+\.\w+\b' 'Email me at user@example.com'

# Against a file (streaming, handles large files)
re-x test 'ERROR|WARN' --file app.log --max-matches 20

# Via pipe
cat data.csv | re-x test '^\d{4}-\d{2}-\d{2},'
```

### `re-x replace` — Preview replacements

```bash
re-x replace '(\w+)@(\w+)' '$1 [at] $2' 'user@example.com'
# → {"result": "user [at] example.com", "replacements_made": 1}

# Dry-run on a file (never modifies the file)
re-x replace 'http://' 'https://' --file urls.txt --dry-run
```

### `re-x validate` — Check syntax & portability

```bash
re-x validate '(?<=\d{3})\w+'
# {
#   "valid": true,
#   "engine_required": "fancy-regex",
#   "portability": {
#     "rust_regex": false,
#     "javascript": false,
#     "python_re": false,
#     "pcre2": true
#   }
# }
```

### `re-x explain` — Break down a pattern

```bash
re-x explain '^(?:https?://)?(?:www\.)?([^/]+)'
# Returns structured JSON with each token explained
```

### `re-x from-examples` — Infer pattern from strings

```bash
re-x from-examples '2024-01-15' '2025-12-31' '2023-06-01'
# → [{"pattern": "\\d{4}-\\d{2}-\\d{2}", "confidence": 0.95}, ...]
```

### `re-x apply` — Apply replacements to a file

```bash
# Dry-run first (shows what would change)
re-x apply 'http://' 'https://' --file urls.txt --dry-run

# Apply with backup (creates urls.txt.bak)
re-x apply 'http://' 'https://' --file urls.txt

# Apply without backup
re-x apply 'http://' 'https://' --file urls.txt --no-backup

# Multiline replacements (cross-line matching)
re-x apply '(?ms)^import.*?;$' 'use crate::*;' --file src/main.rs -m
```

### `re-x benchmark` — Performance & ReDoS detection

```bash
re-x benchmark '(a+)+$' --input 'aaaaaaaaaaab'
# → {"catastrophic_backtracking": true, "warning": "..."}
```

## AI Integration

### Use with Claude Code (MCP)

```bash
# Register as MCP server
claude mcp add re-x -- re-x --mcp
```

Or add to `.mcp.json`:

```json
{
  "mcpServers": {
    "re-x": {
      "type": "stdio",
      "command": "re-x",
      "args": ["--mcp"]
    }
  }
}
```

### Use with Claude Code (bash — zero config)

Claude Code can call re-x directly via bash — just install it and it's available:

```
You: "Write a regex to extract emails from this log file and test it"
Claude Code: runs `re-x test '\b[\w.-]+@[\w.-]+\.\w+\b' --file app.log`
```

### Skills

Copy `skills/regex-testing/SKILL.md` to your project's `.claude/skills/` directory to teach Claude Code when and how to use re-x:

```bash
mkdir -p .claude/skills/regex-testing
cp skills/regex-testing/SKILL.md .claude/skills/regex-testing/
```

For Cursor, copy `skills/cursorrules/.cursor/rules/regex-testing.mdc` to your project's `.cursor/rules/` directory.

## Human-Friendly Mode

While re-x defaults to JSON (for AI), humans can use `--format text`:

```bash
re-x test '\d+' 'abc 123 def 456' --format text

Pattern: \d+
Engine:  regex (linear time)

Match 1: "123" [4..7]
Match 2: "456" [12..15]

2 matches found in 8μs
```

## Design Decisions

**JSON-first**: Every command outputs structured JSON by default. AI agents parse JSON; humans can use `--format text`.

**Dual engine**: Simple patterns use the `regex` crate (linear time guaranteed). Patterns with lookahead/backreferences automatically use `fancy-regex`. You never need to think about it.

**Safe by default**: `re-x replace` previews changes but never modifies files. `re-x apply` writes to files but creates a `.bak` backup by default and supports `--dry-run`.

**Zero dependencies**: Single static binary. No Python, no Node.js, no runtime. Just download and run.

## Performance

Built in Rust with the world-class `regex` crate:

| Operation | Speed |
|---|---|
| Simple pattern, 1MB file | ~3ms |
| Complex pattern, 10MB file | ~35ms |
| Pattern compilation | ~5μs |

## Contributing

Contributions welcome!

```bash
git clone https://github.com/re-x-ai/re-x
cd re-x
cargo test
cargo run -- test '\d+' 'hello 123'
```

## Sponsor

re-x is free and open source (MIT). Sponsoring keeps it maintained:

- 🚀 New features and language support
- 🐛 Quick bug fixes
- 📖 Better docs and AI Skills

[💖 Sponsor on GitHub](https://github.com/sponsors/re-x-ai)

## License

MIT
