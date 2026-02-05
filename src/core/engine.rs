//! Regex engine selection and compilation
//!
//! Automatically chooses between `regex` (fast, linear time) and
//! `fancy-regex` (full features, backtracking) based on pattern analysis.

use std::sync::LazyLock;

use thiserror::Error;

static BACKREFERENCE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\\[1-9]").expect("BUG: backreference detection pattern is invalid")
});

/// Engine types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    /// Standard regex crate (linear time guaranteed)
    Regex,
    /// Fancy-regex (supports lookahead, lookbehind, backreferences)
    FancyRegex,
}

impl std::fmt::Display for EngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineType::Regex => write!(f, "regex"),
            EngineType::FancyRegex => write!(f, "fancy-regex"),
        }
    }
}

/// Errors that can occur during engine operations
#[allow(dead_code, clippy::result_large_err)]
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Invalid regex pattern: {0}")]
    InvalidPattern(String),

    #[error("Pattern requires fancy-regex engine: {0}")]
    RequiresFancy(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Fancy-regex error: {0}")]
    FancyRegexError(#[from] fancy_regex::Error),
}

/// Features detected in a pattern that require fancy-regex
#[derive(Debug, Default)]
pub struct FancyFeatures {
    pub lookahead: bool,
    pub lookbehind: bool,
    pub backreference: bool,
    pub atomic_group: bool,
}

impl FancyFeatures {
    /// Returns true if any fancy feature is detected
    pub fn needs_fancy(&self) -> bool {
        self.lookahead || self.lookbehind || self.backreference || self.atomic_group
    }

    /// Get a description of why fancy-regex is needed
    pub fn reason(&self) -> Option<String> {
        let mut reasons = Vec::new();
        if self.lookahead {
            reasons.push("lookahead assertion");
        }
        if self.lookbehind {
            reasons.push("lookbehind assertion");
        }
        if self.backreference {
            reasons.push("backreference");
        }
        if self.atomic_group {
            reasons.push("atomic group");
        }

        if reasons.is_empty() {
            None
        } else {
            Some(format!("Pattern uses {}", reasons.join(", ")))
        }
    }
}

/// Detect which engine features are used in a pattern
pub fn detect_fancy_features(pattern: &str) -> FancyFeatures {
    let mut features = FancyFeatures::default();

    // Note: regex_syntax's AST parser cannot parse lookahead, lookbehind,
    // backreferences, or atomic groups — they are fancy-regex extensions.
    // Detection relies on string scanning below.
    if pattern.contains("(?=") || pattern.contains("(?!") {
        features.lookahead = true;
    }
    if pattern.contains("(?<=") || pattern.contains("(?<!") {
        features.lookbehind = true;
    }
    if pattern.contains("(?>") {
        features.atomic_group = true;
    }

    // Check for backreferences (\1, \2, etc.)
    if BACKREFERENCE_RE.is_match(pattern) {
        features.backreference = true;
    }

    features
}

/// Select the appropriate engine for a pattern
pub fn select_engine(pattern: &str) -> (EngineType, FancyFeatures) {
    let features = detect_fancy_features(pattern);
    let engine = if features.needs_fancy() {
        EngineType::FancyRegex
    } else {
        EngineType::Regex
    };
    (engine, features)
}

/// A compiled regex that can use either engine
pub enum CompiledRegex {
    Regex(regex::Regex),
    FancyRegex(fancy_regex::Regex),
}

#[allow(dead_code, clippy::result_large_err)]
impl CompiledRegex {
    /// Compile a pattern with automatic engine selection
    pub fn new(pattern: &str) -> Result<(Self, EngineType), EngineError> {
        let (engine, _features) = select_engine(pattern);

        match engine {
            EngineType::Regex => {
                match regex::Regex::new(pattern) {
                    Ok(re) => Ok((CompiledRegex::Regex(re), EngineType::Regex)),
                    Err(_) => {
                        // Fall back to fancy-regex if standard regex fails
                        let re = fancy_regex::Regex::new(pattern)?;
                        Ok((CompiledRegex::FancyRegex(re), EngineType::FancyRegex))
                    }
                }
            }
            EngineType::FancyRegex => {
                let re = fancy_regex::Regex::new(pattern)?;
                Ok((CompiledRegex::FancyRegex(re), EngineType::FancyRegex))
            }
        }
    }

    /// Compile with a specific engine
    pub fn with_engine(pattern: &str, engine: EngineType) -> Result<Self, EngineError> {
        match engine {
            EngineType::Regex => {
                let re = regex::Regex::new(pattern)?;
                Ok(CompiledRegex::Regex(re))
            }
            EngineType::FancyRegex => {
                let re = fancy_regex::Regex::new(pattern)?;
                Ok(CompiledRegex::FancyRegex(re))
            }
        }
    }

    /// Check if the pattern matches anywhere in the text
    pub fn is_match(&self, text: &str) -> Result<bool, EngineError> {
        match self {
            CompiledRegex::Regex(re) => Ok(re.is_match(text)),
            CompiledRegex::FancyRegex(re) => re.is_match(text).map_err(EngineError::from),
        }
    }

    /// Find the first match
    pub fn find(&self, text: &str) -> Result<Option<(usize, usize)>, EngineError> {
        match self {
            CompiledRegex::Regex(re) => Ok(re.find(text).map(|m| (m.start(), m.end()))),
            CompiledRegex::FancyRegex(re) => re
                .find(text)
                .map(|opt| opt.map(|m| (m.start(), m.end())))
                .map_err(EngineError::from),
        }
    }

    /// Get the engine type
    pub fn engine_type(&self) -> EngineType {
        match self {
            CompiledRegex::Regex(_) => EngineType::Regex,
            CompiledRegex::FancyRegex(_) => EngineType::FancyRegex,
        }
    }
}

/// Detect whether a pattern contains any capturing groups by walking the regex AST.
/// Falls back to `true` (conservative — always collect captures) for patterns that
/// `regex_syntax` cannot parse (e.g., fancy-regex-only features like lookahead).
pub fn has_capturing_groups(pattern: &str) -> bool {
    use regex_syntax::ast::parse::Parser as AstParser;
    use regex_syntax::ast::{Ast, GroupKind};

    fn walk(ast: &Ast) -> bool {
        match ast {
            Ast::Group(group) => match &group.kind {
                GroupKind::CaptureIndex(_) | GroupKind::CaptureName { .. } => true,
                GroupKind::NonCapturing(_) => walk(&group.ast),
            },
            Ast::Concat(concat) => concat.asts.iter().any(walk),
            Ast::Alternation(alt) => alt.asts.iter().any(walk),
            Ast::Repetition(rep) => walk(&rep.ast),
            _ => false,
        }
    }

    match AstParser::new().parse(pattern) {
        Ok(ast) => walk(&ast),
        Err(_) => true, // fancy-regex pattern — conservatively assume captures exist
    }
}

/// Try to compile with standard regex crate
pub fn try_regex_crate(pattern: &str) -> Result<regex::Regex, regex::Error> {
    regex::Regex::new(pattern)
}

/// Try to compile with fancy-regex
#[allow(clippy::result_large_err)]
pub fn try_fancy_regex(pattern: &str) -> Result<fancy_regex::Regex, fancy_regex::Error> {
    fancy_regex::Regex::new(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pattern_uses_regex() {
        let (engine, _) = select_engine(r"\d+");
        assert_eq!(engine, EngineType::Regex);
    }

    #[test]
    fn test_lookahead_uses_fancy() {
        let (engine, features) = select_engine(r"foo(?=bar)");
        assert_eq!(engine, EngineType::FancyRegex);
        assert!(features.lookahead);
    }

    #[test]
    fn test_lookbehind_uses_fancy() {
        let (engine, features) = select_engine(r"(?<=foo)bar");
        assert_eq!(engine, EngineType::FancyRegex);
        assert!(features.lookbehind);
    }

    #[test]
    fn test_backreference_uses_fancy() {
        let (engine, features) = select_engine(r"(\w+)\s+\1");
        assert_eq!(engine, EngineType::FancyRegex);
        assert!(features.backreference);
    }

    #[test]
    fn test_compile_simple() {
        let (re, engine) = CompiledRegex::new(r"\d+").unwrap();
        assert_eq!(engine, EngineType::Regex);
        assert!(re.is_match("123").unwrap());
    }

    #[test]
    fn test_compile_fancy() {
        let (re, engine) = CompiledRegex::new(r"foo(?=bar)").unwrap();
        assert_eq!(engine, EngineType::FancyRegex);
        assert!(re.is_match("foobar").unwrap());
        assert!(!re.is_match("foobaz").unwrap());
    }
}
