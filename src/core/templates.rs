//! Known pattern templates for format recognition
//!
//! Shared table of well-known formats (IPv4, UUID, email, dates, etc.)
//! used by both `from-examples` inference and `explain` semantic recognition.

use std::sync::LazyLock;

/// A known format template
struct FormatTemplate {
    /// Regex that detects if a string is this format (full match, anchored)
    detect: &'static LazyLock<regex::Regex>,
    /// Output regex pattern to suggest
    pattern: &'static str,
    /// Human-readable description
    desc: &'static str,
}

// --- Detection regexes (all anchored for full-match detection) ---

static ISO_DATE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").expect("BUG: ISO date detection pattern is invalid")
});

static US_DATE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^\d{2}/\d{2}/\d{4}$").expect("BUG: US date detection pattern is invalid")
});

static TIME_SHORT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^\d{2}:\d{2}$").expect("BUG: time short detection pattern is invalid")
});

static TIME_LONG_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^\d{2}:\d{2}:\d{2}$").expect("BUG: time long detection pattern is invalid")
});

static EMAIL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$")
        .expect("BUG: email detection pattern is invalid")
});

static IPV4_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$")
        .expect("BUG: IPv4 detection pattern is invalid")
});

static UUID_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
        .expect("BUG: UUID detection pattern is invalid")
});

static URL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^https?://\S+$").expect("BUG: URL detection pattern is invalid")
});

static SEMVER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$")
        .expect("BUG: semver detection pattern is invalid")
});

static HEX_COLOR_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)^#([0-9a-f]{3}|[0-9a-f]{6})$")
        .expect("BUG: hex color detection pattern is invalid")
});

static MAC_ADDR_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)^([0-9a-f]{2}[:-]){5}[0-9a-f]{2}$")
        .expect("BUG: MAC address detection pattern is invalid")
});

static PHONE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^\+?\d[\d\-\s().]{6,}\d$")
        .expect("BUG: phone number detection pattern is invalid")
});

/// Ordered list of templates (more specific patterns first)
fn templates() -> Vec<FormatTemplate> {
    vec![
        // Specific formats first (order matters — more specific before generic)
        FormatTemplate {
            detect: &UUID_RE,
            pattern: r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
            desc: "UUID",
        },
        FormatTemplate {
            detect: &MAC_ADDR_RE,
            pattern: r"[0-9a-fA-F]{2}[:-][0-9a-fA-F]{2}(?:[:-][0-9a-fA-F]{2}){4}",
            desc: "MAC address",
        },
        FormatTemplate {
            detect: &HEX_COLOR_RE,
            pattern: r"#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})",
            desc: "Hex color code",
        },
        FormatTemplate {
            detect: &ISO_DATE_RE,
            pattern: r"\d{4}-\d{2}-\d{2}",
            desc: "ISO 8601 date (YYYY-MM-DD)",
        },
        FormatTemplate {
            detect: &US_DATE_RE,
            pattern: r"\d{2}/\d{2}/\d{4}",
            desc: "US date format (MM/DD/YYYY)",
        },
        FormatTemplate {
            detect: &TIME_LONG_RE,
            pattern: r"\d{2}:\d{2}:\d{2}",
            desc: "Time with seconds (HH:MM:SS)",
        },
        FormatTemplate {
            detect: &TIME_SHORT_RE,
            pattern: r"\d{2}:\d{2}",
            desc: "Time (HH:MM)",
        },
        FormatTemplate {
            detect: &EMAIL_RE,
            pattern: r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
            desc: "Email address",
        },
        FormatTemplate {
            detect: &IPV4_RE,
            pattern: r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}",
            desc: "IPv4 address",
        },
        FormatTemplate {
            detect: &URL_RE,
            pattern: r"https?://\S+",
            desc: "URL (HTTP/HTTPS)",
        },
        FormatTemplate {
            detect: &SEMVER_RE,
            pattern: r"\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?(?:\+[a-zA-Z0-9.]+)?",
            desc: "Semantic version (SemVer)",
        },
        FormatTemplate {
            detect: &PHONE_RE,
            pattern: r"\+?\d[\d\-\s().]{6,}\d",
            desc: "Phone number",
        },
    ]
}

/// Detect known formats from example strings.
///
/// Returns all matching `(pattern, description)` pairs.
/// Templates are ordered most-specific first; only the first match
/// per template family is returned.
pub fn detect_known_formats(examples: &[String]) -> Vec<(String, String)> {
    templates()
        .iter()
        .filter(|t| examples.iter().all(|e| t.detect.is_match(e)))
        .map(|t| (t.pattern.to_string(), t.desc.to_string()))
        .collect()
}

