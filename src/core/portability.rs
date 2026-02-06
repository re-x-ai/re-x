//! Cross-language regex portability checking
//!
//! Uses AST-based analysis for standard regex patterns (accurate),
//! with string-based fallback for fancy-regex patterns.

use std::sync::LazyLock;

use crate::output::Portability;

static LOOKBEHIND_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\(\?<[=!][^)]*[+*?][^)]*\)")
        .expect("BUG: variable lookbehind detection pattern is invalid")
});

static BACKREF_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\\[1-9]").expect("BUG: backreference detection pattern is invalid")
});

static INLINE_FLAGS_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\(\?[imsx]+\)").expect("BUG: inline flags detection pattern is invalid")
});

static SUBROUTINE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\\g<[^>]+>").expect("BUG: subroutine detection pattern is invalid")
});

/// Feature flags for portability checking
#[derive(Debug, Default)]
pub struct PatternFeatures {
    // Anchors
    pub start_anchor: bool,      // ^
    pub end_anchor: bool,        // $
    pub word_boundary: bool,     // \b
    pub non_word_boundary: bool, // \B

    // Character classes
    pub unicode_classes: bool, // \p{...}
    pub negated_unicode: bool, // \P{...}
    pub posix_classes: bool,   // [:alpha:]

    // Assertions
    pub lookahead: bool,           // (?=...) (?!...)
    pub lookbehind: bool,          // (?<=...) (?<!...)
    pub variable_lookbehind: bool, // (?<=a+) - variable length

    // Groups
    pub named_capture: bool, // (?P<name>...) or (?<name>...)
    pub non_capturing: bool, // (?:...)
    pub atomic_group: bool,  // (?>...)
    pub backreference: bool, // \1, \2

    // Quantifiers
    pub possessive: bool, // a++, a*+

    // Flags
    pub inline_flags: bool, // (?i), (?m), etc.

    // Special
    pub conditional: bool, // (?(1)then|else)
    pub recursion: bool,   // (?R), (?1)
    pub subroutine: bool,  // \g<name>
}

impl PatternFeatures {
    /// Analyze a pattern and extract features.
    ///
    /// Uses AST-based analysis when `regex_syntax` can parse the pattern (standard regex),
    /// and falls back to string-based heuristics for fancy-regex patterns.
    pub fn analyze(pattern: &str) -> Self {
        use regex_syntax::ast::parse::Parser as AstParser;

        match AstParser::new().parse(pattern) {
            Ok(ast) => Self::analyze_from_ast(&ast),
            Err(_) => Self::analyze_from_string(pattern),
        }
    }

    /// AST-based analysis for standard regex patterns (no false positives)
    fn analyze_from_ast(ast: &regex_syntax::ast::Ast) -> Self {
        let mut features = Self::default();
        walk_ast(ast, &mut features);
        // Fancy-only features are always false in AST path:
        // lookahead, lookbehind, backreference, atomic_group,
        // possessive, conditional, recursion, subroutine
        features
    }

