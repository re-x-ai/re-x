//! CLI end-to-end tests

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn re_x() -> Command {
    Command::new(assert_cmd::cargo_bin!("re-x"))
}

#[test]
fn test_help() {
    re_x().arg("--help").assert().success();
}

#[test]
fn test_version() {
    re_x()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("re-x"));
}

#[test]
fn test_simple_match() {
    re_x()
        .args(["test", r"\d+", "hello 123 world"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"matched\": true"))
        .stdout(predicate::str::contains("\"text\": \"123\""));
}

#[test]
fn test_no_match() {
    re_x()
        .args(["test", r"\d+", "hello world"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"matched\": false"))
        .stdout(predicate::str::contains("\"match_count\": 0"));
}

#[test]
fn test_capture_groups() {
    re_x()
        .args(["test", r"(\d{3})-(\d{4})", "123-4567"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"captures\""))
        .stdout(predicate::str::contains("\"group\": 1"))
        .stdout(predicate::str::contains("\"group\": 2"));
}

#[test]
fn test_replace() {
    re_x()
        .args(["replace", r"\d+", "NUM", "a1b2c3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"result\": \"aNUMbNUMcNUM\""))
        .stdout(predicate::str::contains("\"replacements_made\": 3"));
}

#[test]
fn test_validate_valid() {
    re_x()
        .args(["validate", r"\d+"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\": true"));
}

#[test]
fn test_validate_invalid() {
    re_x()
        .args(["validate", r"(\d+"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\": false"));
}

#[test]
fn test_explain() {
    re_x()
        .args(["explain", r"\d+"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"parts\""))
        .stdout(predicate::str::contains("\"summary\""));
}

#[test]
fn test_from_examples() {
    re_x()
        .args(["from-examples", "2024-01-15", "2025-12-31"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"inferred\""))
        .stdout(predicate::str::contains("\"confidence\""));
}

#[test]
fn test_benchmark() {
    re_x()
        .args(["benchmark", r"\d+", "--input", "hello 123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"avg_us\""))
        .stdout(predicate::str::contains("\"catastrophic_backtracking\""));
}

#[test]
fn test_text_format() {
    re_x()
        .args(["test", r"\d+", "hello 123", "--format", "text"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pattern:"))
        .stdout(predicate::str::contains("Match 1:"));
}

#[test]
fn test_portability_check() {
    re_x()
        .args(["validate", r"(?=\d)\w+"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"portability\""))
        .stdout(predicate::str::contains("\"rust_regex\": false"));
}

// --- apply command tests ---

#[test]
fn test_apply_basic() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello world\nfoo bar\n").unwrap();

    re_x()
        .args([
            "apply",
            r"\bworld\b",
            "earth",
            "--file",
            file_path.to_str().unwrap(),
            "--no-backup",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"applied\": true"))
        .stdout(predicate::str::contains("\"replacements_made\": 1"));

    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "hello earth\nfoo bar\n");
}

#[test]
fn test_apply_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello world\n").unwrap();

    re_x()
        .args([
            "apply",
            "world",
            "earth",
            "--file",
            file_path.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"applied\": false"))
        .stdout(predicate::str::contains("\"replacements_made\": 1"));

    // File should NOT be modified
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "hello world\n");
}

#[test]
fn test_apply_multiline() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello\nworld\n").unwrap();

    re_x()
        .args([
            "apply",
            r"hello.world",
            "REPLACED",
            "--file",
            file_path.to_str().unwrap(),
            "--no-backup",
            "-m",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"applied\": true"))
        .stdout(predicate::str::contains("\"replacements_made\": 1"));

    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("REPLACED"));
}

// --- MCP server tests ---

#[test]
fn test_mcp_initialize() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    re_x()
        .arg("--mcp")
        .write_stdin(format!("{}\n", req))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"protocolVersion\""))
        .stdout(predicate::str::contains("re-x"));
}

#[test]
fn test_mcp_tools_list() {
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
    re_x()
        .arg("--mcp")
        .write_stdin(format!("{}\n{}\n", init, list))
        .assert()
        .success()
        .stdout(predicate::str::contains("regex_test"))
        .stdout(predicate::str::contains("regex_validate"))
        .stdout(predicate::str::contains("regex_explain"))
        .stdout(predicate::str::contains("regex_replace"))
        .stdout(predicate::str::contains("regex_apply"))
        .stdout(predicate::str::contains("regex_benchmark"))
        .stdout(predicate::str::contains("regex_from_examples"));
}

#[test]
fn test_mcp_tool_call_test() {
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    let call = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"regex_test","arguments":{"pattern":"\\d+","input":"hello 123"}}}"#;
    re_x()
        .arg("--mcp")
        .write_stdin(format!("{}\n{}\n", init, call))
        .assert()
        .success()
        .stdout(predicate::str::contains("\\\"matched\\\": true"))
        .stdout(predicate::str::contains("123"));
}

#[test]
fn test_mcp_tool_call_validate() {
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    let call = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"regex_validate","arguments":{"pattern":"\\d+"}}}"#;
    re_x()
        .arg("--mcp")
        .write_stdin(format!("{}\n{}\n", init, call))
        .assert()
        .success()
        .stdout(predicate::str::contains("\\\"valid\\\": true"));
}

#[test]
fn test_mcp_invalid_json() {
    re_x()
        .arg("--mcp")
        .write_stdin("not valid json\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("-32700"))
        .stdout(predicate::str::contains("Parse error"));
}

#[test]
fn test_mcp_unknown_method() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"nonexistent","params":{}}"#;
    re_x()
        .arg("--mcp")
        .write_stdin(format!("{}\n", req))
        .assert()
        .success()
        .stdout(predicate::str::contains("-32601"))
        .stdout(predicate::str::contains("Method not found"));
}

#[test]
fn test_mcp_ping() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#;
    re_x()
        .arg("--mcp")
        .write_stdin(format!("{}\n", req))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"result\""));
}

// --- replace --file tests ---

#[test]
fn test_replace_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("urls.txt");
    fs::write(
        &file_path,
        "http://example.com\nhttps://safe.com\nhttp://test.org\n",
    )
    .unwrap();

    re_x()
        .args([
            "replace",
            "http://",
            "https://",
            "--file",
            file_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"replacements_made\": 2"))
        .stdout(predicate::str::contains("\"preview\""));

    // replace --file never modifies the file
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("http://example.com"));
}

#[test]
fn test_replace_file_fancy_captures() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("data.txt");
    fs::write(&file_path, "price: 100USD\nprice: 200EUR\n").unwrap();

    // Lookahead pattern (fancy-regex) with capture group and $1 expansion
    re_x()
        .args([
            "replace",
            r"(\d+)(?=USD|EUR)",
            "[$1]",
            "--file",
            file_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"replacements_made\": 2"))
        .stdout(predicate::str::contains("\"preview\""));
}
