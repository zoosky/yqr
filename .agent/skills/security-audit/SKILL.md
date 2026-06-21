---
name: security-audit
description: Run security audit on dependencies and review code for vulnerabilities. Use before releases or when updating dependencies.
---

# Security Audit

Audit dependencies and code for security vulnerabilities.

## Dependency Audit

```bash
# Run cargo-audit (requires: cargo install cargo-audit)
cargo audit
```

## What to Check

### Dependencies
- Known vulnerabilities in crates (CVEs)
- Outdated dependencies with security patches
- Unmaintained crates that should be replaced

### Code Review Checklist
1. **Input validation**: All user inputs sanitized
2. **Error handling**: No sensitive info leaked in errors
3. **Unsafe code**: Minimal and well-documented unsafe blocks
4. **File operations**: Path traversal protection
5. **Secrets**: No hardcoded credentials or API keys

## Common Issues in Rust Web Apps

- Path traversal in file serving (`../` attacks)
- Template injection in user content
- Denial of service via large payloads
- Missing rate limiting
- CORS misconfiguration

## When to Use

- Before releases
- When adding new dependencies
- During code review
- After `cargo update`
