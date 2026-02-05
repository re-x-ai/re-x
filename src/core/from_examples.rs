//! Implementation of `re-x from-examples` command
//!
//! Infers regex patterns from example strings.

use super::templates::detect_known_formats;
use crate::output::{FromExamplesResult, InferredPattern};

/// Infer patterns from examples
pub fn infer_patterns(
    examples: &[String],
    negative_examples: Option<&[String]>,
) -> Result<FromExamplesResult, String> {
    if examples.is_empty() {
        return Err("At least one example is required".to_string());
    }

    if examples.len() < 2 {
        return Err("At least two examples are recommended for better inference".to_string());
    }

    let mut candidates = Vec::new();

    // Strategy 1: Known format templates (highest priority — precise patterns)
    // Template patterns are curated, so skip the generic specificity penalty.
    for (pattern, desc) in detect_known_formats(examples) {
        let confidence = calculate_confidence(&pattern, examples, negative_examples, true);
        candidates.push(InferredPattern {
            pattern,
            confidence,
            desc,
        });
    }

    // Strategy 2: Character class based inference
    if let Some(pattern) = infer_character_classes(examples) {
        let confidence = calculate_confidence(&pattern, examples, negative_examples, false);
        candidates.push(InferredPattern {
            pattern,
            confidence,
            desc: "Character class based pattern".to_string(),
        });
    }

    // Strategy 3: Common structure detection
    if let Some((pattern, desc)) = infer_common_structure(examples) {
        let confidence = calculate_confidence(&pattern, examples, negative_examples, false);
        candidates.push(InferredPattern {
            pattern,
            confidence,
            desc,
        });
    }

    // Strategy 4: Exact literal pattern (if all examples are identical)
    if examples.iter().all(|e| e == &examples[0]) {
        let escaped = regex::escape(&examples[0]);
        candidates.push(InferredPattern {
            pattern: escaped,
            confidence: 1.0,
            desc: "Exact match (all examples identical)".to_string(),
        });
    }

    // Strategy 5: Literal prefix/suffix with wildcard
    if let Some((pattern, desc)) = infer_anchored_pattern(examples) {
        let confidence = calculate_confidence(&pattern, examples, negative_examples, false);
        candidates.push(InferredPattern {
            pattern,
            confidence,
            desc,
        });
    }

    // Sort by confidence (highest first) and deduplicate
    candidates.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.dedup_by(|a, b| a.pattern == b.pattern);

    // Keep top 5
    candidates.truncate(5);

    Ok(FromExamplesResult {
        examples: examples.to_vec(),
        negative_examples: negative_examples.map(|n| n.to_vec()),
        inferred: candidates,
    })
}

/// Infer pattern based on character classes
fn infer_character_classes(examples: &[String]) -> Option<String> {
    if examples.is_empty() {
        return None;
    }

    let mut pattern = String::new();
    let max_len = examples.iter().map(|e| e.len()).max().unwrap_or(0);
    let min_len = examples.iter().map(|e| e.len()).min().unwrap_or(0);

    // Pre-convert to char vectors to avoid O(n²) from repeated chars().nth()
    let char_vecs: Vec<Vec<char>> = examples.iter().map(|e| e.chars().collect()).collect();

    // Build pattern character by character
    let mut pos = 0;
    while pos < min_len {
        let chars_at_pos: Vec<char> = char_vecs
            .iter()
            .filter_map(|chars| chars.get(pos).copied())
            .collect();

        if chars_at_pos.is_empty() {
            break;
        }

        let first_char = chars_at_pos[0];

        // Check if all chars at this position are the same
        if chars_at_pos.iter().all(|&c| c == first_char) {
            // Literal character
            pattern.push_str(&regex::escape(&first_char.to_string()));
        } else if chars_at_pos.iter().all(|c| c.is_ascii_digit()) {
            // All digits
            pattern.push_str(r"\d");
        } else if chars_at_pos.iter().all(|c| c.is_ascii_alphabetic()) {
            if chars_at_pos.iter().all(|c| c.is_ascii_lowercase()) {
                pattern.push_str("[a-z]");
            } else if chars_at_pos.iter().all(|c| c.is_ascii_uppercase()) {
                pattern.push_str("[A-Z]");
            } else {
                pattern.push_str("[a-zA-Z]");
            }
        } else if chars_at_pos.iter().all(|c| c.is_ascii_alphanumeric()) {
            pattern.push_str(r"\w");
        } else {
            // Generic non-whitespace character
            pattern.push_str(r"\S");
        }

        pos += 1;
    }

    // Handle variable length
    if max_len > min_len {
        pattern.push_str(r".*");
    }

    if pattern.is_empty() {
        None
    } else {
        Some(pattern)
    }
}

