# AGENTS.md - re-x Regex Tool

## Tool: re-x

AI-native regex CLI tool for testing, validating, and explaining regular expressions.

## When to Use

- After generating any regex pattern, use `re-x test` to verify it works
- Before committing regex changes, use `re-x validate` to check syntax and portability
- When encountering complex regex in code, use `re-x explain` to understand it
- For security-sensitive patterns, use `re-x benchmark` to detect ReDoS vulnerabilities

## Commands

### re-x test
Test a pattern against input and get structured JSON output.

```bash
re-x test 'PATTERN' 'INPUT'
re-x test 'PATTERN' --file path/to/file.txt
```

### re-x validate
Check pattern syntax and cross-language compatibility.

```bash
re-x validate 'PATTERN'
re-x validate 'PATTERN' --target-lang javascript
```

### re-x replace
Preview regex replacement without modifying files.

```bash
re-x replace 'PATTERN' 'REPLACEMENT' 'INPUT'
```

### re-x explain
Get a structured breakdown of pattern components.

```bash
re-x explain 'PATTERN'
```

### re-x from-examples
Infer regex patterns from example strings.

```bash
re-x from-examples 'example1' 'example2' 'example3'
```

### re-x apply
Apply regex replacement to a file with backup.

```bash
re-x apply 'PATTERN' 'REPLACEMENT' --file path/to/file.txt
re-x apply 'PATTERN' 'REPLACEMENT' --file path/to/file.txt --dry-run
re-x apply 'PATTERN' 'REPLACEMENT' --file path/to/file.txt --no-backup
re-x apply 'PATTERN' 'REPLACEMENT' --file path/to/file.txt -m  # multiline
```

### re-x benchmark
Measure performance and detect catastrophic backtracking.

```bash
re-x benchmark 'PATTERN' --input 'test input'
```

## Output Format

All commands return JSON by default. Key fields:

- `matched`: boolean - whether any match was found
- `match_count`: number of matches
- `matches[]`: array of match objects with `text`, `start`, `end`, `captures`
- `valid`: boolean - whether pattern is syntactically valid
- `portability`: object showing language compatibility
- `applied`: boolean - whether file was modified (apply command)
- `backup_path`: string - path to backup file (apply command)

## Recommended Workflow

```
Generate → Validate → Test → Fix → Check Portability → Commit
```

Always validate and test before using a regex in code.
