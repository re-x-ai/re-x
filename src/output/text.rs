//! Human-friendly text output formatting
//!
//! Used when --format text is specified.

use super::types::*;

/// Format TestResult as human-readable text
pub fn format_test_result(result: &TestResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Pattern: {}\n", result.pattern));
    output.push_str(&format!(
        "Engine:  {} ({})\n",
        result.engine,
        if result.engine == "regex" {
            "linear time"
        } else {
            "backtracking"
        }
    ));
    output.push('\n');

    if result.matched {
        for (i, m) in result.matches.iter().enumerate() {
            output.push_str(&format!(
                "Match {}: \"{}\" [{}..{}]\n",
                i + 1,
                m.text,
                m.start,
                m.end
            ));

            for cap in &m.captures {
                let name_str = cap
                    .name
                    .as_ref()
                    .map(|n| format!(" ({})", n))
                    .unwrap_or_default();
                output.push_str(&format!(
                    "  Group {}{}: \"{}\" [{}..{}]\n",
                    cap.group, name_str, cap.text, cap.start, cap.end
                ));
            }
        }
        output.push('\n');
        output.push_str(&format!(
            "{} match{} found in {}μs\n",
            result.match_count,
            if result.match_count == 1 { "" } else { "es" },
            result.elapsed_us
        ));
    } else {
        output.push_str("No matches found\n");
    }

    output
}

/// Format ReplaceResult as human-readable text
pub fn format_replace_result(result: &ReplaceResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Pattern:     {}\n", result.pattern));
    output.push_str(&format!("Replacement: {}\n", result.replacement));
    output.push('\n');
    output.push_str(&format!("Original: {}\n", result.original));
    output.push_str(&format!("Result:   {}\n", result.result));
    output.push('\n');
    output.push_str(&format!(
        "{} replacement{} made\n",
        result.replacements_made,
        if result.replacements_made == 1 {
            ""
        } else {
            "s"
        }
    ));

    output
}

/// Format ValidateResult as human-readable text
pub fn format_validate_result(result: &ValidateResult) -> String {
    let mut output = String::new();

    if result.valid {
        output.push_str("✓ Pattern is valid\n");

        if let Some(ref engine) = result.engine_required {
            output.push_str(&format!("\nEngine required: {}\n", engine));
        }

        if let Some(ref reason) = result.reason {
            output.push_str(&format!("Reason: {}\n", reason));
        }

        if let Some(ref portability) = result.portability {
            output.push_str("\nPortability:\n");
            output.push_str(&format!(
                "  Rust regex:    {}\n",
                if portability.rust_regex { "✓" } else { "✗" }
            ));
            output.push_str(&format!(
                "  PCRE2:         {}\n",
                if portability.pcre2 { "✓" } else { "✗" }
            ));
            output.push_str(&format!(
                "  JavaScript:    {}\n",
                if portability.javascript { "✓" } else { "✗" }
            ));
            output.push_str(&format!(
                "  Python re:     {}\n",
                if portability.python_re { "✓" } else { "✗" }
            ));
            output.push_str(&format!(
                "  Python regex:  {}\n",
                if portability.python_regex {
                    "✓"
                } else {
                    "✗"
                }
            ));
            output.push_str(&format!(
                "  Go regexp:     {}\n",
                if portability.go_regexp { "✓" } else { "✗" }
            ));
        }
    } else {
        output.push_str("✗ Pattern is invalid\n");

        if let Some(ref error) = result.error {
            output.push('\n');
            output.push_str(&format!("Error: {}\n", error.message));
            if let Some(pos) = error.position {
                output.push_str(&format!("Position: {}\n", pos));
            }
        }

        if let Some(ref suggestion) = result.suggestion {
            output.push_str(&format!("\nSuggestion: {}\n", suggestion));
        }
    }

    output
}