/// Detect common structure in examples
fn infer_common_structure(examples: &[String]) -> Option<(String, String)> {
    // Check for repeated patterns with separators
    let separators = ['-', '/', '.', '_', ' ', ':'];

    for sep in &separators {
        if examples.iter().all(|e| e.contains(*sep)) {
            // Split by separator and analyze parts
            let parts: Vec<Vec<&str>> = examples.iter().map(|e| e.split(*sep).collect()).collect();

            // Check if all have same number of parts
            let part_count = parts[0].len();
            if parts.iter().all(|p| p.len() == part_count) {
                let mut pattern_parts = Vec::new();

                for i in 0..part_count {
                    let part_examples: Vec<&str> = parts.iter().map(|p| p[i]).collect();

                    // Analyze each part
                    if part_examples
                        .iter()
                        .all(|p| p.chars().all(|c| c.is_ascii_digit()))
                    {
                        let max_digits = part_examples.iter().map(|p| p.len()).max().unwrap_or(1);
                        let min_digits = part_examples.iter().map(|p| p.len()).min().unwrap_or(1);

                        if max_digits == min_digits {
                            pattern_parts.push(format!(r"\d{{{}}}", max_digits));
                        } else {
                            pattern_parts.push(format!(r"\d{{{},{}}}", min_digits, max_digits));
                        }
                    } else if part_examples
                        .iter()
                        .all(|p| p.chars().all(|c| c.is_ascii_alphabetic()))
                    {
                        let max_chars = part_examples.iter().map(|p| p.len()).max().unwrap_or(1);
                        let min_chars = part_examples.iter().map(|p| p.len()).min().unwrap_or(1);

                        if max_chars == min_chars {
                            pattern_parts.push(format!("[a-zA-Z]{{{}}}", max_chars));
                        } else {
                            pattern_parts.push(format!("[a-zA-Z]{{{},{}}}", min_chars, max_chars));
                        }
                    } else {
                        pattern_parts.push(r"[^".to_string() + &sep.to_string() + "]+");
                    }
                }

                let escaped_sep = regex::escape(&sep.to_string());
                let pattern = pattern_parts.join(&escaped_sep);
                let desc = format!("{}-separated pattern with {} parts", sep, part_count);
                return Some((pattern, desc));
            }
        }
    }

    None
}

/// Infer pattern with common prefix/suffix
fn infer_anchored_pattern(examples: &[String]) -> Option<(String, String)> {
    // Find common prefix
    let common_prefix = examples.iter().fold(examples[0].clone(), |acc, s| {
        acc.chars()
            .zip(s.chars())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c)
            .collect()
    });

    // Find common suffix
    let common_suffix: String = examples
        .iter()
        .map(|s| s.chars().rev().collect::<String>())
        .fold(examples[0].chars().rev().collect::<String>(), |acc, s| {
            acc.chars()
                .zip(s.chars())
                .take_while(|(a, b)| a == b)
                .map(|(c, _)| c)
                .collect()
        })
        .chars()
        .rev()
        .collect();

    if !common_prefix.is_empty() || !common_suffix.is_empty() {
        let mut pattern = String::new();

        if !common_prefix.is_empty() {
            pattern.push_str(&regex::escape(&common_prefix));
        }

        pattern.push_str(r".*?");

        if !common_suffix.is_empty() {
            pattern.push_str(&regex::escape(&common_suffix));
        }

        let desc = if !common_prefix.is_empty() && !common_suffix.is_empty() {
            format!(
                "Common prefix '{}' and suffix '{}'",
                common_prefix, common_suffix
            )
        } else if !common_prefix.is_empty() {
            format!("Common prefix '{}'", common_prefix)
        } else {
            format!("Common suffix '{}'", common_suffix)
        };

        return Some((pattern, desc));
    }

    None
}

