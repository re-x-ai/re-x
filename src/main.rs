//! re-x - AI-native regex CLI
//!
//! Test, validate, explain. Built for coding agents.

mod core;
mod output;

#[cfg(feature = "cli")]
mod cli;

#[cfg(feature = "mcp")]
mod mcp;

use std::process::ExitCode;

fn main() -> ExitCode {
    #[cfg(feature = "cli")]
    {
        use cli::{parse, Commands};

        let args = parse();

        // Check for MCP mode
        #[cfg(feature = "mcp")]
        if args.mcp {
            return run_mcp_server();
        }

        // If no command and no MCP mode, show help
        let Some(command) = args.command else {
            eprintln!("re-x: AI-native regex CLI");
            eprintln!();
            eprintln!("Usage: re-x <COMMAND>");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  test          Test a regex pattern against input");
            eprintln!("  replace       Test regex replacement");
            eprintln!("  validate      Validate regex syntax and check portability");
            eprintln!("  explain       Explain a regex pattern");
            eprintln!("  from-examples Infer regex pattern from examples");
            eprintln!("  apply         Apply regex replacement to a file (with backup)");
            eprintln!("  benchmark     Benchmark regex performance and detect ReDoS");
            eprintln!();
            eprintln!("Options:");
            eprintln!("  -f, --format <FORMAT>  Output format [json|text] (default: json)");
            eprintln!("  --mcp                  Run as MCP server");
            eprintln!("  -h, --help             Print help");
            eprintln!("  -V, --version          Print version");
            return ExitCode::SUCCESS;
        };

        let format = args.format;

        let result = match command {
            Commands::Test {
                pattern,
                input,
                file,
                max_matches,
                engine,
                multiline,
            } => cli::handle_test(
                &pattern,
                input.as_deref(),
                file.as_ref(),
                max_matches,
                engine.as_deref(),
                multiline,
                format,
            ),

            Commands::Replace {
                pattern,
                replacement,
                input,
                file,
                dry_run: _,
                max_preview,
                multiline,
            } => cli::handle_replace(
                &pattern,
                &replacement,
                input.as_deref(),
                file.as_ref(),
                max_preview,
                multiline,
                format,
            ),

            Commands::Validate {
                pattern,
                target_lang,
            } => cli::handle_validate(&pattern, target_lang.as_deref(), format),

            Commands::Explain { pattern } => cli::handle_explain(&pattern, format),

            Commands::FromExamples { examples, negative } => {
                cli::handle_from_examples(&examples, negative.as_deref(), format)
            }

            Commands::Apply {
                pattern,
                replacement,
                file,
                dry_run,
                no_backup,
                max_preview,
                multiline,
            } => cli::handle_apply(
                &pattern,
                &replacement,
                &file,
                dry_run,
                no_backup,
                max_preview,
                multiline,
                format,
            ),

            Commands::Benchmark {
                pattern,
                input,
                file,
                timeout_ms,
                iterations,
            } => cli::handle_benchmark(
                &pattern,
                input.as_deref(),
                file.as_ref(),
                timeout_ms,
                iterations,
                format,
            ),
        };

        match result {
            Ok(output) => {
                println!("{}", output);
                ExitCode::SUCCESS
            }
            Err(e) => {
                // Output error as structured JSON for AI consumption
                let error = crate::output::ErrorResponse::new("COMMAND_ERROR", &e);
                let error_json = serde_json::to_string(&error)
                    .unwrap_or_else(|_| format!(r#"{{"error":true,"message":"{}"}}"#, e));
                eprintln!("{}", error_json);
                ExitCode::FAILURE
            }
        }
    }

    #[cfg(not(feature = "cli"))]
    {
        eprintln!("CLI feature not enabled. Build with --features cli");
        ExitCode::FAILURE
    }
}

#[cfg(feature = "mcp")]
fn run_mcp_server() -> ExitCode {
    match mcp::run_server() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("MCP server error: {}", e);
            ExitCode::FAILURE
        }
    }
}