    /// String-based fallback for fancy-regex patterns.
    /// Only runs when regex_syntax cannot parse the pattern, meaning
    /// the pattern genuinely uses fancy features (lower false-positive risk).
    fn analyze_from_string(pattern: &str) -> Self {
        let lookbehind = pattern.contains("(?<=") || pattern.contains("(?<!");
        Self {
            start_anchor: pattern.starts_with('^')
                || pattern.contains("(?m)") && pattern.contains('^'),
            end_anchor: pattern.ends_with('$') || pattern.contains("(?m)") && pattern.contains('$'),
            word_boundary: pattern.contains(r"\b"),
            non_word_boundary: pattern.contains(r"\B"),
            unicode_classes: pattern.contains(r"\p{") || pattern.contains(r"\P{"),
            negated_unicode: pattern.contains(r"\P{"),
            posix_classes: pattern.contains("[:") && pattern.contains(":]"),
            lookahead: pattern.contains("(?=") || pattern.contains("(?!"),
            lookbehind,
            variable_lookbehind: lookbehind && LOOKBEHIND_RE.is_match(pattern),
            named_capture: pattern.contains("(?P<")
                || (pattern.contains("(?<")
                    && !pattern.contains("(?<=")
                    && !pattern.contains("(?<!")),
            non_capturing: pattern.contains("(?:"),
            atomic_group: pattern.contains("(?>"),
            backreference: BACKREF_RE.is_match(pattern),
            possessive: pattern.contains("++")
                || pattern.contains("*+")
                || pattern.contains("?+")
                || pattern.contains("}+"),
            inline_flags: INLINE_FLAGS_RE.is_match(pattern),
            conditional: pattern.contains("(?("),
            recursion: pattern.contains("(?R)") || pattern.contains("(?0)"),
            subroutine: SUBROUTINE_RE.is_match(pattern),
        }
    }
}

/// Recursively walk the AST to detect features
fn walk_ast(ast: &regex_syntax::ast::Ast, features: &mut PatternFeatures) {
    use regex_syntax::ast::{AssertionKind, Ast, GroupKind};

    match ast {
        Ast::Assertion(a) => match a.kind {
            AssertionKind::StartLine => features.start_anchor = true,
            AssertionKind::EndLine => features.end_anchor = true,
            AssertionKind::WordBoundary => features.word_boundary = true,
            AssertionKind::NotWordBoundary => features.non_word_boundary = true,
            _ => {}
        },
        Ast::ClassUnicode(c) => {
            features.unicode_classes = true;
            if c.negated {
                features.negated_unicode = true;
            }
        }
        Ast::ClassBracketed(c) => {
            walk_class_set(&c.kind, features);
        }
        Ast::Group(g) => {
            match &g.kind {
                GroupKind::CaptureName { .. } => features.named_capture = true,
                GroupKind::NonCapturing(_) => features.non_capturing = true,
                GroupKind::CaptureIndex(_) => {}
            }
            walk_ast(&g.ast, features);
        }
        Ast::Flags(_) => {
            features.inline_flags = true;
        }
        Ast::Concat(c) => {
            for child in &c.asts {
                walk_ast(child, features);
            }
        }
        Ast::Alternation(a) => {
            for child in &a.asts {
                walk_ast(child, features);
            }
        }
        Ast::Repetition(r) => {
            walk_ast(&r.ast, features);
        }
        _ => {}
    }
}

/// Walk a ClassSet to detect POSIX/ASCII classes and Unicode classes
fn walk_class_set(set: &regex_syntax::ast::ClassSet, features: &mut PatternFeatures) {
    use regex_syntax::ast::ClassSet;

    match set {
        ClassSet::Item(item) => walk_class_set_item(item, features),
        ClassSet::BinaryOp(op) => {
            walk_class_set(&op.lhs, features);
            walk_class_set(&op.rhs, features);
        }
    }
}

/// Walk a ClassSetItem to detect specific class types
fn walk_class_set_item(item: &regex_syntax::ast::ClassSetItem, features: &mut PatternFeatures) {
    use regex_syntax::ast::ClassSetItem;

    match item {
        ClassSetItem::Ascii(_) => {
            features.posix_classes = true;
        }
        ClassSetItem::Unicode(c) => {
            features.unicode_classes = true;
            if c.negated {
                features.negated_unicode = true;
            }
        }
        ClassSetItem::Bracketed(b) => {
            walk_class_set(&b.kind, features);
        }
        ClassSetItem::Union(u) => {
            for item in &u.items {
                walk_class_set_item(item, features);
            }
        }
        _ => {}
    }
}

