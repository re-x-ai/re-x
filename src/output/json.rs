//! JSON output formatting
//!
//! JSON is the default output format, optimized for AI consumption.

use serde::Serialize;

/// Format a result as JSON
pub fn format_json<T: Serialize>(result: &T) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|e| {
        format!(
            r#"{{"error": true, "code": "SERIALIZATION_ERROR", "message": "{}"}}"#,
            e
        )
    })
}

/// Format a result as compact JSON (single line)
#[allow(dead_code)]
pub fn format_json_compact<T: Serialize>(result: &T) -> String {
    serde_json::to_string(result).unwrap_or_else(|e| {
        format!(
            r#"{{"error": true, "code": "SERIALIZATION_ERROR", "message": "{}"}}"#,
            e
        )
    })
}
