# Vision — Sovereign Kernel

## The Idea

What if your AI assistant was an operating system?

Not a chatbot. Not a wrapper. Not a framework. A system that treats AI agents as first-class citizens the way Linux treats processes — where agents have identities (Soul Files), persistent memory, tools, security policies, crash recovery, and a cost budget.

Sovereign Kernel is that system, built in Rust, running entirely from your terminal.

---

## Why Rust?

- **Memory safety** — no segfaults, no data races. Critical for a system processing untrusted agent input 24/7.
- **Single binary** — `cargo build --release` produces one executable. No npm, no Python runtime, no Docker. Drop it on any machine and run `sovereign chat`.
- **Performance** — faster agent dispatch means fewer tokens burned per task.
- **Cross-platform** — the same codebase runs on Windows, macOS, and Linux.

---

## Terminal-First

No Electron. No browser dependency. No web dashboard (yet).

Sovereign Kernel runs from your terminal — like `git`, like `docker`. This is deliberate:

- Works over SSH
- Works in containers
- Works on headless servers
- Composes with other shell tools

---

## What Is Built (v1.0)

These subsystems are fully implemented, tested, and working:

| Subsystem | What it does | Home crate |
|---|---|---|
| **The Engine** | Agent execution loop — LLM call → tool use → iterate | `sk-engine` |
| **The Drivers** | Native LLM drivers for Anthropic, OpenAI, Gemini, Copilot | `sk-engine/drivers` |
| **The Archive** | 5-store memory substrate: KV, vectors, BM25, knowledge graph, hybrid ranking | `sk-memory` |
| **The Soul** | SOUL.md identity parser — injects persona into every agent | `sk-soul` |
| **The Warden** | Security sandbox, exec allowlist, subprocess env isolation | `sk-engine/runtime` |
| **The Treasury** | Token metering, cost estimation, budget enforcement | `sk-kernel/metering` |
| **The Tools** | 20+ built-in tools: shell, file ops, web search, web fetch, browser automation, memory | `sk-tools` |
| **The Nervous System** | MCP protocol client — connect any external tool server via stdio or SSE | `sk-mcp` |
| **The Hands** | 11 bundled autonomous capability packages (Researcher, Clip, Email, Otto, etc.) | `sk-hands` |
| **The Scheduler** | Cron jobs, background agent loops, auto-resurrection of crashed agents | `sk-kernel/cron` |
| **The Registry** | Live agent inventory — inspect, stop, remove running agents | `sk-kernel/registry` |
| **The Auditor** | Merkle-chained audit trail of every agent action | `sk-kernel/audit` |
| **The Resurrector** | Supervisor that restarts crashed agents on daemon restart | `sk-kernel/supervisor` |
| **The CLI** | Full `sovereign` binary: chat, run, start, stop, hands, soul, doctor, treasury, memory | `sk-cli` |

Test coverage: **821 tests passing** across 84 files.

---

## What Is Planned (Not Yet Built)

These are designed and partially scaffolded but not implemented:

**Messaging channel adapters** — Config schemas exist for 40+ platforms (Telegram, Discord, Slack, WhatsApp, Matrix, Signal, email, etc.) but no actual adapter code has been written. The kernel can't yet connect to any of these channels. This is the next major milestone.

**The Watchtower** — A web dashboard served at `localhost:8080` for viewing agent status, memory, and audit logs without using the CLI. Not started.

**The Diplomat** — Agent-to-Agent protocol for federated SK instances to collaborate across machines. A2A config exists in the types but no network transport is implemented.

**The Bazaar** — Community marketplace for publishing and sharing Hands (like pip/npm for agent capabilities). Not started.

**The Alchemist** — WASM and dynamic library plugin system. Not started.

---

## What "Terminal-First" Means Long-Term

The CLI will always be the primary interface. But the roadmap adds:

1. **Channel adapters** — so the kernel can receive tasks from Telegram, Discord, email, etc.
2. **The Watchtower** — a local web dashboard for non-CLI users
3. **The Diplomat** — so SK instances can collaborate across machines

The goal: make personal AI assistants as ubiquitous and reliable as web servers — running on your laptop, your server, your Raspberry Pi, available through whatever interface you prefer.