/// Check portability to various languages/engines
pub fn check_portability(pattern: &str) -> Portability {
    let features = PatternFeatures::analyze(pattern);

    Portability {
        rust_regex: is_rust_regex_compatible(&features),
        pcre2: is_pcre2_compatible(&features),
        javascript: is_javascript_compatible(&features),
        python_re: is_python_re_compatible(&features),
        python_regex: is_python_regex_compatible(&features),
        go_regexp: is_go_regexp_compatible(&features),
        java: Some(is_java_compatible(&features)),
        dotnet: is_dotnet_compatible(&features),
        ruby: is_ruby_compatible(&features),
    }
}

/// Rust regex crate compatibility
fn is_rust_regex_compatible(features: &PatternFeatures) -> bool {
    !features.lookahead
        && !features.lookbehind
        && !features.backreference
        && !features.atomic_group
        && !features.possessive
        && !features.conditional
        && !features.recursion
        && !features.subroutine
}

/// PCRE2 compatibility (most features supported)
fn is_pcre2_compatible(features: &PatternFeatures) -> bool {
    // PCRE2 supports almost everything
    !features.posix_classes // PCRE uses different syntax for POSIX
}

/// JavaScript RegExp compatibility
/// Note: Unicode property classes (\p{...}) are supported with /u flag since ES2018
fn is_javascript_compatible(features: &PatternFeatures) -> bool {
    !features.variable_lookbehind // JS lookbehind must be fixed-length
        && !features.atomic_group
        && !features.possessive
        && !features.conditional
        && !features.recursion
        && !features.subroutine
        && !features.posix_classes
}

/// Python re module compatibility
fn is_python_re_compatible(features: &PatternFeatures) -> bool {
    !features.atomic_group
        && !features.possessive
        && !features.recursion
        && !features.subroutine
        && !features.posix_classes
}

/// Python regex module compatibility (third-party, more features)
fn is_python_regex_compatible(features: &PatternFeatures) -> bool {
    // Python regex module supports nearly everything except POSIX bracket classes
    !features.posix_classes
}

/// Go regexp package compatibility (RE2-based)
fn is_go_regexp_compatible(features: &PatternFeatures) -> bool {
    // Go uses RE2, similar to Rust regex
    !features.lookahead
        && !features.lookbehind
        && !features.backreference
        && !features.atomic_group
        && !features.possessive
        && !features.conditional
        && !features.recursion
        && !features.subroutine
}

/// Java java.util.regex compatibility
fn is_java_compatible(features: &PatternFeatures) -> bool {
    !features.recursion && !features.subroutine && !features.posix_classes
}

/// .NET System.Text.RegularExpressions compatibility
/// Supports: lookahead, lookbehind (variable-length), backreferences, atomic groups, conditionals
/// Does NOT support: recursion, subroutines, possessive quantifiers (pre-.NET 7), POSIX classes
fn is_dotnet_compatible(features: &PatternFeatures) -> bool {
    !features.recursion
        && !features.subroutine
        && !features.possessive
        && !features.posix_classes
}

/// Ruby (Oniguruma/Onigmo) compatibility
/// Supports: lookahead, lookbehind, backreferences, atomic groups, possessive, POSIX classes, subroutines
/// Does NOT support: PCRE-style recursion (?R), conditionals (?(1)...|...)
fn is_ruby_compatible(features: &PatternFeatures) -> bool {
    !features.conditional && !features.recursion
}

