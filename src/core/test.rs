//! Implementation of `re-x test` command
//!
//! Tests a regex pattern against input text or a file, returning all matches
//! with positions and capture groups.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use std::time::Instant;

use super::engine::{CompiledRegex, EngineType};
use crate::output::{Capture, Match, TestResult};

/// Options for the test command
pub struct TestOptions {
    /// Maximum number of matches to return
    pub max_matches: Option<usize>,
    /// Force a specific engine
    pub engine: Option<EngineType>,
    /// Enable multiline mode ((?ms) — dot matches newline, ^/$ match line boundaries)
    pub multiline: bool,
}

impl Default for TestOptions {
    fn default() -> Self {
        Self {
            max_matches: Some(100),
            engine: None,
            multiline: false,
        }
    }
}

/// Apply multiline flags to pattern if needed
fn apply_multiline(pattern: &str, multiline: bool) -> String {
    if multiline && !pattern.starts_with("(?") {
        format!("(?ms){}", pattern)
    } else if multiline {
        // Pattern already has flags — inject m and s
        format!("(?ms){}", pattern)
    } else {
        pattern.to_string()
    }
}

/// Test a pattern against a string
pub fn test_string(
    pattern: &str,
    input: &str,
    options: &TestOptions,
) -> Result<TestResult, String> {
    let start = Instant::now();

    let effective_pattern = apply_multiline(pattern, options.multiline);
    let pattern_ref = effective_pattern.as_str();

    // Compile the regex
    let (compiled, engine_type) = match options.engine {
        Some(engine) => {
            let compiled =
                CompiledRegex::with_engine(pattern_ref, engine).map_err(|e| e.to_string())?;
            (compiled, engine)
        }
        None => CompiledRegex::new(pattern_ref).map_err(|e| e.to_string())?,
    };

    let max_matches = options.max_matches.unwrap_or(usize::MAX);
    let matches = collect_matches(&compiled, input, pattern_ref, max_matches)?;

    let elapsed = start.elapsed();

    Ok(TestResult {
        pattern: pattern.to_string(),
        engine: engine_type.to_string(),
        input_length: input.len(),
        matched: !matches.is_empty(),
        match_count: matches.len(),
        matches,
        elapsed_us: elapsed.as_micros() as u64,
    })
}

/// Test a pattern against a file (streaming)
pub fn test_file(
    pattern: &str,
    file_path: &Path,
    options: &TestOptions,
) -> Result<TestResult, String> {
    let start = Instant::now();

    let effective_pattern = apply_multiline(pattern, options.multiline);
    let pattern_ref = effective_pattern.as_str();

    // Compile the regex
    let (compiled, engine_type) = match options.engine {
        Some(engine) => {
            let compiled =
                CompiledRegex::with_engine(pattern_ref, engine).map_err(|e| e.to_string())?;
            (compiled, engine)
        }
        None => CompiledRegex::new(pattern_ref).map_err(|e| e.to_string())?,
    };

    // Open file
    let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;

    let metadata = file
        .metadata()
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

    let file_size = metadata.len() as usize;
    let max_matches = options.max_matches.unwrap_or(usize::MAX);

    // Multiline mode requires full content (pattern spans across lines).
    // For small files, also read entirely into memory.
    // For large files without multiline, process line by line.
    let matches = if options.multiline || file_size < 10 * 1024 * 1024 {
        let mut content = String::new();
        BufReader::new(file)
            .read_to_string(&mut content)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        collect_matches(&compiled, &content, pattern_ref, max_matches)?
    } else {
        // Large file without multiline - process line by line
        collect_matches_streaming(&compiled, file, pattern_ref, max_matches)?
    };

    let elapsed = start.elapsed();

    Ok(TestResult {
        pattern: pattern.to_string(),
        engine: engine_type.to_string(),
        input_length: file_size,
        matched: !matches.is_empty(),
        match_count: matches.len(),
        matches,
        elapsed_us: elapsed.as_micros() as u64,
    })
}

/// Test a pattern against stdin
pub fn test_stdin(pattern: &str, options: &TestOptions) -> Result<TestResult, String> {
    let start = Instant::now();

    let effective_pattern = apply_multiline(pattern, options.multiline);
    let pattern_ref = effective_pattern.as_str();

    // Compile the regex
    let (compiled, engine_type) = match options.engine {
        Some(engine) => {
            let compiled =
                CompiledRegex::with_engine(pattern_ref, engine).map_err(|e| e.to_string())?;
            (compiled, engine)
        }
        None => CompiledRegex::new(pattern_ref).map_err(|e| e.to_string())?,
    };

    // Read stdin
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("Failed to read stdin: {}", e))?;

    let max_matches = options.max_matches.unwrap_or(usize::MAX);
    let matches = collect_matches(&compiled, &input, pattern_ref, max_matches)?;

    let elapsed = start.elapsed();

    Ok(TestResult {
        pattern: pattern.to_string(),
        engine: engine_type.to_string(),
        input_length: input.len(),
        matched: !matches.is_empty(),
        match_count: matches.len(),
        matches,
        elapsed_us: elapsed.as_micros() as u64,
    })
}

