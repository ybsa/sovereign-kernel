# Sovereign Kernel: Security & Privacy Model

Sovereign Kernel is designed as a "Security-First" AI Operating System. This document outlines the security architecture, data handling practices, and approval mechanisms.

## Core Security Principles

1. **Deny-by-Default Capabilities**: Agents have zero access to the host system by default. Every action (file read, network, shell) requires an explicit `Capability` grant in the agent's manifest.

2. **Unified Approval Manager**: All tool executions pass through a single `ApprovalManager` with risk-based classification:
   - **Low** — Auto-approved (read-only operations)
   - **Medium** — Logged with audit trail (file writes, browser actions)
   - **High** — Requires explicit human approval (shell exec, file delete)
   - **Critical** — Always requires human intervention (code exec, host desktop control, destructive shell commands)

3. **Modular Tool Registry**: Tools are dispatched through a type-safe `ToolHandler` trait. Each tool is a registered handler that can be individually secured, audited, and replaced.

4. **Shell Bleed Protection**: Every script output is scanned for "Environment Bleed" to prevent API keys or secrets in the host environment from being leaked to the LLM.

5. **SSRF Protection**: Outbound network requests are checked against private IP ranges (localhost, 169.254.169.254, etc.) to prevent Server-Side Request Forgery.

## Credential Security

- **API keys are never stored in source code** or configuration files
- All keys are loaded from environment variables via `api_key_env` fields
- `config.toml` is gitignored to prevent accidental credential exposure
- Custom `Debug` implementations redact sensitive fields in log output
- The `Shell Bleed` scanner detects patterns like `_KEY=`, `_TOKEN=`, `_SECRET=` in agent-visible output

## Data Privacy

- **Zero Telemetry**: Sovereign Kernel does not "phone home." No data about your tasks, prompts, or files is sent anywhere.
- **User-Controlled Risk**: Toggle between `sandbox` (default, safe) and `unrestricted` modes in `config.toml`.
- **Budget Enforcement**: Hard token limits and USD budgets prevent cost overruns. See [SAFETY_CONTROLS.md](SAFETY_CONTROLS.md).
- **Audit Trails**: Every agent action is recorded in a tamper-evident Merkle chain for forensic analysis.

## Responsible Use

While Sovereign Kernel provides powerful autonomous tools, it is the user's responsibility to:
- Review agent capabilities before granting them
- Use `sandbox` mode for untrusted or experimental agents
- Rotate API keys regularly
- Never commit `.env` or `config.toml` to version control