/// Calculate confidence score for a pattern.
///
/// `is_template` — when true, the pattern comes from a curated template
/// and the generic specificity penalty (dot-count) is skipped.
/// Templates are capped at 0.95 to leave room for exact-match patterns.
fn calculate_confidence(
    pattern: &str,
    examples: &[String],
    negative_examples: Option<&[String]>,
    is_template: bool,
) -> f64 {
    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return 0.0,
    };

    // Count how many examples match
    let positive_matches = examples.iter().filter(|e| re.is_match(e)).count();
    let positive_total = examples.len();

    let mut confidence = positive_matches as f64 / positive_total as f64;

    // Penalize if negative examples match
    if let Some(negatives) = negative_examples {
        let negative_matches = negatives.iter().filter(|e| re.is_match(e)).count();
        let negative_total = negatives.len();

        if negative_total > 0 {
            let false_positive_rate = negative_matches as f64 / negative_total as f64;
            confidence *= 1.0 - false_positive_rate;
        }
    }

    if is_template {
        // Template patterns are curated — no specificity penalty.
        // Cap at 0.95 to keep exact-match (1.0) ranked higher.
        confidence = confidence.min(0.95);
    } else {
        // Generic patterns: penalize many wildcard dots (`.`)
        let specificity = 1.0 - (pattern.matches('.').count() as f64 * 0.1).min(0.3);
        confidence *= specificity;
        confidence = confidence.min(1.0);
    }

    confidence
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_date_pattern() {
        let examples = vec![
            "2024-01-15".to_string(),
            "2025-12-31".to_string(),
            "2023-06-01".to_string(),
        ];

        let result = infer_patterns(&examples, None).unwrap();
        assert!(!result.inferred.is_empty());

        // Should contain ISO date pattern
        assert!(result
            .inferred
            .iter()
            .any(|p| p.pattern.contains(r"\d{4}-\d{2}-\d{2}")));
    }

    #[test]
    fn test_infer_with_negatives() {
        let examples = vec!["abc123".to_string(), "def456".to_string()];
        let negatives = vec!["123abc".to_string(), "xyz".to_string()];

        let result = infer_patterns(&examples, Some(&negatives)).unwrap();
        assert!(!result.inferred.is_empty());
    }

    #[test]
    fn test_infer_email() {
        let examples = vec![
            "user@example.com".to_string(),
            "admin@test.org".to_string(),
            "info@company.co.uk".to_string(),
        ];

        let result = infer_patterns(&examples, None).unwrap();
        assert!(result.inferred.iter().any(|p| p.desc.contains("Email")));
    }

    #[test]
    fn test_infer_ipv4() {
        let examples = vec![
            "192.168.1.1".to_string(),
            "10.0.0.1".to_string(),
            "255.255.255.0".to_string(),
        ];

        let result = infer_patterns(&examples, None).unwrap();
        assert!(result.inferred.iter().any(|p| p.desc.contains("IPv4")));
    }

    #[test]
    fn test_infer_uuid() {
        let examples = vec![
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
            "123e4567-e89b-12d3-a456-426614174000".to_string(),
        ];

        let result = infer_patterns(&examples, None).unwrap();
        assert!(result.inferred.iter().any(|p| p.desc.contains("UUID")));
    }

    #[test]
    fn test_infer_semver() {
        let examples = vec![
            "1.0.0".to_string(),
            "2.3.4".to_string(),
            "10.20.30".to_string(),
        ];

        let result = infer_patterns(&examples, None).unwrap();
        assert!(result
            .inferred
            .iter()
            .any(|p| p.desc.contains("Semantic version")));
    }

    #[test]
    fn test_infer_hex_color() {
        let examples = vec![
            "#ff0000".to_string(),
            "#00ff00".to_string(),
            "#0000ff".to_string(),
        ];

        let result = infer_patterns(&examples, None).unwrap();
        assert!(result.inferred.iter().any(|p| p.desc.contains("Hex color")));
    }

    #[test]
    fn test_infer_url() {
        let examples = vec![
            "https://example.com".to_string(),
            "http://test.org/path".to_string(),
        ];

        let result = infer_patterns(&examples, None).unwrap();
        assert!(result.inferred.iter().any(|p| p.desc.contains("URL")));
    }

    #[test]
    fn test_ipv4_ranks_above_phone() {
        let examples = vec![
            "192.168.1.1".to_string(),
            "10.0.0.1".to_string(),
            "255.255.255.0".to_string(),
        ];

        let result = infer_patterns(&examples, None).unwrap();
        let ipv4_pos = result.inferred.iter().position(|p| p.desc.contains("IPv4"));
        let phone_pos = result
            .inferred
            .iter()
            .position(|p| p.desc.contains("Phone"));

        // IPv4 should appear before Phone (higher confidence)
        assert!(ipv4_pos.is_some(), "IPv4 pattern should be present");
        if let (Some(ip), Some(ph)) = (ipv4_pos, phone_pos) {
            assert!(
                ip < ph,
                "IPv4 (pos {}) should rank above Phone (pos {})",
                ip,
                ph
            );
        }
    }
}
