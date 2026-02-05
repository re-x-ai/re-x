//! Implementation of `re-x explain` command
//!
//! Breaks down a regex pattern into its component parts with descriptions.

use regex_syntax::ast::parse::Parser as AstParser;
use regex_syntax::ast::{self, Ast, ClassPerlKind, ClassUnicodeKind};

use super::templates::recognize_pattern;
use crate::output::{ExplainPart, ExplainResult};

/// Explain a regex pattern
pub fn explain_pattern(pattern: &str) -> Result<ExplainResult, String> {
    // Check for fancy-regex features first
    let fancy_features = super::engine::detect_fancy_features(pattern);

    if fancy_features.needs_fancy() {
        return explain_fancy_pattern(pattern, &fancy_features);
    }

    let ast = AstParser::new()
        .parse(pattern)
        .map_err(|e| format!("Failed to parse pattern: {}", e))?;

    let parts = explain_ast(&ast);
    let summary = generate_summary(pattern, &parts);

    Ok(ExplainResult {
        pattern: pattern.to_string(),
        parts,
        summary,
    })
}

/// Explain a pattern that uses fancy-regex features (lookahead, lookbehind, etc.)
fn explain_fancy_pattern(
    pattern: &str,
    features: &super::engine::FancyFeatures,
) -> Result<ExplainResult, String> {
    let mut parts = Vec::new();

    if features.lookahead {
        parts.push(ExplainPart {
            token: "(?=...) / (?!...)".to_string(),
            token_type: "lookahead".to_string(),
            desc: "Lookahead assertion: checks what follows without consuming characters"
                .to_string(),
            quantifier: None,
            group: None,
            children: None,
        });
    }
    if features.lookbehind {
        parts.push(ExplainPart {
            token: "(?<=...) / (?<!...)".to_string(),
            token_type: "lookbehind".to_string(),
            desc: "Lookbehind assertion: checks what precedes without consuming characters"
                .to_string(),
            quantifier: None,
            group: None,
            children: None,
        });
    }
    if features.backreference {
        parts.push(ExplainPart {
            token: r"\1, \2, ...".to_string(),
            token_type: "backreference".to_string(),
            desc: "Backreference: matches the same text as a previous capturing group".to_string(),
            quantifier: None,
            group: None,
            children: None,
        });
    }
    if features.atomic_group {
        parts.push(ExplainPart {
            token: "(?>...)".to_string(),
            token_type: "atomic_group".to_string(),
            desc: "Atomic group: prevents backtracking into the group once matched".to_string(),
            quantifier: None,
            group: None,
            children: None,
        });
    }

    let feature_desc = features.reason().unwrap_or_default();
    let summary = format!(
        "This pattern uses advanced features ({}) that require the fancy-regex engine",
        feature_desc
    );

    Ok(ExplainResult {
        pattern: pattern.to_string(),
        parts,
        summary,
    })
}

