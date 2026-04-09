# Security Policy

## Supported Versions

| Version | Supported          |
|---------|-------------------|
| v1.0.x  | ✅ Active support  |
| `main`  | ✅ Active support  |
| < v1.0  | ❌ Unsupported     |

## Security Architecture

Sovereign Kernel provides multi-layered protection for LLM-driven agent operations:

### Unified Approval Manager
All dangerous operations pass through a single `ApprovalManager` with risk-based classification:
- **Low** — Auto-approved (read operations, web search)
- **Medium** — Logged, auto-approved with audit trail (file writes, browser navigation)
- **High** — Requires explicit human approval (shell exec, file delete)
- **Critical** — Always requires human intervention, cannot be auto-approved (code exec, host control, destructive shell commands)

### Tool Registry Security
- Every tool is dispatched through a type-safe `ToolHandler` trait
- Each tool call is classified by `ApprovalManager::classify_risk()`
- Atomic pending-count tracking prevents request exhaustion attacks

### Filesystem Sandbox
- Agents operate within a configured `workspaces_dir`
- Path traversal is validated before file operations
- Default execution mode is `sandbox` (requires approval for dangerous ops)

### Credential Security
- API keys are **never stored in source code or config files**
- All keys are loaded from environment variables via `api_key_env` fields
- `config.toml` is listed in `.gitignore` to prevent accidental key exposure
- Custom `Debug` implementations redact sensitive fields in logs

### Audit Trail
- Merkle chain (`AuditStore`) records every agent action
- Tamper-evident SHA-256 hash chain for forensic analysis

## Configuration Security

> **Important:** Always use `config.toml.example` as your starting template. The real `config.toml` is gitignored to prevent accidental exposure of API credentials.

```bash
cp config.toml.example config.toml
cp .env.example .env
# Edit both files with your specific values
```

## Reporting a Vulnerability

If you discover a security vulnerability, **DO NOT open a public issue**.

Instead:
1. Use GitHub's private vulnerability reporting feature, or
2. Email the project maintainers directly

We will:
- Acknowledge receipt within **48 hours**
- Provide regular progress updates
- Allow up to **90 days** for patching before public disclosure

Please do not share vulnerability details publicly until a patch has been released.
