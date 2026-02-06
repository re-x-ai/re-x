//! CLI interface using clap
//!
//! Defines all command-line arguments and subcommands.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "re-x")]
#[command(author, version, about = "AI-native regex CLI — Test, validate, explain. Built for coding agents.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output format
    #[arg(long, short = 'f', global = true, default_value = "json")]
    pub format: OutputFormat,

    /// Enable MCP server mode
    #[arg(long)]
    pub mcp: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// JSON output (default, for AI consumption)
    Json,
    /// Human-readable text
    Text,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Test a regex pattern against input
    Test {
        /// The regex pattern to test
        pattern: String,

        /// Input text to test against (use --file for file input)
        input: Option<String>,

        /// File to test against
        #[arg(long, short = 'F')]
        file: Option<PathBuf>,

        /// Maximum number of matches to return
        #[arg(long, default_value = "100")]
        max_matches: usize,

        /// Force specific engine (regex or fancy-regex)
        #[arg(long)]
        engine: Option<String>,

        /// Enable multiline mode (dot matches newline, ^/$ match line boundaries)
        #[arg(long, short = 'm')]
        multiline: bool,
    },

    /// Test regex replacement
    Replace {
        /// The regex pattern
        pattern: String,

        /// The replacement string (supports $1, $2, etc.)
        replacement: String,

        /// Input text to transform
        input: Option<String>,

        /// File to preview replacements on (dry-run)
        #[arg(long, short = 'F')]
        file: Option<PathBuf>,

        /// Maximum number of preview lines
        #[arg(long, default_value = "20")]
        max_preview: usize,

        /// Enable multiline mode (dot matches newline, ^/$ match line boundaries)
        #[arg(long, short = 'm')]
        multiline: bool,
    },

    /// Validate regex syntax and check portability
    Validate {
        /// The regex pattern to validate
        pattern: String,

        /// Target language to check compatibility
        #[arg(long, short = 't')]
        target_lang: Option<String>,
    },

    /// Explain a regex pattern
    Explain {
        /// The regex pattern to explain
        pattern: String,
    },

    /// Infer regex pattern from examples
    FromExamples {
        /// Example strings that should match
        #[arg(required = true, num_args = 2..)]
        examples: Vec<String>,

        /// Strings that should NOT match
        #[arg(long, short = 'n', num_args = 1..)]
        negative: Option<Vec<String>>,
    },

    /// Apply regex replacement to a file (with backup)
    Apply {
        /// The regex pattern
        pattern: String,

        /// The replacement string (supports $1, $2, etc.)
        replacement: String,

        /// File to apply replacements to
        #[arg(long, short = 'F', required = true)]
        file: PathBuf,

        /// Dry-run mode (show what would change, don't write)
        #[arg(long)]
        dry_run: bool,

        /// Disable backup (.bak) creation
        #[arg(long)]
        no_backup: bool,

        /// Maximum number of preview lines
        #[arg(long, default_value = "20")]
        max_preview: usize,

        /// Enable multiline mode (dot matches newline, ^/$ match line boundaries)
        #[arg(long, short = 'm')]
        multiline: bool,
    },

    /// Benchmark regex performance and detect ReDoS
    Benchmark {
        /// The regex pattern to benchmark
        pattern: String,

        /// Input text to test against
        #[arg(long, short = 'i')]
        input: Option<String>,

        /// File to benchmark against
        #[arg(long, short = 'F')]
        file: Option<PathBuf>,

        /// Timeout in milliseconds
        #[arg(long, default_value = "5000")]
        timeout_ms: u64,

        /// Number of iterations
        #[arg(long, default_value = "100")]
        iterations: usize,
    },
}

/// Parse CLI arguments
pub fn parse() -> Cli {
    Cli::parse()
}

/// Handle the test command
pub fn handle_test(
    pattern: &str,
    input: Option<&str>,
    file: Option<&PathBuf>,
    max_matches: usize,
    engine: Option<&str>,
    multiline: bool,
    format: OutputFormat,
) -> Result<String, String> {
    use crate::core::{test_file, test_stdin, test_string, EngineType, TestOptions};
    use crate::output::json::format_json;
    use crate::output::text::format_test_result;
    use std::io::IsTerminal;

    let engine_type = match engine {
        Some(e) => Some(match e {
            "regex" => EngineType::Regex,
            "fancy-regex" | "fancy" => EngineType::FancyRegex,
            _ => {
                return Err(format!(
                    "Unknown engine '{}'. Valid options: regex, fancy-regex",
                    e
                ))
            }
        }),
        None => None,
    };

    let options = TestOptions {
        max_matches: Some(max_matches),
        engine: engine_type,
        multiline,
    };

    let result = if let Some(file_path) = file {
        test_file(pattern, file_path, &options)?
    } else if let Some(text) = input {
        test_string(pattern, text, &options)?
    } else {
        // Read from stdin — but warn if it's a terminal (no pipe)
        if std::io::stdin().is_terminal() {
            eprintln!("re-x: reading from stdin (pipe data or press Ctrl-D when done)");
            eprintln!(
                "  hint: re-x test '{}' \"text\" — or — cat file | re-x test '{}'",
                pattern, pattern
            );
        }
        test_stdin(pattern, &options)?
    };

    match format {
        OutputFormat::Json => Ok(format_json(&result)),
        OutputFormat::Text => Ok(format_test_result(&result)),
    }
}

