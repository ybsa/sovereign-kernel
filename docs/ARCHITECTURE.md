# 🏛️ Sovereign Kernel Architecture (v0.1.0)

The Sovereign Kernel is a virtual operating system for AI agents, built with a microkernel-inspired architecture in Rust. It is a **10-crate Cargo workspace** engineered for modularity, security, and long-running autonomous operation.

---

## 📦 Crate Overview

### 1. `sk-types` — The Core Ledger
Defines all shared data structures: `Message`, `ToolCall`, `ToolDefinition`, `AgentManifest`, `SovereignError`, `KernelConfig`, and capability tracking. Ensures type safety and taint propagation across crate boundaries.

### 2. `sk-engine` — The Brain
Manages the agent reasoning loop, LLM driver abstraction, tool execution pipeline, and browser automation.
- **LLM Drivers**: `AnthropicDriver`, `OpenAIDriver` (GPT-4o + Groq), `GeminiDriver`, `CopilotDriver`, `FallbackDriver`
- **Tool Runner**: Dispatches validated tool calls to handlers in `sk-tools`
- **Loop Guards**: `loop_guard`, `tool_policy`, and `session_repair` prevent autonomous runaway
- **Browser**: `BrowserManager` with full Playwright CDP bridge for web automation

### 3. `sk-kernel` — The Supervisor
The OS monitor that manages agent lifecycles and system-level concerns.
- **CronScheduler**: A background daemon thread (`start_background_services`) polling for due agent alarms and spawning dynamic wake-up sessions.
- Heartbeat monitor with configurable timeouts
- Config hot-reload without daemon restart
- **Inter-Agent Bus** (`bus.rs`): Persistent message routing between agents
- **Worker Spawning**: Dynamic sub-agent creation with forced Sandbox mode
- OpenAI-compatible `/v1/chat/completions` API bridge
- Pairing/auth system for secure remote agent access

### 4. `sk-soul` — The Identity
Parses `SOUL.md` files to inject consistent agent persona, behavioral directives, and ethical constraints into the reasoning engine at runtime.

### 5. `sk-memory` — The Substrate
A unified memory system for persistent agent state.
- **SQLite** for sessions, key-value store, and audit logs
- **BM25 full-text search** for semantic memory recall
- **Knowledge Graph** for entity-relation storage
- **Shared Semantic Memory** (`shared.rs`): Global knowledge pool accessible across all authorized agents
- **Cryptographic Merkle Audit Trail** — every action is SHA-256 chained; any tampering is detectable

### 6. `sk-mcp` — The Nervous System
A native Rust implementation of the Model Context Protocol (MCP). Allows the kernel to consume tools from any MCP-compatible server and expose its own tools as an MCP server via JSON-RPC 2.0.

### 7. `sk-tools` — The Tool Implementations
Implements the concrete action handlers available to agents.
- **Shell Hand** (`shell.rs`): Commands with per-tool timeout, working directory scoping, stdout/stderr capture. Enforces an `ExecPolicy` allowlist in Sandbox mode.
- **File Hand** (`file_ops.rs`): Full filesystem operations — read (1 MB limit), write, append, delete, move, copy, list with rich metadata. Path validation prevents traversal attacks.
- **Web Hand** (`web_fetch.rs`): Fetches web pages via `reqwest` with automatic HTML-to-text stripping and response truncation.
- **Code Hand** (`code_exec.rs`): Sandboxed script runner for Python, Node.js, and Bash with configurable timeouts and policy gating.
- **Scheduler Hand** (`scheduler.rs`): Exposes `schedule_create`, `schedule_list`, and `schedule_delete` for autonomous time-based triggers.
- **Skills System**: Dynamic registry of **52 expert skills** (Obsidian, GitHub, Weather, etc.) parsed from `SKILL.md` files — on-demand instructions without prompt bloat.

### 8. `sk-hands` — Autonomous Capability Packages
Pre-built, self-contained agents called **Hands** — each with a validated `HAND.toml`, `SKILL.md`, tool list, requirements checker, dashboard metrics, and agent prompt.

**10 Bundled Hands:**
| Hand | Category | Core Tools |
|------|----------|------------|
| `browser` | Automation | `browser_navigate`, `browser_click`, `browser_type`, `browser_screenshot` |
| `researcher` | Research | `web_search`, `web_fetch`, `knowledge_*`, `memory_*` |
| `web-search` | Research | `web_search`, `web_fetch` (Brave/Tavily API) |
| `clip` | Content | `memory_store`, `file_write`, `knowledge_add_entity` |
| `collector` | Data | `web_fetch`, `file_write`, `knowledge_*`, `schedule_*` |
| `lead` | Sales | `web_search`, `web_fetch`, `knowledge_*`, `file_write` |
| `predictor` | Analytics | `web_search`, `memory_*`, `knowledge_*` |
| `email` | Communication | `shell_exec` (SMTP/IMAP via Python), `schedule_*` |
| `twitter` | Social | `web_fetch`, `shell_exec`, `knowledge_*` |

