//! Core regex engine and command implementations
//!
//! This module contains all the business logic for re-x commands.

pub mod benchmark;
pub mod engine;
pub mod explain;
pub mod from_examples;
pub mod portability;
pub mod replace;
pub mod templates;
pub mod test;
pub mod validate;

// Re-export commonly used types
pub use benchmark::{benchmark_file, benchmark_pattern, BenchmarkOptions};
pub use engine::EngineType;
pub use explain::explain_pattern;
pub use from_examples::infer_patterns;
pub use replace::{apply_file, replace_file_preview, replace_with_captures};
pub use test::{test_file, test_stdin, test_string, TestOptions};
pub use validate::{validate_for_language, validate_pattern};