/// Try to recognize what a regex pattern semantically describes.
///
/// Tests the pattern against canonical examples for each known format.
/// Returns `Some(description)` if a known format is recognized.
pub fn recognize_pattern(pattern: &str) -> Option<String> {
    let re = match regex::Regex::new(&format!("^(?:{})$", pattern)) {
        Ok(r) => r,
        Err(_) => return None,
    };

    // Canonical test examples for each format
    let format_tests: &[(&[&str], &[&str], &str)] = &[
        // (positive_examples, negative_examples, description)
        (
            &[
                "550e8400-e29b-41d4-a716-446655440000",
                "123e4567-e89b-12d3-a456-426614174000",
            ],
            &["not-a-uuid", "123"],
            "UUID",
        ),
        (
            &["AA:BB:CC:DD:EE:FF", "00:11:22:33:44:55"],
            &["not-mac", "ZZ:ZZ:ZZ:ZZ:ZZ:ZZ"],
            "MAC address",
        ),
        (
            &["#ff0000", "#0a0", "#ABC123"],
            &["ff0000", "#xyz", "red"],
            "Hex color code",
        ),
        (
            &["2024-01-15", "2000-12-31"],
            &["not-a-date", "2024/01/15"],
            "ISO 8601 date (YYYY-MM-DD)",
        ),
        (
            &["01/15/2024", "12/31/2000"],
            &["2024-01-15", "not-date"],
            "US date format (MM/DD/YYYY)",
        ),
        (
            &["14:30:00", "23:59:59"],
            &["14:30", "not-time"],
            "Time with seconds (HH:MM:SS)",
        ),
        (
            &["14:30", "23:59", "00:00"],
            &["14:30:00", "not-time"],
            "Time (HH:MM)",
        ),
        (
            &["user@example.com", "admin@test.org"],
            &["not-email", "@missing", "no-at-sign"],
            "Email address",
        ),
        (
            &["192.168.1.1", "10.0.0.1", "255.255.255.0"],
            &["not-ip", "999.999.999.999.999"],
            "IPv4 address",
        ),
        (
            &["https://example.com", "http://test.org/path?q=1"],
            &["not-url", "ftp://other"],
            "URL (HTTP/HTTPS)",
        ),
        (
            &["1.0.0", "2.3.4-beta.1", "10.20.30+build.123"],
            &["not-semver", "1.2"],
            "Semantic version (SemVer)",
        ),
    ];

    for (positives, negatives, desc) in format_tests {
        let all_pos_match = positives.iter().all(|e| re.is_match(e));
        let no_neg_match = negatives.iter().all(|e| !re.is_match(e));
        if all_pos_match && no_neg_match {
            return Some(desc.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ipv4() {
        let examples = vec![
            "192.168.1.1".to_string(),
            "10.0.0.1".to_string(),
            "255.255.255.0".to_string(),
        ];
        let results = detect_known_formats(&examples);
        assert!(results.iter().any(|(_, d)| d == "IPv4 address"));
    }

    #[test]
    fn test_detect_uuid() {
        let examples = vec![
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
            "123e4567-e89b-12d3-a456-426614174000".to_string(),
        ];
        let results = detect_known_formats(&examples);
        assert!(results.iter().any(|(_, d)| d == "UUID"));
    }

    #[test]
    fn test_detect_url() {
        let examples = vec![
            "https://example.com/path".to_string(),
            "http://test.org".to_string(),
        ];
        let results = detect_known_formats(&examples);
        assert!(results.iter().any(|(_, d)| d == "URL (HTTP/HTTPS)"));
    }

    #[test]
    fn test_detect_semver() {
        let examples = vec![
            "1.0.0".to_string(),
            "2.3.4".to_string(),
            "10.20.30".to_string(),
        ];
        let results = detect_known_formats(&examples);
        assert!(results
            .iter()
            .any(|(_, d)| d == "Semantic version (SemVer)"));
    }

    #[test]
    fn test_detect_hex_color() {
        let examples = vec![
            "#ff0000".to_string(),
            "#00ff00".to_string(),
            "#abc".to_string(),
        ];
        let results = detect_known_formats(&examples);
        assert!(results.iter().any(|(_, d)| d == "Hex color code"));
    }

    #[test]
    fn test_detect_mac_address() {
        let examples = vec![
            "AA:BB:CC:DD:EE:FF".to_string(),
            "00:11:22:33:44:55".to_string(),
        ];
        let results = detect_known_formats(&examples);
        assert!(results.iter().any(|(_, d)| d == "MAC address"));
    }

    #[test]
    fn test_detect_email() {
        let examples = vec!["user@example.com".to_string(), "admin@test.org".to_string()];
        let results = detect_known_formats(&examples);
        assert!(results.iter().any(|(_, d)| d == "Email address"));
    }

    #[test]
    fn test_detect_iso_date() {
        let examples = vec!["2024-01-15".to_string(), "2025-12-31".to_string()];
        let results = detect_known_formats(&examples);
        assert!(results.iter().any(|(_, d)| d.contains("ISO 8601")));
    }

    #[test]
    fn test_recognize_ipv4_pattern() {
        let desc = recognize_pattern(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}");
        assert_eq!(desc, Some("IPv4 address".to_string()));
    }

    #[test]
    fn test_recognize_email_pattern() {
        let desc = recognize_pattern(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}");
        assert_eq!(desc, Some("Email address".to_string()));
    }

    #[test]
    fn test_recognize_unknown_pattern() {
        let desc = recognize_pattern(r"\w+");
        assert_eq!(desc, None);
    }
}
