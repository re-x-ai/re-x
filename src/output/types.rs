//! Output types for re-x commands
//!
//! All output structures are designed to be JSON-first for AI consumption.

use serde::{Deserialize, Serialize};

/// A single capture group within a match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capture {
    /// Group number (1-indexed for capturing groups)
    pub group: usize,
    /// Named group name (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Captured text
    pub text: String,
    /// Start byte position (0-indexed)
    pub start: usize,
    /// End byte position (exclusive)
    pub end: usize,
}

/// A single match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    /// Full matched text
    pub text: String,
    /// Start byte position (0-indexed)
    pub start: usize,
    /// End byte position (exclusive)
    pub end: usize,
    /// Capture groups (empty if no capturing groups)
    pub captures: Vec<Capture>,
}

/// Result of `re-x test` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// The pattern that was tested
    pub pattern: String,
    /// Which engine was used (regex or fancy-regex)
    pub engine: String,
    /// Length of input in bytes
    pub input_length: usize,
    /// Whether any match was found
    pub matched: bool,
    /// Number of matches found
    pub match_count: usize,
    /// All matches with positions and captures
    pub matches: Vec<Match>,
    /// Elapsed time in microseconds
    pub elapsed_us: u64,
}

/// Result of `re-x replace` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceResult {
    /// The pattern that was used
    pub pattern: String,
    /// The replacement string
    pub replacement: String,
    /// Original input
    pub original: String,
    /// Result after replacement
    pub result: String,
    /// Number of replacements made
    pub replacements_made: usize,
}

/// A single replacement preview (for file dry-run)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacePreview {
    /// Line number (1-indexed)
    pub line: usize,
    /// Original line content
    pub before: String,
    /// Line content after replacement
    pub after: String,
}

/// Result of `re-x replace --file --dry-run`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceFileResult {
    /// The pattern that was used
    pub pattern: String,
    /// The replacement string
    pub replacement: String,
    /// Total number of replacements
    pub replacements_made: usize,
    /// Preview of changes
    pub preview: Vec<ReplacePreview>,
}

/// Language/engine portability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portability {
    /// Rust regex crate
    pub rust_regex: bool,
    /// PCRE2
    pub pcre2: bool,
    /// JavaScript RegExp
    pub javascript: bool,
    /// Python re module
    pub python_re: bool,
    /// Python regex module (third-party)
    pub python_regex: bool,
    /// Go regexp package
    pub go_regexp: bool,
    /// Java java.util.regex
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java: Option<bool>,
    /// .NET System.Text.RegularExpressions
    pub dotnet: bool,
    /// Ruby (Oniguruma/Onigmo)
    pub ruby: bool,
}

/// Error information for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error kind/type
    pub kind: String,
    /// Position in pattern where error occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
    /// Human-readable error message
    pub message: String,
}

/// Result of `re-x validate` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResult {
    /// Whether the pattern is valid
    pub valid: bool,
    /// Error details (if invalid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ValidationError>,
    /// Which engine is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_required: Option<String>,
    /// Reason for engine requirement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Portability to other languages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portability: Option<Portability>,
    /// Suggested fix (if invalid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// A single token/part in pattern explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainPart {
    /// The token text
    pub token: String,
    /// Token type (anchor, literal, quantifier, etc.)
    #[serde(rename = "type")]
    pub token_type: String,
    /// Human-readable description
    pub desc: String,
    /// Quantifier if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantifier: Option<String>,
    /// Capturing group number (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<usize>,
    /// Child parts (for groups)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<ExplainPart>>,
}

/// Result of `re-x explain` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainResult {
    /// The pattern that was explained
    pub pattern: String,
    /// Breakdown of pattern parts
    pub parts: Vec<ExplainPart>,
    /// High-level summary of what the pattern does
    pub summary: String,
}

/// A single inferred pattern candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredPattern {
    /// The inferred pattern
    pub pattern: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Human-readable description
    pub desc: String,
}

/// Result of `re-x from-examples` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FromExamplesResult {
    /// Input examples
    pub examples: Vec<String>,
    /// Negative examples (should not match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_examples: Option<Vec<String>>,
    /// Inferred pattern candidates
    pub inferred: Vec<InferredPattern>,
}

/// Result of `re-x benchmark` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// The pattern that was benchmarked
    pub pattern: String,
    /// Which engine was used
    pub engine: String,
    /// Input size in bytes
    pub input_size_bytes: usize,
    /// Number of iterations run
    pub iterations: usize,
    /// Average time in microseconds
    pub avg_us: f64,
    /// Median time in microseconds
    pub median_us: f64,
    /// Throughput in MB/s
    pub throughput_mb_s: f64,
    /// Whether catastrophic backtracking was detected
    pub catastrophic_backtracking: bool,
    /// Whether timeout occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<bool>,
    /// Warning message (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// Suggestion for improvement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Result of `re-x apply` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    /// The pattern that was used
    pub pattern: String,
    /// The replacement string
    pub replacement: String,
    /// Path to the file that was modified
    pub file_path: String,
    /// Path to the backup file (None if --no-backup)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    /// Total number of replacements made
    pub replacements_made: usize,
    /// Whether changes were actually written (false for dry-run)
    pub applied: bool,
    /// Preview of changes
    pub preview: Vec<ReplacePreview>,
}

/// Generic error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Always true for errors
    pub error: bool,
    /// Error code
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Position in pattern (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
    /// Context around the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Suggested fix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Link to relevant documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_hint: Option<String>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: true,
            code: code.into(),
            message: message.into(),
            position: None,
            context: None,
            suggestion: None,
            docs_hint: None,
        }
    }

    /// Add position information
    #[allow(dead_code)]
    pub fn with_position(mut self, position: usize) -> Self {
        self.position = Some(position);
        self
    }

    /// Add context
    #[allow(dead_code)]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add suggestion
    #[allow(dead_code)]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Error codes used throughout re-x
#[allow(dead_code)]
pub mod error_codes {
    pub const INVALID_PATTERN: &str = "INVALID_PATTERN";
    pub const FILE_NOT_FOUND: &str = "FILE_NOT_FOUND";
    pub const FILE_TOO_LARGE: &str = "FILE_TOO_LARGE";
    pub const TIMEOUT: &str = "TIMEOUT";
    pub const ENCODING_ERROR: &str = "ENCODING_ERROR";
    pub const ENGINE_UNSUPPORTED: &str = "ENGINE_UNSUPPORTED";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
}
