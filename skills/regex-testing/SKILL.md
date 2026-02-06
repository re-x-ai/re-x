---
name: regex-testing
description: >
  Tests, validates, explains, benchmarks, and applies regular expression
  replacements using the re-x CLI tool. Use when generating regex patterns,
  debugging regex matches, checking cross-language portability, detecting
  ReDoS vulnerabilities, or applying regex-based find-and-replace to files.
allowed-tools:
  - Bash(re-x *)
---

# Regex Testing & Validation with re-x

## Workflow

Follow this cycle when working with regex:

```
Validate → Test → Fix → Check Portability → Apply/Commit
```

### Step 1: Validate syntax
```bash
re-x validate 'PATTERN'
```
Check `valid`. If false, read `error.suggestion` for the fix.

### Step 2: Test against data
```bash
re-x test 'PATTERN' 'sample input'
re-x test 'PATTERN' --file path/to/data.txt
re-x test 'PATTERN' --file big.log --max-matches 10
```
Check `match_count` and `matches[].text`.

### Step 3: Check portability
```bash
re-x validate 'PATTERN' --target-lang javascript
```
Read `portability` to confirm the pattern works in the target language.

### Step 4: Preview replacement
```bash
re-x replace 'PATTERN' 'REPLACEMENT' 'sample input'
```

### Step 5: Apply to file
```bash
re-x apply 'PATTERN' 'REPL' --file path.txt --dry-run
re-x apply 'PATTERN' 'REPL' --file path.txt
re-x apply 'PATTERN' 'REPL' --file path.txt -m  # multiline
```

## Command Reference

| Command | Purpose | Key Output Fields |
|---|---|---|
| `re-x test PAT INPUT` | Test matching | `matched`, `match_count`, `matches[]` |
| `re-x test PAT --file F` | Test on file | Same, streaming for large files |
| `re-x replace PAT REPL INPUT` | Preview replacement | `result`, `replacements_made` |
| `re-x validate PAT` | Syntax + portability (8 langs) | `valid`, `portability{}`, `engine_required` |
| `re-x explain PAT` | Structured breakdown | `parts[]` with `token`, `type`, `desc` |
| `re-x from-examples E1 E2..` | Infer pattern | `inferred[]` with `pattern`, `confidence` |
| `re-x apply PAT REPL --file F` | Apply to file | `applied`, `replacements_made`, `backup_path` |
| `re-x benchmark PAT` | Performance check | `catastrophic_backtracking`, `throughput_mb_s` |

## Output

All commands return JSON by default. Key conventions:
- `matched: true/false` — whether any match was found
- `matches[].start` / `matches[].end` — byte positions (0-indexed)
- `matches[].captures[]` — capture groups with `group`, `name`, `text`
- Exit code 0 = success (even if no matches). Exit code 1 = error.

## Examples

```bash
# Validate and test a date pattern
re-x validate '\d{4}-\d{2}-\d{2}'
re-x test '\d{4}-\d{2}-\d{2}' '2024-01-15 meeting'

# Check portability for JavaScript
re-x validate '(?<=\d{3})\w+' --target-lang javascript

# Explain complex regex
re-x explain '^(?:https?://)?(?:www\.)?([^/]+)'

# Infer from examples
re-x from-examples '2024-01-15' '2025-12-31' '2023-06-01'

# Apply replacement with dry-run
re-x apply 'http://' 'https://' --file urls.txt --dry-run
re-x apply 'http://' 'https://' --file urls.txt

# Check for ReDoS
re-x benchmark '(a+)+$' --input 'aaaaaaaaaaab'
```

## Fallback

If re-x is not installed:
```bash
grep -cP 'pattern' file.txt                                    # basic match
python3 -c "import re; print(re.findall(r'pattern', 'input'))" # Python
```
These cannot check portability, detect ReDoS, or return structured output.

## Tips

- Use single-quoted raw strings in bash to avoid escaping issues
- `--max-matches` limits output when testing against large files
- `re-x explain` helps understand regex in unfamiliar codebases
- Always `re-x benchmark` before using regex on untrusted user input