/// Recursively explain an AST node
fn explain_ast(ast: &Ast) -> Vec<ExplainPart> {
    match ast {
        Ast::Empty(_) => vec![],

        Ast::Flags(flags) => {
            vec![ExplainPart {
                token: format!("(?{})", flags_to_string(&flags.flags)),
                token_type: "flags".to_string(),
                desc: describe_flags(&flags.flags),
                quantifier: None,
                group: None,
                children: None,
            }]
        }

        Ast::Literal(lit) => {
            let c = lit.c;
            let desc = if c.is_ascii_alphanumeric() {
                format!("Literal '{}'", c)
            } else {
                format!("Literal '{}' (U+{:04X})", c, c as u32)
            };

            vec![ExplainPart {
                token: c.to_string(),
                token_type: "literal".to_string(),
                desc,
                quantifier: None,
                group: None,
                children: None,
            }]
        }

        Ast::Dot(_) => {
            vec![ExplainPart {
                token: ".".to_string(),
                token_type: "any_char".to_string(),
                desc: "Matches any character (except newline by default)".to_string(),
                quantifier: None,
                group: None,
                children: None,
            }]
        }

        Ast::Assertion(assertion) => {
            let (token, desc) = match assertion.kind {
                ast::AssertionKind::StartLine => ("^", "Start of line/string"),
                ast::AssertionKind::EndLine => ("$", "End of line/string"),
                ast::AssertionKind::StartText => (r"\A", "Start of text (absolute)"),
                ast::AssertionKind::EndText => (r"\z", "End of text (absolute)"),
                ast::AssertionKind::WordBoundary => (r"\b", "Word boundary"),
                ast::AssertionKind::NotWordBoundary => (r"\B", "Non-word boundary"),
                ast::AssertionKind::WordBoundaryStart => (r"\<", "Start of word"),
                ast::AssertionKind::WordBoundaryEnd => (r"\>", "End of word"),
                ast::AssertionKind::WordBoundaryStartAngle => (r"\<", "Start of word"),
                ast::AssertionKind::WordBoundaryEndAngle => (r"\>", "End of word"),
                ast::AssertionKind::WordBoundaryStartHalf => {
                    (r"\b{start}", "Start of word boundary")
                }
                ast::AssertionKind::WordBoundaryEndHalf => (r"\b{end}", "End of word boundary"),
            };

            vec![ExplainPart {
                token: token.to_string(),
                token_type: "anchor".to_string(),
                desc: desc.to_string(),
                quantifier: None,
                group: None,
                children: None,
            }]
        }

        Ast::ClassUnicode(class) => {
            let desc = match &class.kind {
                ClassUnicodeKind::Named(name) => format!("Unicode property: {}", name),
                ClassUnicodeKind::OneLetter(c) => describe_unicode_class(*c),
                ClassUnicodeKind::NamedValue { name, value, .. } => {
                    format!("Unicode {}={}", name, value)
                }
            };

            let kind_str = match &class.kind {
                ClassUnicodeKind::OneLetter(c) => c.to_string(),
                ClassUnicodeKind::Named(name) => name.clone(),
                ClassUnicodeKind::NamedValue { name, value, .. } => format!("{}={}", name, value),
            };
            let token = if class.negated {
                format!(r"\P{{{}}}", kind_str)
            } else {
                format!(r"\p{{{}}}", kind_str)
            };

            vec![ExplainPart {
                token,
                token_type: "unicode_class".to_string(),
                desc,
                quantifier: None,
                group: None,
                children: None,
            }]
        }

        Ast::ClassPerl(class) => {
            let (token, desc) = match class.kind {
                ClassPerlKind::Digit => {
                    if class.negated {
                        (r"\D", "Non-digit character")
                    } else {
                        (r"\d", "Digit character [0-9]")
                    }
                }
                ClassPerlKind::Space => {
                    if class.negated {
                        (r"\S", "Non-whitespace character")
                    } else {
                        (r"\s", "Whitespace character")
                    }
                }
                ClassPerlKind::Word => {
                    if class.negated {
                        (r"\W", "Non-word character")
                    } else {
                        (r"\w", "Word character [a-zA-Z0-9_]")
                    }
                }
            };

            vec![ExplainPart {
                token: token.to_string(),
                token_type: "perl_class".to_string(),
                desc: desc.to_string(),
                quantifier: None,
                group: None,
                children: None,
            }]
        }

        Ast::ClassBracketed(class) => {
            // Simplified handling of bracketed classes
            let original = format!("{}", ast);
            let negated = if class.negated { "not " } else { "" };

            vec![ExplainPart {
                token: original,
                token_type: "character_class".to_string(),
                desc: format!(
                    "Character class: matches {}one of the specified characters",
                    negated
                ),
                quantifier: None,
                group: None,
                children: None,
            }]
        }

        Ast::Repetition(rep) => {
            let mut child_parts = explain_ast(&rep.ast);

            let quantifier = match rep.op.kind {
                ast::RepetitionKind::ZeroOrOne => "?",
                ast::RepetitionKind::ZeroOrMore => "*",
                ast::RepetitionKind::OneOrMore => "+",
                ast::RepetitionKind::Range(ref range) => match range {
                    ast::RepetitionRange::Exactly(n) => {
                        return vec![ExplainPart {
                            token: format!("{}{{{}}}", rep.ast, n),
                            token_type: "repetition".to_string(),
                            desc: format!("Exactly {} of the preceding element", n),
                            quantifier: Some(format!("{{{}}}", n)),
                            group: None,
                            children: if child_parts.len() > 1 {
                                Some(child_parts)
                            } else {
                                None
                            },
                        }];
                    }
                    ast::RepetitionRange::AtLeast(n) => {
                        return vec![ExplainPart {
                            token: format!("{}{{{},}}", rep.ast, n),
                            token_type: "repetition".to_string(),
                            desc: format!("{} or more of the preceding element", n),
                            quantifier: Some(format!("{{{},}}", n)),
                            group: None,
                            children: if child_parts.len() > 1 {
                                Some(child_parts)
                            } else {
                                None
                            },
                        }];
                    }
                    ast::RepetitionRange::Bounded(m, n) => {
                        return vec![ExplainPart {
                            token: format!("{}{{{},{}}}", rep.ast, m, n),
                            token_type: "repetition".to_string(),
                            desc: format!("Between {} and {} of the preceding element", m, n),
                            quantifier: Some(format!("{{{},{}}}", m, n)),
                            group: None,
                            children: if child_parts.len() > 1 {
                                Some(child_parts)
                            } else {
                                None
                            },
                        }];
                    }
                },
            };

            let greedy = if rep.greedy { "" } else { " (non-greedy)" };
            let desc = match quantifier {
                "?" => format!("Zero or one{}", greedy),
                "*" => format!("Zero or more{}", greedy),
                "+" => format!("One or more{}", greedy),
                _ => quantifier.to_string(),
            };

            // If there's only one child, merge the quantifier into it
            if child_parts.len() == 1 {
                let mut part = child_parts.pop().unwrap();
                part.quantifier = Some(format!(
                    "{}{}",
                    quantifier,
                    if rep.greedy { "" } else { "?" }
                ));
                part.token = format!("{}{}", part.token, quantifier);
                if !rep.greedy {
                    part.token.push('?');
                }
                part.desc = format!("{} ({})", part.desc, desc);
                vec![part]
            } else {
                vec![ExplainPart {
                    token: format!("{}", ast),
                    token_type: "repetition".to_string(),
                    desc,
                    quantifier: Some(quantifier.to_string()),
                    group: None,
                    children: Some(child_parts),
                }]
            }
        }

        Ast::Group(group) => {
            let children = explain_ast(&group.ast);

            let (token_type, desc, group_num): (&str, String, Option<usize>) = match &group.kind {
                ast::GroupKind::CaptureIndex(index) => (
                    "capturing_group",
                    "Capturing group".to_string(),
                    Some(*index as usize),
                ),
                ast::GroupKind::NonCapturing(_) => (
                    "non_capturing_group",
                    "Non-capturing group".to_string(),
                    None,
                ),
                ast::GroupKind::CaptureName { name, .. } => {
                    ("named_group", format!("Named capture: {}", name.name), None)
                }
            };

            vec![ExplainPart {
                token: format!("{}", ast),
                token_type: token_type.to_string(),
                desc: desc.to_string(),
                quantifier: None,
                group: group_num,
                children: if children.is_empty() {
                    None
                } else {
                    Some(children)
                },
            }]
        }

        Ast::Alternation(alt) => {
            let branches: Vec<_> = alt
                .asts
                .iter()
                .map(|a| ExplainPart {
                    token: format!("{}", a),
                    token_type: "branch".to_string(),
                    desc: "Alternative branch".to_string(),
                    quantifier: None,
                    group: None,
                    children: Some(explain_ast(a)),
                })
                .collect();

            vec![ExplainPart {
                token: format!("{}", ast),
                token_type: "alternation".to_string(),
                desc: format!("Match one of {} alternatives", alt.asts.len()),
                quantifier: None,
                group: None,
                children: Some(branches),
            }]
        }

        Ast::Concat(concat) => concat.asts.iter().flat_map(explain_ast).collect(),
    }
}

