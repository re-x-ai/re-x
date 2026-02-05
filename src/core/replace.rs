//! Implementation of `re-x replace` command
//!
//! Tests regex replacement without modifying files.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read as _};
use std::path::Path;

use super::engine::CompiledRegex;
use crate::output::{ApplyResult, ReplaceFileResult, ReplacePreview, ReplaceResult};

/// Apply multiline flags to pattern if needed
fn apply_multiline(pattern: &str, multiline: bool) -> String {
    if multiline {
        format!("(?ms){}", pattern)
    } else {
        pattern.to_string()
    }
}

/// Replace all occurrences in a string
#[allow(dead_code)]
pub fn replace_string(
    pattern: &str,
    replacement: &str,
    input: &str,
) -> Result<ReplaceResult, String> {
    let (compiled, _engine) = CompiledRegex::new(pattern).map_err(|e| e.to_string())?;

    let (result, count) = match &compiled {
        CompiledRegex::Regex(re) => {
            let mut count = 0;
            let _count_only = re.replace_all(input, |_caps: &regex::Captures| {
                count += 1;
                replacement.to_string()
            });

            // Re-do with actual replacement to handle backreferences
            let result = re.replace_all(input, replacement);
            (result.into_owned(), count)
        }
        CompiledRegex::FancyRegex(re) => {
            let mut count = 0;
            let mut last_end = 0;
            let mut result = String::new();

            loop {
                match re.captures_from_pos(input, last_end) {
                    Ok(Some(caps)) => {
                        if let Some(full_match) = caps.get(0) {
                            result.push_str(&input[last_end..full_match.start()]);
                            let expanded = expand_replacement(replacement, &caps);
                            result.push_str(&expanded);
                            last_end = full_match.end();
                            count += 1;

                            if full_match.start() == full_match.end() {
                                if last_end < input.len() {
                                    result.push_str(&input[last_end..last_end + 1]);
                                    last_end += 1;
                                } else {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    Ok(None) => {
                        result.push_str(&input[last_end..]);
                        break;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }

            (result, count)
        }
    };

    Ok(ReplaceResult {
        pattern: pattern.to_string(),
        replacement: replacement.to_string(),
        original: input.to_string(),
        result,
        replacements_made: count,
    })
}

/// Replace all occurrences in a content string, returning (new_content, count).
/// Handles capture expansion for both regex and fancy-regex engines.
fn replace_content(
    compiled: &CompiledRegex,
    content: &str,
    replacement: &str,
) -> Result<(String, usize), String> {
    match compiled {
        CompiledRegex::Regex(re) => {
            let count = re.find_iter(content).count();
            let result = re.replace_all(content, replacement).into_owned();
            Ok((result, count))
        }
        CompiledRegex::FancyRegex(re) => {
            let mut result = String::new();
            let mut last_end = 0;
            let mut count = 0;

            loop {
                match re.captures_from_pos(content, last_end) {
                    Ok(Some(caps)) => {
                        if let Some(full_match) = caps.get(0) {
                            result.push_str(&content[last_end..full_match.start()]);
                            let expanded = expand_replacement(replacement, &caps);
                            result.push_str(&expanded);
                            last_end = full_match.end();
                            count += 1;

                            if full_match.start() == full_match.end() {
                                if last_end < content.len() {
                                    result.push_str(&content[last_end..last_end + 1]);
                                    last_end += 1;
                                } else {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    Ok(None) => {
                        result.push_str(&content[last_end..]);
                        break;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }

            Ok((result, count))
        }
    }
}

/// Generate line-by-line preview by diffing original and new content
fn diff_preview(original: &str, new_content: &str, max_preview: usize) -> Vec<ReplacePreview> {
    let mut preview = Vec::new();
    let original_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    if original_lines.len() != new_lines.len() {
        // Line count changed (multiline replacement merged/split lines)
        // Show a single diff entry for the whole file
        if original != new_content {
            preview.push(ReplacePreview {
                line: 1,
                before: original.to_string(),
                after: new_content.to_string(),
            });
        }
    } else {
        for (i, (orig, new)) in original_lines.iter().zip(new_lines.iter()).enumerate() {
            if orig != new && preview.len() < max_preview {
                preview.push(ReplacePreview {
                    line: i + 1,
                    before: orig.to_string(),
                    after: new.to_string(),
                });
            }
        }
    }

    preview
}

/// Preview replacements in a file (dry-run, never modifies the file)
pub fn replace_file_preview(
    pattern: &str,
    replacement: &str,
    file_path: &Path,
    max_preview: Option<usize>,
    multiline: bool,
) -> Result<ReplaceFileResult, String> {
    let effective_pattern = apply_multiline(pattern, multiline);
    let (compiled, _engine) = CompiledRegex::new(&effective_pattern).map_err(|e| e.to_string())?;
    let max_preview = max_preview.unwrap_or(20);

    if multiline {
        // Multiline: process entire content as one string for cross-line matches
        let mut content = String::new();
        File::open(file_path)
            .and_then(|mut f| {
                f.read_to_string(&mut content)?;
                Ok(())
            })
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let (new_content, total_replacements) = replace_content(&compiled, &content, replacement)?;
        let preview = diff_preview(&content, &new_content, max_preview);

        Ok(ReplaceFileResult {
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
            replacements_made: total_replacements,
            preview,
        })
    } else {
        // Non-multiline: line-by-line processing (streaming, memory efficient)
        let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        let mut total_replacements = 0;
        let mut preview = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result.map_err(|e| format!("Failed to read line: {}", e))?;
            let (new_line, count) = replace_line(&compiled, &line, replacement)?;
            if count > 0 {
                total_replacements += count;
                if preview.len() < max_preview {
                    preview.push(ReplacePreview {
                        line: line_num + 1,
                        before: line,
                        after: new_line,
                    });
                }
            }
        }

        Ok(ReplaceFileResult {
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
            replacements_made: total_replacements,
            preview,
        })
    }
}

/// Replace in a single line and return the result with count
fn replace_line(
    compiled: &CompiledRegex,
    line: &str,
    replacement: &str,
) -> Result<(String, usize), String> {
    match compiled {
        CompiledRegex::Regex(re) => {
            let mut count = 0;
            let _count_only = re.replace_all(line, |_caps: &regex::Captures| {
                count += 1;
                replacement.to_string()
            });

            // Re-do with actual replacement
            let result = re.replace_all(line, replacement);
            Ok((result.into_owned(), count))
        }
        CompiledRegex::FancyRegex(re) => {
            let mut count = 0;
            let mut last_end = 0;
            let mut result = String::new();

            loop {
                match re.captures_from_pos(line, last_end) {
                    Ok(Some(caps)) => {
                        if let Some(full_match) = caps.get(0) {
                            result.push_str(&line[last_end..full_match.start()]);
                            let expanded = expand_replacement(replacement, &caps);
                            result.push_str(&expanded);
                            last_end = full_match.end();
                            count += 1;

                            if full_match.start() == full_match.end() {
                                if last_end < line.len() {
                                    result.push_str(&line[last_end..last_end + 1]);
                                    last_end += 1;
                                } else {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    Ok(None) => {
                        result.push_str(&line[last_end..]);
                        break;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }

            Ok((result, count))
        }
    }
}

/// Replace all occurrences in a string with capture group references
/// Supports $1, $2, etc. and ${name} syntax
pub fn replace_with_captures(
    pattern: &str,
    replacement: &str,
    input: &str,
    multiline: bool,
) -> Result<ReplaceResult, String> {
    let effective_pattern = apply_multiline(pattern, multiline);
    let (compiled, _engine) = CompiledRegex::new(&effective_pattern).map_err(|e| e.to_string())?;

    let (result, count) = match &compiled {
        CompiledRegex::Regex(re) => {
            let count = re.find_iter(input).count();
            let result = re.replace_all(input, replacement).into_owned();
            (result, count)
        }
        CompiledRegex::FancyRegex(re) => {
            // For fancy-regex, we need to handle captures manually
            let mut result = String::new();
            let mut last_end = 0;
            let mut count = 0;

            loop {
                match re.captures_from_pos(input, last_end) {
                    Ok(Some(caps)) => {
                        if let Some(full_match) = caps.get(0) {
                            result.push_str(&input[last_end..full_match.start()]);

                            // Expand replacement with captures
                            let expanded = expand_replacement(replacement, &caps);
                            result.push_str(&expanded);

                            last_end = full_match.end();
                            count += 1;

                            if full_match.start() == full_match.end() {
                                if last_end < input.len() {
                                    result.push_str(&input[last_end..last_end + 1]);
                                    last_end += 1;
                                } else {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    Ok(None) => {
                        result.push_str(&input[last_end..]);
                        break;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }

            (result, count)
        }
    };

    Ok(ReplaceResult {
        pattern: pattern.to_string(),
        replacement: replacement.to_string(),
        original: input.to_string(),
        result,
        replacements_made: count,
    })
}

/// Apply regex replacements to a file, optionally creating a backup.
///
/// * `dry_run` — if true, previews changes without writing.
/// * `backup` — if true, copies the original file to `<path>.bak` before writing.
/// * `multiline` — if true, enables cross-line matching with `(?ms)` flags.
pub fn apply_file(
    pattern: &str,
    replacement: &str,
    file_path: &Path,
    dry_run: bool,
    backup: bool,
    max_preview: Option<usize>,
    multiline: bool,
) -> Result<ApplyResult, String> {
    let effective_pattern = apply_multiline(pattern, multiline);
    let (compiled, _engine) = CompiledRegex::new(&effective_pattern).map_err(|e| e.to_string())?;

    // Read entire file
    let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut content = String::new();
    BufReader::new(file)
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let max_preview = max_preview.unwrap_or(20);

    let (new_content, total_replacements, preview) = if multiline {
        // Multiline: replace on full content, then diff for preview
        let (new_content, count) = replace_content(&compiled, &content, replacement)?;
        let preview = diff_preview(&content, &new_content, max_preview);
        (new_content, count, preview)
    } else {
        // Line-by-line processing
        let mut total = 0;
        let mut preview = Vec::new();
        let mut new_lines = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let (new_line, count) = replace_line(&compiled, line, replacement)?;
            if count > 0 {
                total += count;
                if preview.len() < max_preview {
                    preview.push(ReplacePreview {
                        line: line_num + 1,
                        before: line.to_string(),
                        after: new_line.clone(),
                    });
                }
            }
            new_lines.push(new_line);
        }

        let new_content = if content.ends_with('\n') {
            new_lines.join("\n") + "\n"
        } else {
            new_lines.join("\n")
        };

        (new_content, total, preview)
    };

    let mut backup_path = None;

    if !dry_run && total_replacements > 0 {
        if backup {
            let bak = std::path::PathBuf::from(format!("{}.bak", file_path.display()));
            fs::copy(file_path, &bak).map_err(|e| format!("Failed to create backup: {}", e))?;
            backup_path = Some(bak.to_string_lossy().into_owned());
        }

        fs::write(file_path, &new_content).map_err(|e| format!("Failed to write file: {}", e))?;
    }

    Ok(ApplyResult {
        pattern: pattern.to_string(),
        replacement: replacement.to_string(),
        file_path: file_path.to_string_lossy().into_owned(),
        backup_path,
        replacements_made: total_replacements,
        applied: !dry_run && total_replacements > 0,
        preview,
    })
}

/// Expand replacement string with capture groups
fn expand_replacement(replacement: &str, caps: &fancy_regex::Captures) -> String {
    let mut result = String::new();
    let mut chars = replacement.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            match chars.peek() {
                Some(&d) if d.is_ascii_digit() => {
                    // $1, $2, etc.
                    chars.next();
                    let group_num: usize = d.to_digit(10).unwrap() as usize;
                    if let Some(m) = caps.get(group_num) {
                        result.push_str(m.as_str());
                    }
                }
                Some(&'{') => {
                    // ${name} or ${num}
                    chars.next(); // consume '{'
                    let mut name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            chars.next();
                            break;
                        }
                        name.push(c);
                        chars.next();
                    }
                    if let Ok(num) = name.parse::<usize>() {
                        if let Some(m) = caps.get(num) {
                            result.push_str(m.as_str());
                        }
                    } else if let Some(m) = caps.name(&name) {
                        result.push_str(m.as_str());
                    }
                }
                Some(&'$') => {
                    // $$ -> literal $
                    chars.next();
                    result.push('$');
                }
                _ => {
                    result.push('$');
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_replace() {
        let result = replace_string(r"\d+", "NUM", "a1b2c3").unwrap();
        assert_eq!(result.result, "aNUMbNUMcNUM");
        assert_eq!(result.replacements_made, 3);
    }

    #[test]
    fn test_replace_with_captures() {
        let result = replace_with_captures(r"(\d+)-(\d+)", "$2-$1", "Call 123-456", false).unwrap();
        assert_eq!(result.result, "Call 456-123");
    }

    #[test]
    fn test_replace_multiline() {
        let result =
            replace_with_captures(r"hello.world", "REPLACED", "hello\nworld", true).unwrap();
        assert_eq!(result.result, "REPLACED");
        assert_eq!(result.replacements_made, 1);
    }

    #[test]
    fn test_no_match_replace() {
        let result = replace_string(r"\d+", "NUM", "hello").unwrap();
        assert_eq!(result.result, "hello");
        assert_eq!(result.replacements_made, 0);
    }
}