/// Format ExplainResult as human-readable text
pub fn format_explain_result(result: &ExplainResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Pattern: {}\n\n", result.pattern));
    output.push_str("Breakdown:\n");

    fn format_parts(parts: &[ExplainPart], indent: usize, output: &mut String) {
        let indent_str = "  ".repeat(indent);
        for part in parts {
            let quantifier_str = part
                .quantifier
                .as_ref()
                .map(|q| format!(" ({})", q))
                .unwrap_or_default();
            let group_str = part
                .group
                .map(|g| format!(" [group {}]", g))
                .unwrap_or_default();

            output.push_str(&format!(
                "{}• {} [{}]{}{}\n",
                indent_str, part.token, part.token_type, quantifier_str, group_str
            ));
            output.push_str(&format!("{}  {}\n", indent_str, part.desc));

            if let Some(ref children) = part.children {
                format_parts(children, indent + 1, output);
            }
        }
    }

    format_parts(&result.parts, 0, &mut output);

    output.push('\n');
    output.push_str(&format!("Summary: {}\n", result.summary));

    output
}

/// Format BenchmarkResult as human-readable text
pub fn format_benchmark_result(result: &BenchmarkResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Pattern: {}\n", result.pattern));
    output.push_str(&format!("Engine:  {}\n", result.engine));
    output.push_str(&format!("Input:   {} bytes\n\n", result.input_size_bytes));

    output.push_str("Performance:\n");
    output.push_str(&format!("  Iterations: {}\n", result.iterations));
    output.push_str(&format!("  Average:    {:.1}μs\n", result.avg_us));
    output.push_str(&format!("  Median:     {:.1}μs\n", result.median_us));
    output.push_str(&format!(
        "  Throughput: {:.2} MB/s\n",
        result.throughput_mb_s
    ));

    output.push('\n');
    if result.catastrophic_backtracking {
        output.push_str("⚠ CATASTROPHIC BACKTRACKING DETECTED\n");
        if let Some(ref warning) = result.warning {
            output.push_str(&format!("  {}\n", warning));
        }
        if let Some(ref suggestion) = result.suggestion {
            output.push_str(&format!("  Suggestion: {}\n", suggestion));
        }
    } else {
        output.push_str("✓ No backtracking issues detected\n");
    }

    output
}

/// Format ApplyResult as human-readable text
pub fn format_apply_result(result: &ApplyResult) -> String {
    let mut output = String::new();

    let mode = if result.applied { "APPLIED" } else { "DRY-RUN" };
    output.push_str(&format!("[{}]\n", mode));
    output.push_str(&format!("Pattern:     {}\n", result.pattern));
    output.push_str(&format!("Replacement: {}\n", result.replacement));
    output.push_str(&format!("File:        {}\n", result.file_path));

    if let Some(ref bak) = result.backup_path {
        output.push_str(&format!("Backup:      {}\n", bak));
    }

    output.push('\n');
    output.push_str(&format!(
        "{} replacement{}\n",
        result.replacements_made,
        if result.replacements_made == 1 {
            ""
        } else {
            "s"
        }
    ));

    if !result.preview.is_empty() {
        output.push_str("\nPreview:\n");
        for p in &result.preview {
            output.push_str(&format!("  L{}: {} -> {}\n", p.line, p.before, p.after));
        }
    }

    output
}

/// Format FromExamplesResult as human-readable text
pub fn format_from_examples_result(result: &FromExamplesResult) -> String {
    let mut output = String::new();

    output.push_str("Examples:\n");
    for ex in &result.examples {
        output.push_str(&format!("  • {}\n", ex));
    }

    if let Some(ref neg) = result.negative_examples {
        output.push_str("\nNegative examples (should NOT match):\n");
        for ex in neg {
            output.push_str(&format!("  • {}\n", ex));
        }
    }

    output.push_str("\nInferred patterns:\n");
    for (i, inf) in result.inferred.iter().enumerate() {
        output.push_str(&format!(
            "\n{}. {} (confidence: {:.0}%)\n",
            i + 1,
            inf.pattern,
            inf.confidence * 100.0
        ));
        output.push_str(&format!("   {}\n", inf.desc));
    }

    output
}