/// Handle the replace command
pub fn handle_replace(
    pattern: &str,
    replacement: &str,
    input: Option<&str>,
    file: Option<&PathBuf>,
    max_preview: usize,
    multiline: bool,
    format: OutputFormat,
) -> Result<String, String> {
    use crate::core::{replace_file_preview, replace_with_captures};
    use crate::output::json::format_json;
    use crate::output::text::format_replace_result;

    if let Some(file_path) = file {
        let result = replace_file_preview(
            pattern,
            replacement,
            file_path,
            Some(max_preview),
            multiline,
        )?;
        match format {
            OutputFormat::Json => Ok(format_json(&result)),
            OutputFormat::Text => {
                // Simple text format for file preview
                let mut output = format!(
                    "Pattern: {}\nReplacement: {}\n\n",
                    result.pattern, result.replacement
                );
                output.push_str(&format!(
                    "Total replacements: {}\n\nPreview:\n",
                    result.replacements_made
                ));
                for preview in &result.preview {
                    output.push_str(&format!(
                        "Line {}: {} → {}\n",
                        preview.line, preview.before, preview.after
                    ));
                }
                Ok(output)
            }
        }
    } else if let Some(text) = input {
        let result = replace_with_captures(pattern, replacement, text, multiline)?;
        match format {
            OutputFormat::Json => Ok(format_json(&result)),
            OutputFormat::Text => Ok(format_replace_result(&result)),
        }
    } else {
        use std::io::{self, IsTerminal, Read};
        // Read from stdin like test does
        if io::stdin().is_terminal() {
            eprintln!("re-x: reading from stdin (pipe data or press Ctrl-D when done)");
            eprintln!(
                "  hint: re-x replace '{}' '{}' \"text\" — or — cat file | re-x replace '{}' '{}'",
                pattern, replacement, pattern, replacement
            );
        }
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|e| format!("Failed to read stdin: {}", e))?;
        let result = replace_with_captures(pattern, replacement, &input, multiline)?;
        match format {
            OutputFormat::Json => Ok(format_json(&result)),
            OutputFormat::Text => Ok(format_replace_result(&result)),
        }
    }
}

/// Handle the validate command
pub fn handle_validate(
    pattern: &str,
    target_lang: Option<&str>,
    format: OutputFormat,
) -> Result<String, String> {
    use crate::core::{validate_for_language, validate_pattern};
    use crate::output::json::format_json;
    use crate::output::text::format_validate_result;

    let result = if let Some(lang) = target_lang {
        validate_for_language(pattern, lang)
    } else {
        validate_pattern(pattern)
    };

    match format {
        OutputFormat::Json => Ok(format_json(&result)),
        OutputFormat::Text => Ok(format_validate_result(&result)),
    }
}

/// Handle the explain command
pub fn handle_explain(pattern: &str, format: OutputFormat) -> Result<String, String> {
    use crate::core::explain_pattern;
    use crate::output::json::format_json;
    use crate::output::text::format_explain_result;

    let result = explain_pattern(pattern)?;

    match format {
        OutputFormat::Json => Ok(format_json(&result)),
        OutputFormat::Text => Ok(format_explain_result(&result)),
    }
}

/// Handle the from-examples command
pub fn handle_from_examples(
    examples: &[String],
    negative: Option<&[String]>,
    format: OutputFormat,
) -> Result<String, String> {
    use crate::core::infer_patterns;
    use crate::output::json::format_json;
    use crate::output::text::format_from_examples_result;

    let result = infer_patterns(examples, negative)?;

    match format {
        OutputFormat::Json => Ok(format_json(&result)),
        OutputFormat::Text => Ok(format_from_examples_result(&result)),
    }
}

/// Handle the apply command
#[allow(clippy::too_many_arguments)]
pub fn handle_apply(
    pattern: &str,
    replacement: &str,
    file: &std::path::Path,
    dry_run: bool,
    no_backup: bool,
    max_preview: usize,
    multiline: bool,
    format: OutputFormat,
) -> Result<String, String> {
    use crate::core::apply_file;
    use crate::output::json::format_json;
    use crate::output::text::format_apply_result;

    let result = apply_file(
        pattern,
        replacement,
        file,
        dry_run,
        !no_backup,
        Some(max_preview),
        multiline,
    )?;

    match format {
        OutputFormat::Json => Ok(format_json(&result)),
        OutputFormat::Text => Ok(format_apply_result(&result)),
    }
}

/// Handle the benchmark command
pub fn handle_benchmark(
    pattern: &str,
    input: Option<&str>,
    file: Option<&PathBuf>,
    timeout_ms: u64,
    iterations: usize,
    format: OutputFormat,
) -> Result<String, String> {
    use crate::core::{
        benchmark::generate_redos_input, benchmark_file, benchmark_pattern, BenchmarkOptions,
    };
    use crate::output::json::format_json;
    use crate::output::text::format_benchmark_result;

    let options = BenchmarkOptions {
        iterations,
        timeout_ms,
    };

    let result = if let Some(file_path) = file {
        benchmark_file(pattern, file_path, &options)?
    } else if let Some(text) = input {
        benchmark_pattern(pattern, text, &options)?
    } else {
        // Generate adversarial input for ReDoS testing
        let evil_input = generate_redos_input(pattern);
        benchmark_pattern(pattern, &evil_input, &options)?
    };

    match format {
        OutputFormat::Json => Ok(format_json(&result)),
        OutputFormat::Text => Ok(format_benchmark_result(&result)),
    }
}
