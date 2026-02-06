# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in re-x, please report it responsibly.

**Do NOT open a public issue.**

Instead, use [GitHub Security Advisories](https://github.com/re-x-ai/re-x/security/advisories/new) to report privately.

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

We aim to respond within 48 hours and release a fix within 7 days for critical issues.

## Scope

re-x processes user-supplied regex patterns. Key security considerations:

- **ReDoS**: Patterns can cause catastrophic backtracking. Use `re-x benchmark` to detect this before using patterns on untrusted input.
- **File operations**: The `apply` command writes to files. It creates `.bak` backups by default.
- **MCP server**: Runs locally via stdio. Does not open network ports (in stdio mode).