/// Get a human-readable explanation of compatibility issues
#[allow(dead_code)]
pub fn explain_compatibility(pattern: &str) -> Vec<String> {
    let features = PatternFeatures::analyze(pattern);
    let mut issues = Vec::new();

    if features.lookahead {
        issues.push(
            "Lookahead assertions ((?=...) (?!...)) are not supported in Rust regex or Go regexp"
                .to_string(),
        );
    }
    if features.lookbehind {
        issues.push("Lookbehind assertions ((?<=...) (?<!...)) are not supported in Rust regex or Go regexp".to_string());
    }
    if features.variable_lookbehind {
        issues.push("Variable-length lookbehind is not supported in JavaScript".to_string());
    }
    if features.backreference {
        issues.push(
            "Backreferences (\\1, \\2) are not supported in Rust regex or Go regexp".to_string(),
        );
    }
    if features.atomic_group {
        issues.push(
            "Atomic groups (?>) are not supported in Rust regex, Go, JavaScript, or Python re"
                .to_string(),
        );
    }
    if features.possessive {
        issues.push("Possessive quantifiers (++, *+) are not supported in Rust regex, Go, JavaScript, or Python re".to_string());
    }
    if features.conditional {
        issues.push(
            "Conditional patterns (?(1)...) are only supported in PCRE and Python regex"
                .to_string(),
        );
    }
    if features.recursion {
        issues.push("Recursion (?R) is only supported in PCRE and Python regex".to_string());
    }
    if features.unicode_classes {
        issues.push("Unicode property classes (\\p{...}) require special handling in JavaScript (needs /u flag)".to_string());
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pattern_portable() {
        let portability = check_portability(r"\d+");
        assert!(portability.rust_regex);
        assert!(portability.javascript);
        assert!(portability.go_regexp);
        assert!(portability.dotnet);
        assert!(portability.ruby);
    }

    #[test]
    fn test_lookahead_not_portable_to_rust() {
        let portability = check_portability(r"foo(?=bar)");
        assert!(!portability.rust_regex);
        assert!(portability.javascript);
        assert!(!portability.go_regexp);
        assert!(portability.dotnet);
        assert!(portability.ruby);
    }

    #[test]
    fn test_backreference_limited_portability() {
        let portability = check_portability(r"(\w+)\s+\1");
        assert!(!portability.rust_regex);
        assert!(portability.javascript);
        assert!(!portability.go_regexp);
        assert!(portability.dotnet);
        assert!(portability.ruby);
    }

    #[test]
    fn test_dotnet_blocks_possessive() {
        // Possessive quantifiers are not supported in .NET (pre-.NET 7)
        // Use lookahead to force string-based fallback where possessive is detected
        let portability = check_portability(r"(?=.)a++");
        assert!(!portability.dotnet);
        assert!(portability.ruby); // Ruby (Oniguruma) supports possessive
    }

    #[test]
    fn test_ruby_blocks_conditional() {
        // Ruby doesn't support PCRE-style conditionals
        let portability = check_portability(r"(?(1)a|b)");
        assert!(!portability.ruby);
        assert!(portability.dotnet); // .NET supports conditionals
    }

    // --- AST accuracy tests (false-positive prevention) ---

    #[test]
    fn test_escaped_paren_not_detected_as_lookahead() {
        // \(?=foo has an escaped paren, not a lookahead
        let features = PatternFeatures::analyze(r"\(?=foo");
        assert!(!features.lookahead);
    }

    #[test]
    fn test_char_class_not_detected_as_lookahead() {
        // [(?=] is a character class containing (, ?, =
        let features = PatternFeatures::analyze(r"[(?=]");
        assert!(!features.lookahead);
    }

    #[test]
    fn test_actual_lookahead_detected() {
        // Real lookahead — falls to string-based path
        let features = PatternFeatures::analyze(r"foo(?=bar)");
        assert!(features.lookahead);
    }

    #[test]
    fn test_unicode_class_detected_from_ast() {
        let features = PatternFeatures::analyze(r"\p{L}+");
        assert!(features.unicode_classes);
    }

    #[test]
    fn test_named_capture_detected_from_ast() {
        let features = PatternFeatures::analyze(r"(?P<name>\w+)");
        assert!(features.named_capture);
    }

    #[test]
    fn test_inline_flags_detected_from_ast() {
        let features = PatternFeatures::analyze(r"(?i)hello");
        assert!(features.inline_flags);
    }

    #[test]
    fn test_word_boundary_detected_from_ast() {
        let features = PatternFeatures::analyze(r"\bword\b");
        assert!(features.word_boundary);
    }
}