/// Convert flags to string representation
fn flags_to_string(flags: &ast::Flags) -> String {
    let mut s = String::new();
    for item in &flags.items {
        match item.kind {
            ast::FlagsItemKind::Negation => s.push('-'),
            ast::FlagsItemKind::Flag(flag) => {
                s.push(match flag {
                    ast::Flag::CaseInsensitive => 'i',
                    ast::Flag::MultiLine => 'm',
                    ast::Flag::DotMatchesNewLine => 's',
                    ast::Flag::SwapGreed => 'U',
                    ast::Flag::Unicode => 'u',
                    ast::Flag::IgnoreWhitespace => 'x',
                    ast::Flag::CRLF => 'R',
                });
            }
        }
    }
    s
}

/// Describe flags
fn describe_flags(flags: &ast::Flags) -> String {
    let mut descs = Vec::new();
    for item in &flags.items {
        if let ast::FlagsItemKind::Flag(flag) = item.kind {
            descs.push(match flag {
                ast::Flag::CaseInsensitive => "case-insensitive",
                ast::Flag::MultiLine => "multi-line mode",
                ast::Flag::DotMatchesNewLine => "dot matches newline",
                ast::Flag::SwapGreed => "swap greedy/non-greedy",
                ast::Flag::Unicode => "unicode mode",
                ast::Flag::IgnoreWhitespace => "ignore whitespace",
                ast::Flag::CRLF => "CRLF mode",
            });
        }
    }
    format!("Enable {}", descs.join(", "))
}

/// Describe unicode class
fn describe_unicode_class(c: char) -> String {
    match c {
        'L' => "Unicode Letter".to_string(),
        'N' => "Unicode Number".to_string(),
        'P' => "Unicode Punctuation".to_string(),
        'S' => "Unicode Symbol".to_string(),
        'Z' => "Unicode Separator".to_string(),
        'C' => "Unicode Other/Control".to_string(),
        'M' => "Unicode Mark".to_string(),
        _ => format!("Unicode category {}", c),
    }
}

