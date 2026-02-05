# Regex Testing & Validation with re-x

## When to Use

Use `re-x` in these situations:

1. **After generating a regex** — verify it matches expected input before writing it into code
2. **Before committing regex changes** — validate syntax and cross-language portability
3. **When debugging regex** — test against real data to see what matches (and what doesn't)
4. **When reading complex regex** — get a structured explanation of each component
5. **Before deploying user-facing regex** — check for ReDoS vulnerabilities
6. **When building regex from scratch** — generate candidates from example strings

## Recommended Workflow

Always follow this cycle when working with regex:

```
Generate pattern → validate → test → (fix if needed) → check portability → commit
```

### Step 1: Validate syntax
```bash
re-x validate 'your-pattern-here'
```
Check the `valid` field. If false, read `error.suggestion` for the fix.

### Step 2: Test against data
```bash
# Against a string
re-x test 'pattern' 'sample input text'

# Against a file (streaming, handles large files)
re-x test 'pattern' --file path/to/data.txt

# Limit results for large files
re-x test 'pattern' --file big.log --max-matches 10
```
Check `match_count` and `matches[].text` to verify correctness.

### Step 3: Check portability (if targeting a specific language)
```bash
re-x validate 'pattern'
```
Read `portability` object to confirm the pattern works in the target language.

### Step 4: Test replacements (if applicable)
```bash
re-x replace 'pattern' 'replacement' 'sample input'
```
Verify `result` matches expected output.

### Step 5: Apply to file (if needed)
```bash
# Dry-run first to preview changes
re-x apply 'pattern' 'replacement' --file path/to/file.txt --dry-run

# Apply with backup (creates .bak file)
re-x apply 'pattern' 'replacement' --file path/to/file.txt

# Multiline mode for cross-line matches
re-x apply 'pattern' 'replacement' --file path/to/file.txt -m
```
Check `applied` field to confirm changes were written.

## Command Reference

| Command | Purpose | Key Output Fields |
|---|---|---|
| `re-x test PATTERN INPUT` | Test matching | `matched`, `match_count`, `matches[]` |
| `re-x test PATTERN --file FILE` | Test on file | Same, streaming for large files |
| `re-x replace PATTERN REPL INPUT` | Preview replacement | `result`, `replacements_made` |
| `re-x validate PATTERN` | Syntax + portability | `valid`, `portability{}`, `engine_required` |
| `re-x explain PATTERN` | Structured breakdown | `parts[]` with `token`, `type`, `desc` |
| `re-x from-examples EX1 EX2...` | Infer pattern | `inferred[]` with `pattern`, `confidence` |
| `re-x apply PATTERN REPL --file FILE` | Apply replacement to file | `applied`, `replacements_made`, `backup_path` |
| `re-x benchmark PATTERN` | Performance check | `catastrophic_backtracking`, `throughput_mb_s` |

## Output Format

All commands return JSON by default. Key conventions:
- `matched: true/false` — whether any match was found
- `matches[].start` / `matches[].end` — byte positions (0-indexed)
- `matches[].captures[]` — capture groups with `group`, `name`, `text`
- Exit code 0 = success (even if no matches). Exit code 1 = error.

## Examples

### Test a date pattern
```bash
re-x test '\d{4}-\d{2}-\d{2}' '2024-01-15 meeting scheduled'
```

### Test with capture groups
```bash
re-x test '(\d{3})-(\d{4})' 'Call 123-4567 or 987-6543'
```

### Validate for JavaScript
```bash
re-x validate '(?<=\d{3})\w+' --target-lang javascript
# → portability.javascript: false (variable-length lookbehind not supported)
```

### Explain a complex pattern
```bash
re-x explain '^(?:https?://)?(?:www\.)?([^/]+)'
```

### Infer pattern from examples
```bash
re-x from-examples '2024-01-15' '2025-12-31' '2023-06-01'
```

### Apply replacements to a file
```bash
re-x apply 'http://' 'https://' --file urls.txt --dry-run
# → applied: false, preview shows what would change

re-x apply 'http://' 'https://' --file urls.txt
# → applied: true, backup_path: "urls.txt.bak"
```

### Check for ReDoS
```bash
re-x benchmark '(a+)+$' --input 'aaaaaaaaaaab'
# → catastrophic_backtracking: true
```

## If re-x Is Not Installed

Fall back to these alternatives (limited functionality):
```bash
# Basic match test (no JSON, no captures)
grep -cP 'pattern' file.txt

# Python one-liner (slower, Python only)
python3 -c "import re; print(re.findall(r'pattern', 'input'))"
```

Note: These alternatives cannot check cross-language portability, detect ReDoS, or return structured match positions.

## Tips

- Use raw strings in patterns: single-quote in bash to avoid escaping issues
- For patterns with `'` inside, use `$'pattern\'s'` bash syntax
- The `--max-matches` flag is useful when testing against large files to avoid excessive output
- `re-x explain` is invaluable for understanding regex found in unfamiliar codebases
- Always run `re-x benchmark` before using regex on untrusted user input