/// Collect all matches from text
fn collect_matches(
    compiled: &CompiledRegex,
    text: &str,
    pattern: &str,
    max_matches: usize,
) -> Result<Vec<Match>, String> {
    let mut matches = Vec::new();

    match compiled {
        CompiledRegex::Regex(re) => {
            // Try to use captures if the pattern has capture groups
            let has_captures = super::engine::has_capturing_groups(pattern);

            if has_captures {
                for caps in re.captures_iter(text) {
                    if matches.len() >= max_matches {
                        break;
                    }

                    if let Some(full_match) = caps.get(0) {
                        let mut captures = Vec::new();

                        // Collect capture groups (skip group 0 which is the full match)
                        for (i, cap) in caps.iter().enumerate().skip(1) {
                            if let Some(c) = cap {
                                captures.push(Capture {
                                    group: i,
                                    name: re
                                        .capture_names()
                                        .nth(i)
                                        .flatten()
                                        .map(|s| s.to_string()),
                                    text: c.as_str().to_string(),
                                    start: c.start(),
                                    end: c.end(),
                                });
                            }
                        }

                        matches.push(Match {
                            text: full_match.as_str().to_string(),
                            start: full_match.start(),
                            end: full_match.end(),
                            captures,
                        });
                    }
                }
            } else {
                for m in re.find_iter(text) {
                    if matches.len() >= max_matches {
                        break;
                    }

                    matches.push(Match {
                        text: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        captures: Vec::new(),
                    });
                }
            }
        }

        CompiledRegex::FancyRegex(re) => {
            let has_captures = super::engine::has_capturing_groups(pattern);

            if has_captures {
                let mut search_start = 0;
                while search_start < text.len() && matches.len() < max_matches {
                    let result = re
                        .captures_from_pos(text, search_start)
                        .map_err(|e| e.to_string())?;

                    match result {
                        Some(caps) => {
                            if let Some(full_match) = caps.get(0) {
                                let mut captures = Vec::new();

                                for i in 1..caps.len() {
                                    if let Some(c) = caps.get(i) {
                                        captures.push(Capture {
                                            group: i,
                                            name: re
                                                .capture_names()
                                                .nth(i)
                                                .flatten()
                                                .map(|s| s.to_string()),
                                            text: c.as_str().to_string(),
                                            start: c.start(),
                                            end: c.end(),
                                        });
                                    }
                                }

                                search_start = full_match.end().max(search_start + 1);

                                matches.push(Match {
                                    text: full_match.as_str().to_string(),
                                    start: full_match.start(),
                                    end: full_match.end(),
                                    captures,
                                });
                            } else {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            } else {
                let mut search_start = 0;
                while search_start < text.len() && matches.len() < max_matches {
                    let result = re
                        .find_from_pos(text, search_start)
                        .map_err(|e| e.to_string())?;

                    match result {
                        Some(m) => {
                            matches.push(Match {
                                text: m.as_str().to_string(),
                                start: m.start(),
                                end: m.end(),
                                captures: Vec::new(),
                            });
                            search_start = m.end().max(search_start + 1);
                        }
                        None => break,
                    }
                }
            }
        }
    }

    Ok(matches)
}

/// Collect matches from a file using streaming (line by line)
fn collect_matches_streaming(
    compiled: &CompiledRegex,
    file: File,
    pattern: &str,
    max_matches: usize,
) -> Result<Vec<Match>, String> {
    let mut matches = Vec::new();
    let mut reader = BufReader::new(file);
    let mut byte_offset = 0usize;
    let mut raw_line = String::new();

    loop {
        raw_line.clear();
        let bytes_read = reader
            .read_line(&mut raw_line)
            .map_err(|e| format!("Failed to read line: {}", e))?;

        if bytes_read == 0 {
            break; // EOF
        }

        if matches.len() >= max_matches {
            break;
        }

        // Strip the line ending for matching, but use raw length for offset
        let line = raw_line.trim_end_matches(&['\n', '\r'][..]);

        let line_matches = collect_matches(compiled, line, pattern, max_matches - matches.len())?;

        // Adjust positions to account for byte offset
        for mut m in line_matches {
            m.start += byte_offset;
            m.end += byte_offset;
            for cap in &mut m.captures {
                cap.start += byte_offset;
                cap.end += byte_offset;
            }
            matches.push(m);
        }

        byte_offset += raw_line.len(); // includes actual line ending (\n or \r\n)
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_match() {
        let result = test_string(r"\d+", "hello 123 world 456", &TestOptions::default()).unwrap();
        assert!(result.matched);
        assert_eq!(result.match_count, 2);
        assert_eq!(result.matches[0].text, "123");
        assert_eq!(result.matches[1].text, "456");
    }

    #[test]
    fn test_no_match() {
        let result = test_string(r"\d+", "hello world", &TestOptions::default()).unwrap();
        assert!(!result.matched);
        assert_eq!(result.match_count, 0);
    }

    #[test]
    fn test_with_captures() {
        let result = test_string(r"(\d+)-(\d+)", "123-456", &TestOptions::default()).unwrap();
        assert!(result.matched);
        assert_eq!(result.match_count, 1);
        assert_eq!(result.matches[0].captures.len(), 2);
        assert_eq!(result.matches[0].captures[0].text, "123");
        assert_eq!(result.matches[0].captures[1].text, "456");
    }

    #[test]
    fn test_max_matches() {
        let options = TestOptions {
            max_matches: Some(1),
            engine: None,
            multiline: false,
        };
        let result = test_string(r"\d+", "1 2 3 4 5", &options).unwrap();
        assert_eq!(result.match_count, 1);
    }

    #[test]
    fn test_multiline_dot_matches_newline() {
        let options = TestOptions {
            max_matches: Some(100),
            engine: None,
            multiline: true,
        };
        let result = test_string(r"hello.world", "hello\nworld", &options).unwrap();
        assert!(result.matched);
        assert_eq!(result.matches[0].text, "hello\nworld");
    }

    #[test]
    fn test_multiline_anchors() {
        let options = TestOptions {
            max_matches: Some(100),
            engine: None,
            multiline: true,
        };
        let result = test_string(r"^\w+$", "foo\nbar\nbaz", &options).unwrap();
        assert_eq!(result.match_count, 3);
    }
}