/// Generate a summary of the pattern
fn generate_summary(pattern: &str, parts: &[ExplainPart]) -> String {
    if parts.is_empty() {
        return "Empty pattern".to_string();
    }

    // Try semantic recognition first via known format templates
    if let Some(format_name) = recognize_pattern(pattern) {
        return format!("Matches {}", format_name_article(&format_name));
    }

    // Fall back to AST-based structural summary
    let mut fragments = Vec::new();
    let mut has_start_anchor = false;
    let mut has_end_anchor = false;

    for part in parts {
        match part.token_type.as_str() {
            "anchor" if part.token == "^" => has_start_anchor = true,
            "anchor" if part.token == "$" => has_end_anchor = true,
            "capturing_group" => {
                if let Some(children) = &part.children {
                    let child_desc = summarize_children(children);
                    fragments.push(format!("a captured {}", child_desc));
                } else {
                    fragments.push("a captured group".to_string());
                }
            }
            "alternation" => {
                if let Some(branches) = &part.children {
                    let branch_descs: Vec<String> = branches
                        .iter()
                        .map(|b| {
                            if let Some(children) = &b.children {
                                summarize_children(children)
                            } else {
                                b.token.clone()
                            }
                        })
                        .collect();
                    if branch_descs.len() <= 3 {
                        fragments.push(format!("either {}", branch_descs.join(" or ")));
                    } else {
                        fragments.push(format!("one of {} alternatives", branch_descs.len()));
                    }
                }
            }
            "perl_class" => {
                // Token may include quantifier suffix (e.g. "\w+" after merge)
                let base_token = part
                    .token
                    .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '\\');
                let class_desc = match base_token {
                    r"\d" => "digits",
                    r"\D" => "non-digits",
                    r"\w" => "word characters",
                    r"\W" => "non-word characters",
                    r"\s" => "whitespace",
                    r"\S" => "non-whitespace",
                    _ => "characters",
                };
                let quantified = if let Some(q) = &part.quantifier {
                    match q.as_str() {
                        "+" => format!("one or more {}", class_desc),
                        "*" => format!("zero or more {}", class_desc),
                        "?" => format!(
                            "an optional {}",
                            &class_desc[..class_desc.len().saturating_sub(1)]
                        ),
                        _ => format!("{} ({})", class_desc, q),
                    }
                } else {
                    format!("a {}", &class_desc[..class_desc.len().saturating_sub(1)])
                };
                fragments.push(quantified);
            }
            "literal" => {
                // Group consecutive literals
                fragments.push(format!("'{}'", part.token));
            }
            "any_char" => {
                if let Some(q) = &part.quantifier {
                    match q.as_str() {
                        "*" | "*?" => fragments.push("any text".to_string()),
                        "+" | "+?" => fragments.push("some text".to_string()),
                        _ => fragments.push("any character".to_string()),
                    }
                } else {
                    fragments.push("any character".to_string());
                }
            }
            "character_class" => {
                fragments.push(format!("characters matching {}", part.token));
            }
            "repetition" => {
                fragments.push(part.desc.clone());
            }
            _ => {}
        }
    }

    if fragments.is_empty() {
        return "Matches the specified pattern".to_string();
    }

    // Build final summary
    let mut summary = String::from("Matches ");
    summary.push_str(&fragments.join(", then "));

    if has_start_anchor && has_end_anchor {
        summary.push_str(" (full line match)");
    } else if has_start_anchor {
        summary.push_str(" (at start of line)");
    } else if has_end_anchor {
        summary.push_str(" (at end of line)");
    }

    summary
}

/// Add appropriate article before format name
fn format_name_article(name: &str) -> String {
    let vowel_start = matches!(
        name.chars().next(),
        Some('A' | 'E' | 'I' | 'O' | 'U' | 'a' | 'e' | 'i' | 'o' | 'u')
    );
    if vowel_start {
        format!("an {}", name)
    } else {
        format!("a {}", name)
    }
}

/// Summarize a list of child parts into a brief description
fn summarize_children(children: &[ExplainPart]) -> String {
    let descs: Vec<&str> = children
        .iter()
        .filter_map(|c| match c.token_type.as_str() {
            "perl_class" => match c.token.as_str() {
                r"\d" => Some("digits"),
                r"\w" => Some("word chars"),
                r"\s" => Some("whitespace"),
                _ => Some("characters"),
            },
            "literal" => Some("literal"),
            "any_char" => Some("any char"),
            "character_class" => Some("char class"),
            _ => None,
        })
        .collect();

    if descs.is_empty() {
        "group".to_string()
    } else {
        descs.join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_simple() {
        let result = explain_pattern(r"\d+").unwrap();
        assert!(!result.parts.is_empty());
    }

    #[test]
    fn test_explain_with_groups() {
        let result = explain_pattern(r"(\d+)-(\d+)").unwrap();
        assert!(!result.parts.is_empty());
    }

    #[test]
    fn test_explain_alternation() {
        let result = explain_pattern(r"cat|dog").unwrap();
        assert!(result.parts.iter().any(|p| p.token_type == "alternation"));
    }
}
