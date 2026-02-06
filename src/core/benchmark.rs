//! Implementation of `re-x benchmark` command
//!
//! Measures regex performance and detects catastrophic backtracking (ReDoS).

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

static NESTED_QUANTIFIER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\([^)]*[+*][^)]*\)[+*]")
        .expect("BUG: nested quantifier detection pattern is invalid")
});

use super::engine::CompiledRegex;
use crate::output::BenchmarkResult;

/// Options for benchmarking
pub struct BenchmarkOptions {
    /// Number of iterations to run
    pub iterations: usize,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for BenchmarkOptions {
    fn default() -> Self {
        Self {
            iterations: 100,
            timeout_ms: 5000,
        }
    }
}

/// Benchmark a pattern against input
pub fn benchmark_pattern(
    pattern: &str,
    input: &str,
    options: &BenchmarkOptions,
) -> Result<BenchmarkResult, String> {
    let (compiled, engine_type) = CompiledRegex::new(pattern).map_err(|e| e.to_string())?;

    let timeout = Duration::from_millis(options.timeout_ms);
    let mut timings_ns = Vec::with_capacity(options.iterations);

    let start_total = Instant::now();
    let mut catastrophic = false;
    let mut timed_out = false;

    for _ in 0..options.iterations {
        if start_total.elapsed() > timeout {
            timed_out = true;
            break;
        }

        let start = Instant::now();

        match &compiled {
            CompiledRegex::Regex(re) => {
                let _ = re.find_iter(input).count();
            }
            CompiledRegex::FancyRegex(re) => {
                let mut pos = 0;
                while pos < input.len() {
                    match re.find_from_pos(input, pos) {
                        Ok(Some(m)) => {
                            let next =
                                pos + input[pos..].chars().next().map_or(1, |c| c.len_utf8());
                            pos = m.end().max(next);
                        }
                        Ok(None) => break,
                        Err(_) => break,
                    }

                    // Check for timeout within iteration
                    if start.elapsed() > Duration::from_millis(1000) {
                        catastrophic = true;
                        break;
                    }
                }
            }
        }

        let elapsed = start.elapsed();
        timings_ns.push(elapsed.as_nanos() as u64);

        // Detect catastrophic backtracking
        if elapsed > Duration::from_millis(100) && timings_ns.len() > 1 {
            // If a single iteration takes > 100ms, likely catastrophic
            catastrophic = true;
        }
    }

    // Calculate statistics
    if timings_ns.is_empty() {
        return Ok(BenchmarkResult {
            pattern: pattern.to_string(),
            engine: engine_type.to_string(),
            input_size_bytes: input.len(),
            iterations: 0,
            avg_us: 0.0,
            median_us: 0.0,
            throughput_mb_s: 0.0,
            catastrophic_backtracking: true,
            timeout: Some(true),
            warning: Some("Pattern timed out immediately".to_string()),
            suggestion: suggest_fix(pattern),
        });
    }

    timings_ns.sort();
    let avg_ns = timings_ns.iter().sum::<u64>() / timings_ns.len() as u64;
    let median_ns = timings_ns[timings_ns.len() / 2];
    let avg_us = avg_ns as f64 / 1_000.0;
    let median_us = median_ns as f64 / 1_000.0;

    // Calculate throughput (use nanosecond precision to avoid div-by-zero)
    let throughput_mb_s = if avg_ns > 0 {
        (input.len() as f64 / 1_000_000.0) / (avg_ns as f64 / 1_000_000_000.0)
    } else {
        0.0
    };

    // Check for backtracking indicators using nanosecond precision
    let stddev_ns = if timings_ns.len() > 1 {
        let mean = avg_ns as f64;
        let variance: f64 = timings_ns
            .iter()
            .map(|&t| (t as f64 - mean).powi(2))
            .sum::<f64>()
            / (timings_ns.len() - 1) as f64;
        variance.sqrt()
    } else {
        0.0
    };

    // Only flag as catastrophic if avg is slow enough to matter (>1ms)
    // AND variance is high relative to mean
    if avg_ns > 1_000_000 && stddev_ns > avg_ns as f64 * 2.0 {
        catastrophic = true;
    }

    let warning = if catastrophic {
        Some("Pattern exhibits exponential time complexity".to_string())
    } else if timed_out {
        Some("Benchmark timed out before completing all iterations".to_string())
    } else {
        None
    };

    Ok(BenchmarkResult {
        pattern: pattern.to_string(),
        engine: engine_type.to_string(),
        input_size_bytes: input.len(),
        iterations: timings_ns.len(),
        avg_us,
        median_us,
        throughput_mb_s,
        catastrophic_backtracking: catastrophic,
        timeout: if timed_out { Some(true) } else { None },
        warning,
        suggestion: if catastrophic {
            suggest_fix(pattern)
        } else {
            None
        },
    })
}

/// Benchmark a pattern against a file
pub fn benchmark_file(
    pattern: &str,
    file_path: &Path,
    options: &BenchmarkOptions,
) -> Result<BenchmarkResult, String> {
    let mut file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    benchmark_pattern(pattern, &content, options)
}

/// Generate ReDoS test inputs for common patterns
pub fn generate_redos_input(pattern: &str) -> String {
    // Common ReDoS patterns and their corresponding evil inputs
    let evil_inputs = [
        // (a+)+$ pattern
        (r"(a+)+", "aaaaaaaaaaaaaaaaaaaab"),
        // (a|aa)+$ pattern
        (r"(a|aa)+", "aaaaaaaaaaaaaaaaaaaab"),
        // (a|a?)+$ pattern
        (r"(a|a?)+", "aaaaaaaaaaaaaaaaaaaab"),
        // Nested quantifiers
        (r"(.*)*", "aaaaaaaaaaaaaaaaaaaaX"),
        // Email-like with nested quantifiers
        (r"(.+)+@", "aaaaaaaaaaaaaaaaaaaa!"),
    ];

    for (pat, input) in &evil_inputs {
        if pattern.contains(pat) {
            return input.to_string();
        }
    }

    // Generate input based on pattern analysis
    if pattern.contains("a+)+") || pattern.contains("a*)*") {
        return "aaaaaaaaaaaaaaaaaaaab".to_string();
    }

    // Default: use a moderately sized repeated string
    "a".repeat(30) + "X"
}

/// Detect potential ReDoS vulnerability in a pattern
pub fn detect_redos_vulnerability(pattern: &str) -> Option<String> {
    // Patterns that are known to be vulnerable to ReDoS
    let vulnerable_patterns = [
        (r"(\w+)+", "Nested quantifiers on word characters"),
        (r"(a+)+", "Nested + quantifiers"),
        (r"(a*)*", "Nested * quantifiers"),
        (r"(a+)*", "Mixed nested quantifiers"),
        (r"(a|aa)+", "Overlapping alternation with quantifier"),
        (r"(a|a?)+", "Overlapping optional with quantifier"),
        (r"(.+)+", "Nested + on any character"),
        (r"(.*)+", "Nested quantifiers on .*"),
        (r"(.+)*", "Mixed quantifiers on .+"),
        (r"(.*)*", "Nested * on .*"),
    ];

    for (vuln_pat, desc) in &vulnerable_patterns {
        if pattern.contains(vuln_pat) {
            return Some(desc.to_string());
        }
    }

    // Check for nested quantifiers pattern more generally
    if NESTED_QUANTIFIER_RE.is_match(pattern) {
        return Some("Nested quantifiers detected".to_string());
    }

    None
}

/// Suggest fix for ReDoS vulnerable patterns
fn suggest_fix(pattern: &str) -> Option<String> {
    if pattern.contains("(a+)+") {
        return Some("Use atomic group or possessive quantifier: (?>a+)+".to_string());
    }

    if pattern.contains("(.+)+") || pattern.contains("(.*)+") {
        return Some("Use atomic group: (?>.+)+ or limit repetition".to_string());
    }

    if detect_redos_vulnerability(pattern).is_some() {
        return Some("Consider using atomic groups (?>...) or possessive quantifiers to prevent backtracking".to_string());
    }

    None
}

/// Quick check if a pattern might be vulnerable (without benchmarking)
#[allow(dead_code)]
pub fn quick_vulnerability_check(pattern: &str) -> bool {
    detect_redos_vulnerability(pattern).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_simple() {
        let result =
            benchmark_pattern(r"\d+", "hello 123 world 456", &BenchmarkOptions::default()).unwrap();

        assert!(!result.catastrophic_backtracking);
        assert!(result.avg_us < 10000.0); // Should be very fast
    }

    #[test]
    fn test_detect_redos() {
        assert!(detect_redos_vulnerability(r"(a+)+").is_some());
        assert!(detect_redos_vulnerability(r"(.*)*").is_some());
        assert!(detect_redos_vulnerability(r"\d+").is_none());
    }

    #[test]
    fn test_generate_evil_input() {
        let input = generate_redos_input(r"(a+)+");
        assert!(input.contains('a'));
        assert!(input.len() > 10);
    }
}
