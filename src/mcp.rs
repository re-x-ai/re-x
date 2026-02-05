//! MCP Server mode implementation
//!
//! Implements the Model Context Protocol for AI tool integration.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::core;
use crate::output::json::format_json;

/// MCP JSON-RPC request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// MCP JSON-RPC response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// MCP JSON-RPC error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Tool definition for MCP
#[derive(Debug, Serialize)]
struct ToolDefinition {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

/// Send a JSON-RPC response to stdout
fn send_response(stdout: &mut io::Stdout, response: &JsonRpcResponse) -> Result<(), String> {
    let json = serde_json::to_string(response)
        .unwrap_or_else(|_| r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"Internal serialization error"}}"#.to_string());
    writeln!(stdout, "{}", json).map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// Run the MCP server
pub fn run_server() -> Result<(), String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;

        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                send_response(&mut stdout, &error_response)?;
                continue;
            }
        };

        // JSON-RPC 2.0: A Notification is a Request without an "id" member.
        // Notifications MUST NOT receive a response.
        if request.id.is_none() {
            handle_notification(&request);
            continue;
        }

        let response = handle_request(&request);
        send_response(&mut stdout, &response)?;
    }

    Ok(())
}

/// Handle a notification (no response sent)
fn handle_notification(request: &JsonRpcRequest) {
    match request.method.as_str() {
        "notifications/initialized" => {
            // Client confirmed initialization complete — nothing to do
        }
        "notifications/cancelled" => {
            // Client cancelled a request.
            // Currently all operations are synchronous, so nothing to cancel.
        }
        _ => {
            // Unknown notification — ignore per JSON-RPC spec
        }
    }
}

/// Handle a request (always returns a response)
fn handle_request(request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "re-x",
                    "version": env!("CARGO_PKG_VERSION"),
                    "title": "re-x Regex Toolkit",
                    "description": "AI-native regex CLI — Test, validate, explain, benchmark regex patterns"
                }
            })),
            error: None,
        },

        "ping" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({})),
            error: None,
        },

        "tools/list" => {
            let tools = get_tools();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({ "tools": tools })),
                error: None,
            }
        }

        "tools/call" => {
            let params = request.params.as_ref();
            let tool_name = match params.and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                Some(name) => name,
                None => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Invalid params: missing tool name".to_string(),
                            data: None,
                        }),
                    };
                }
            };

            let arguments = params
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or(json!({}));

            match call_tool(tool_name, &arguments) {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": result
                        }]
                    })),
                    error: None,
                },
                Err(e) => {
                    let error_response = crate::output::ErrorResponse::new("TOOL_ERROR", &e);
                    let error_msg = serde_json::to_string(&error_response)
                        .unwrap_or_else(|_| format!(r#"{{"error":true,"message":"{}"}}"#, e));
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(json!({
                            "content": [{
                                "type": "text",
                                "text": error_msg
                            }],
                            "isError": true
                        })),
                        error: None,
                    }
                }
            }
        }

        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
                data: None,
            }),
        },
    }
}

/// Get tool definitions
fn get_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "regex_test".to_string(),
            description: "Test a regex pattern against input text or file. Returns all matches with positions and capture groups as structured JSON. Use this to verify regex patterns work correctly before using them in code.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to test"
                    },
                    "input": {
                        "type": "string",
                        "description": "Text to test against"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "File path to test against (alternative to input)"
                    },
                    "max_matches": {
                        "type": "integer",
                        "description": "Maximum matches to return (default: 100)"
                    },
                    "multiline": {
                        "type": "boolean",
                        "description": "Enable multiline mode: dot matches newline, ^/$ match line boundaries (default: false)"
                    }
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "regex_replace".to_string(),
            description: "Test regex replacement on input text. Shows before/after without modifying any files. Supports capture group references ($1, $2, etc.).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern"
                    },
                    "replacement": {
                        "type": "string",
                        "description": "Replacement string (supports $1, $2 for capture groups)"
                    },
                    "input": {
                        "type": "string",
                        "description": "Text to transform"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "File to preview replacements on"
                    },
                    "multiline": {
                        "type": "boolean",
                        "description": "Enable multiline mode: dot matches newline, ^/$ match line boundaries (default: false)"
                    }
                },
                "required": ["pattern", "replacement"]
            }),
        },
        ToolDefinition {
            name: "regex_validate".to_string(),
            description: "Check if a regex pattern is valid and report which languages/engines support it. Use this before writing regex into source code to ensure cross-language compatibility.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to validate"
                    },
                    "target_lang": {
                        "type": "string",
                        "description": "Check compatibility for specific language (rust|python|javascript|go|java|pcre)"
                    }
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "regex_explain".to_string(),
            description: "Break down a regex pattern into its component parts with descriptions. Use this to understand complex patterns found in existing code.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to explain"
                    }
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "regex_from_examples".to_string(),
            description: "Infer a regex pattern from example strings. Provides multiple candidates with confidence scores. Use when you need to create a pattern that matches specific formats.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "examples": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Example strings that should match (at least 2)"
                    },
                    "negative_examples": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Strings that should NOT match"
                    }
                },
                "required": ["examples"]
            }),
        },
        ToolDefinition {
            name: "regex_apply".to_string(),
            description: "Apply regex replacement to a file. Creates a .bak backup by default. Use dry_run to preview changes without modifying the file.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern"
                    },
                    "replacement": {
                        "type": "string",
                        "description": "Replacement string (supports $1, $2 for capture groups)"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file to modify"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Preview changes without writing (default: false)"
                    },
                    "backup": {
                        "type": "boolean",
                        "description": "Create .bak backup before writing (default: true)"
                    },
                    "max_preview": {
                        "type": "integer",
                        "description": "Maximum preview lines to return (default: 20)"
                    },
                    "multiline": {
                        "type": "boolean",
                        "description": "Enable multiline mode: dot matches newline, ^/$ match line boundaries (default: false)"
                    }
                },
                "required": ["pattern", "replacement", "file_path"]
            }),
        },
        ToolDefinition {
            name: "regex_benchmark".to_string(),
            description: "Measure regex performance and detect catastrophic backtracking (ReDoS). Use before deploying regex in production, especially on user-provided input.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to benchmark"
                    },
                    "input": {
                        "type": "string",
                        "description": "Test input (if not provided, generates adversarial input)"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "File for realistic benchmark"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Timeout in milliseconds (default: 5000)"
                    }
                },
                "required": ["pattern"]
            }),
        },
    ]
}

