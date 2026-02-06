//! Implementation of `re-x validate` command
//!
//! Validates regex syntax and checks cross-language portability.

use regex_syntax::ast;
use regex_syntax::ast::parse::Parser as AstParser;

use super::engine::{select_engine, try_fancy_regex, try_regex_crate};
use super::portability::check_portability;
use crate::output::{ValidateResult, ValidationError};

/// Validate a regex pattern
pub fn validate_pattern(pattern: &str) -> ValidateResult {
    // First, try to parse with regex-syntax for detailed error messages
    let ast_result = AstParser::new().parse(pattern);

    // Check if it's valid with standard regex
    let regex_result = try_regex_crate(pattern);

    // Check if it's valid with fancy-regex
    let fancy_result = try_fancy_regex(pattern);

    // Determine validity and errors
    match (&regex_result, &fancy_result) {
        (Ok(_), _) => {
            // Valid with standard regex
            let portability = check_portability(pattern);

            ValidateResult {
                valid: true,
                error: None,
                engine_required: Some("regex".to_string()),
                reason: None,
                portability: Some(portability),
                suggestion: None,
            }
        }
        (Err(_), Ok(_)) => {
            // Only valid with fancy-regex
            let (_, features) = select_engine(pattern);
            let portability = check_portability(pattern);

            ValidateResult {
                valid: true,
                error: None,
                engine_required: Some("fancy-regex".to_string()),
                reason: features.reason(),
                portability: Some(portability),
                suggestion: None,
            }
        }
        (Err(regex_err), Err(fancy_err)) => {
            // Invalid with both engines
            let (error, suggestion) = if let Err(ast_err) = ast_result {
                // Use AST parser error for better messages
                parse_ast_error(&ast_err)
            } else {
                // Fall back to regex error
                parse_regex_error(regex_err, fancy_err)
            };

            ValidateResult {
                valid: false,
                error: Some(error),
                engine_required: None,
                reason: None,
                portability: None,
                suggestion,
            }
        }
    }
}

/// Validate a pattern for a specific target language
pub fn validate_for_language(pattern: &str, target: &str) -> ValidateResult {
    let mut result = validate_pattern(pattern);

    if result.valid {
        let Some(portability) = result.portability.as_ref() else {
            return result;
        };
        let compatible = match target.to_lowercase().as_str() {
            "rust" | "rust_regex" => portability.rust_regex,
            "pcre" | "pcre2" => portability.pcre2,
            "js" | "javascript" => portability.javascript,
            "python" | "python_re" => portability.python_re,
            "python_regex" | "regex" => portability.python_regex,
            "go" | "go_regexp" | "golang" => portability.go_regexp,
            "java" => portability.java.unwrap_or(true),
            "dotnet" | "csharp" | "c#" | ".net" => portability.dotnet,
            "ruby" | "rb" => portability.ruby,
            _ => true,
        };

        if !compatible {
            result.error = Some(ValidationError {
                kind: "incompatible".to_string(),
                position: None,
                message: format!("Pattern is not compatible with {}", target),
            });
            result.suggestion = suggest_compatible_alternative(pattern, target);
        }
    }

    result
}

/// Parse AST error into ValidationError
fn parse_ast_error(err: &ast::Error) -> (ValidationError, Option<String>) {
    let kind = match err.kind() {
        ast::ErrorKind::GroupUnclosed => "unclosed_group",
        ast::ErrorKind::GroupUnopened => "unopened_group",
        ast::ErrorKind::EscapeUnexpectedEof => "incomplete_escape",
        ast::ErrorKind::ClassUnclosed => "unclosed_class",
        ast::ErrorKind::RepetitionMissing => "missing_repetition_target",
        ast::ErrorKind::RepetitionCountUnclosed => "unclosed_repetition",
        _ => "syntax_error",
    };

    let position = err.span().start.offset;
    let message = err.to_string();

    let suggestion = suggest_fix_for_error(kind, &message);

    (
        ValidationError {
            kind: kind.to_string(),
            position: Some(position),
            message,
        },
        suggestion,
    )
}

/// Parse regex crate error
fn parse_regex_error(
    regex_err: &regex::Error,
    _fancy_err: &fancy_regex::Error,
) -> (ValidationError, Option<String>) {
    let message = regex_err.to_string();

    // Try to extract error type from message
    let kind = if message.contains("unclosed") {
        "unclosed_group"
    } else if message.contains("invalid") {
        "invalid_syntax"
    } else if message.contains("quantifier") {
        "invalid_quantifier"
    } else {
        "syntax_error"
    };

    (
        ValidationError {
            kind: kind.to_string(),
            position: None,
            message,
        },
        None,
    )
}

/// Suggest a fix based on error type
fn suggest_fix_for_error(kind: &str, message: &str) -> Option<String> {
    match kind {
        "unclosed_group" => Some("Add closing ')' to complete the group".to_string()),
        "unopened_group" => Some("Remove extra ')' or add opening '('".to_string()),
        "incomplete_escape" => {
            Some("Complete the escape sequence or escape the backslash with '\\\\'".to_string())
        }
        "unclosed_class" => Some("Add closing ']' to complete the character class".to_string()),
        "missing_repetition_target" => {
            Some("Add a character or group before the quantifier".to_string())
        }
        "unclosed_repetition" => Some("Add closing '}' to complete the repetition".to_string()),
        _ => {
            // Try to infer from message
            if message.contains("nothing to repeat") {
                Some("Add a character or group before the quantifier".to_string())
            } else {
                None
            }
        }
    }
}

/// Suggest compatible alternative for a target language
fn suggest_compatible_alternative(pattern: &str, target: &str) -> Option<String> {
    match target.to_lowercase().as_str() {
        "rust" | "rust_regex" | "go" | "go_regexp" | "golang" => {
            // These don't support lookahead/lookbehind/backreferences
            if pattern.contains("(?=") || pattern.contains("(?!") {
                Some("Consider removing lookahead assertions - Rust regex and Go regexp don't support them".to_string())
            } else if pattern.contains("(?<=") || pattern.contains("(?<!") {
                Some("Consider removing lookbehind assertions - Rust regex and Go regexp don't support them".to_string())
            } else if pattern.contains(r"\1") || pattern.contains(r"\2") {
                Some("Consider removing backreferences - Rust regex and Go regexp don't support them".to_string())
            } else {
                None
            }
        }
        "javascript" | "js" => {
            // JS doesn't support variable-length lookbehind
            if pattern.contains("(?<=") && pattern.contains('+') {
                Some("JavaScript doesn't support variable-length lookbehind - use fixed-length pattern".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_simple_pattern() {
        let result = validate_pattern(r"\d+");
        assert!(result.valid);
        assert_eq!(result.engine_required, Some("regex".to_string()));
    }

    #[test]
    fn test_valid_fancy_pattern() {
        let result = validate_pattern(r"foo(?=bar)");
        assert!(result.valid);
        assert_eq!(result.engine_required, Some("fancy-regex".to_string()));
    }

    #[test]
    fn test_invalid_pattern() {
        let result = validate_pattern(r"(\d+");
        assert!(!result.valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_portability_check() {
        let result = validate_pattern(r"(\w+)\s+\1");
        assert!(result.valid);
        let portability = result.portability.unwrap();
        assert!(!portability.rust_regex);
        assert!(portability.javascript);
    }
}