Hands are managed via the CLI:
```bash
sovereign hands list
sovereign hands activate web-search
sovereign hands status
```

### 9. `sk-channels` — The Channel Bridge
30+ channel adapters for Telegram, Discord, WhatsApp, Signal, Slack, iMessage, and more. Each adapter implements `ChannelBridgeHandle` for a uniform message routing interface.

### 10. `sk-cli` — The Shell + Dashboard
The user-facing binary (`sovereign`) and the embedded terminal web dashboard.
- **CLI**: `init`, `chat`, `start`, `stop`, `status`, `hands`, `audit`, `dashboard` subcommands
- **Dashboard**: Full Axum HTTP server with embedded frontend (no Node.js required)
  - Terminal aesthetic: jet black, Geist Mono, green/cyan/amber accents
  - Three-pane layout: agents/hands panel | live log stream | command bar
  - REST API: `/api/status`, `/api/hands`, `/api/agents`, `/api/audit/recent`
  - OpenAI-compatible: `/v1/chat/completions`, `/v1/models`

---

## 🤖 Multi-Agent Coordination

| Component | Location | Purpose |
|-----------|----------|---------|
| **Inter-Agent Bus** | `sk-kernel/src/bus.rs` | Persistent message routing — messages saved to target agent's SQLite session |
| **Worker Spawning** | `sk-kernel/src/executor.rs` | Dynamic sub-agent creation via `SetupWizard`, auto-forced to Sandbox mode |
| **Shared Memory** | `sk-memory/src/shared.rs` | Global `global_knowledge` table accessible by agents with `SharedMemory` capability |
| **Capability Gate** | `sk-types/src/capability.rs` | `Capability::SharedMemory` controls access to the global knowledge pool |

```
Manager Agent
    ├── spawn_witch_skeleton("researcher", "Find X")
    │       └── Worker Agent (Sandbox Mode)
    │           ├── web_search("X")
    │           └── agent_message(manager_id, "Found X: ...")
    └── check_witch_skeleton(witch_id)
            └── "Status: completed"
```

---

## 🛡️ Security Model

| Layer | Mechanism |
|-------|-----------|
| **Capability Gates** | Tool access controlled by `sk-types::Capability` — declared in agent manifest |
| **Sandbox Policy** | `ExecPolicy` allowlist restricts shell and file operations in Sandbox mode |
| **Path Sanitization** | All file operations validated against workspace root; traversal attacks blocked |
| **Taint Tracking** | Network-fetched data tagged as tainted; cannot be directly executed |
| **Approval Gate** | Risky tool calls paused for human approval before execution |
| **Merkle Audit Trail** | Every action SHA-256 chained in SQLite; tampering is cryptographically detectable |
| **Hardened Core** (v0.1.0) | Zero-warning Clippy state, 100% test coverage, and strict MSRV 1.75.0 compliance |

---

## ⚡ Execution Flow

```
User / Channel Input
       ↓
   ChannelBridge (sk-channels)
       ↓
   SovereignBridge.send_message()
       ↓
   SovereignKernel.run_agent()
       ↓
   sk-engine: Agent Loop
   ┌──────────────┐
   │ 1. Build prompt (soul + memory + tools)
   │ 2. Call LLM driver → stream response
   │ 3. Parse tool calls
   │ 4. Check capability gate
   │ 5. [Sandbox] → ExecPolicy check
   │ 6. Execute tool via sk-tools
   │ 7. Append result to conversation
   │ 8. Log to Merkle audit trail
   │ 9. Repeat until done
   └──────────────┘
       ↓
   Response → User / Channel
```

---

## 🗂️ Directory Structure

```
sovereign-kernel/
├── Cargo.toml              # Workspace manifest
├── README.md
├── ARCHITECTURE.md
├── SECURITY.md
├── USAGE.md
├── VISION.md
├── soul/
│   └── SOUL.md             # Agent identity definition
├── docs/
│   └── PROJECT_PLAN.md     # Full 26-week roadmap
├── config.toml             # Default kernel config
├── config.unrestricted.toml
└── crates/
    ├── sk-types/
    ├── sk-engine/
    ├── sk-kernel/
    ├── sk-soul/
    ├── sk-memory/
    ├── sk-mcp/
    ├── sk-tools/
    │   └── skills/         # 52 bundled expert skills
    ├── sk-hands/
    │   └── bundled/        # 10 bundled autonomous hands
    ├── sk-channels/
    └── sk-cli/
        └── static/         # Embedded dashboard frontend
```