/// Call a specific tool
fn call_tool(name: &str, arguments: &Value) -> Result<String, String> {
    match name {
        "regex_test" => {
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("pattern is required")?;

            let input = arguments.get("input").and_then(|v| v.as_str());

            let file_path = arguments.get("file_path").and_then(|v| v.as_str());

            let max_matches = arguments
                .get("max_matches")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(100);

            let multiline = arguments
                .get("multiline")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let options = core::TestOptions {
                max_matches: Some(max_matches),
                engine: None,
                multiline,
            };

            let result = if let Some(fp) = file_path {
                core::test_file(pattern, std::path::Path::new(fp), &options)?
            } else if let Some(text) = input {
                core::test_string(pattern, text, &options)?
            } else {
                return Err("Either input or file_path is required".to_string());
            };

            Ok(format_json(&result))
        }

        "regex_replace" => {
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("pattern is required")?;

            let replacement = arguments
                .get("replacement")
                .and_then(|v| v.as_str())
                .ok_or("replacement is required")?;

            let input = arguments.get("input").and_then(|v| v.as_str());

            let file_path = arguments.get("file_path").and_then(|v| v.as_str());

            let multiline = arguments
                .get("multiline")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if let Some(fp) = file_path {
                let result = core::replace_file_preview(
                    pattern,
                    replacement,
                    std::path::Path::new(fp),
                    Some(20),
                    multiline,
                )?;
                Ok(format_json(&result))
            } else if let Some(text) = input {
                let result = core::replace_with_captures(pattern, replacement, text, multiline)?;
                Ok(format_json(&result))
            } else {
                Err("Either input or file_path is required".to_string())
            }
        }

        "regex_validate" => {
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("pattern is required")?;

            let target_lang = arguments.get("target_lang").and_then(|v| v.as_str());

            let result = if let Some(lang) = target_lang {
                core::validate_for_language(pattern, lang)
            } else {
                core::validate_pattern(pattern)
            };

            Ok(format_json(&result))
        }

        "regex_explain" => {
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("pattern is required")?;

            let result = core::explain_pattern(pattern)?;
            Ok(format_json(&result))
        }

        "regex_from_examples" => {
            let examples: Vec<String> = arguments
                .get("examples")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .ok_or("examples is required")?;

            let negatives: Option<Vec<String>> = arguments
                .get("negative_examples")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                });

            let result = core::infer_patterns(&examples, negatives.as_deref())?;

            Ok(format_json(&result))
        }

        "regex_apply" => {
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("pattern is required")?;

            let replacement = arguments
                .get("replacement")
                .and_then(|v| v.as_str())
                .ok_or("replacement is required")?;

            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .ok_or("file_path is required")?;

            let dry_run = arguments
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let backup = arguments
                .get("backup")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let max_preview = arguments
                .get("max_preview")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(20);

            let multiline = arguments
                .get("multiline")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let result = core::apply_file(
                pattern,
                replacement,
                std::path::Path::new(file_path),
                dry_run,
                backup,
                Some(max_preview),
                multiline,
            )?;

            Ok(format_json(&result))
        }

        "regex_benchmark" => {
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("pattern is required")?;

            let input = arguments.get("input").and_then(|v| v.as_str());

            let file_path = arguments.get("file_path").and_then(|v| v.as_str());

            let timeout_ms = arguments
                .get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000);

            let options = core::BenchmarkOptions {
                iterations: 100,
                timeout_ms,
            };

            let result = if let Some(fp) = file_path {
                core::benchmark_file(pattern, std::path::Path::new(fp), &options)?
            } else if let Some(text) = input {
                core::benchmark_pattern(pattern, text, &options)?
            } else {
                // Generate adversarial input
                let evil_input = core::benchmark::generate_redos_input(pattern);
                core::benchmark_pattern(pattern, &evil_input, &options)?
            };

            Ok(format_json(&result))
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
