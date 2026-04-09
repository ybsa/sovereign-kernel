# Sovereign Kernel — Architecture

## Overview

Sovereign Kernel is a **9-crate Rust workspace** implementing an AI Operating System with local-first design. It provides a modular, strategy-based framework for building autonomous agents with deep memory, tool execution, and multi-provider LLM support.

---

## Workspace Structure

```text
sovereign-kernel/
├── Cargo.toml              # Workspace root (9 members)
├── Cargo.lock
├── config.toml.example     # Configuration template
├── .env.example            # Environment variable template
├── Dockerfile              # Multi-stage production build
├── docker-compose.yml      # One-command deployment
├── soul/                   # Agent identity files
│   ├── SOUL.md             # Agent personality and rules
│   ├── IDENTITY.md         # Communication style
│   ├── AGENTS.md           # Village hierarchy
│   ├── MEMORY.md           # Memory configuration
│   └── USER.md             # Auto-populated user preferences
├── docs/                   # Project documentation
└── crates/                 # Rust workspace members
    ├── sk-types/            # Shared types and traits
    ├── sk-soul/             # Agent identity parser
    ├── sk-memory/           # Memory substrate (SQLite + BM25 + vectors)
    ├── sk-engine/           # LLM orchestration + agent runtime
    ├── sk-mcp/              # Model Context Protocol integration
    ├── sk-kernel/           # Core daemon: security, tools, event bus
    ├── sk-tools/            # Tool implementations (shell, file, code exec)
    ├── sk-hands/            # Agent capability packages
    └── sk-cli/              # CLI surface
```

---

## Crate Dependency Graph

```text
                     sk-cli
                    /  |   \
                   /   |    \
            sk-kernel  |  sk-hands
             / |  \    |    /
            /  |   \   |   /
      sk-engine|
         |  \  |
         |   \ |
      sk-mcp sk-tools
         |      |
         |      |
      sk-memory |
         |  \   |
         |   \  |
      sk-soul   |
         \      |
          \     |
          sk-types
```

---

## Crate Details

### sk-types

Foundation crate. Provides shared types, error definitions, capability-based security gates, taint tracking, and configuration schema.

**Key exports:** `AgentId`, `SovereignError`, `SovereignResult`, `KernelConfig`, `ToolDefinition`, `Capability`, `RiskLevel`

---

### sk-soul — Agent Identity

Parses Soul Files (`SOUL.md`, `AGENTS.md`, `IDENTITY.md`) to construct agent personalities, workspace prompts, and behavioral constraints. Uses YAML frontmatter for structured metadata.

---

### sk-memory — The Archive

The memory substrate. All data is stored in a single SQLite database with WAL mode for concurrent access.

| Component | Role |
| --- | --- |
| `MemorySubstrate` | Central hub implementing the `Memory` trait |
| `StructuredStore` | SQLite key-value store |
| `SemanticStore` | Vector embeddings with cosine similarity |
| `Bm25Index` | FTS5 full-text search with BM25 ranking |
| `KnowledgeStore` | Entity-relation triples |
| `SessionStore` | Conversation persistence |
| `SharedMemoryStore` | Global knowledge accessible to all agents |
| `AuditStore` | Merkle chain integrity |
| `CheckpointStore` | Crash recovery state |

---

### sk-engine — LLM Orchestration

The intelligence layer. Manages LLM calls, agent loops, and resource accounting.

| Component | Role |
| --- | --- |
| **Driver Catalog** | 50+ LLM providers: Anthropic, OpenAI, Gemini, Groq, NVIDIA, DeepSeek, xAI, Ollama, OpenRouter, Mistral, and more |
| **Agent Loop** | Core reasoning loop with tool calling, streaming, and checkpointing |
| **The Healer** | Context compaction at 80% token capacity |
| **Strategies (ADK)** | Pluggable reasoning strategies (ReAct, Plan-Execute, etc.) |
| **Sandbox Runtime** | Docker-based and subprocess sandboxing |

---

### sk-mcp — Model Context Protocol

External integration layer for MCP server/client connectivity. Supports `stdio` and `SSE` transports.

---

### sk-kernel — The Operating System Core

The daemon core managing lifecycle, security, and tool dispatch.

| Component | Role |
| --- | --- |
| **Tool Registry** | Modular `ToolHandler` trait-based dispatch system. Each tool is a registered handler. |
| **Approval Manager** | Unified risk-based gating with atomic pending-count tracking. Consolidates legacy SafetyGate. |
| **Event Bus** | Kernel-wide notification system (`KernelEvent` enum) |
| **Agent Registry** | Agent lifecycle management (register, status, kill) |
| **Inter-Agent Bus** | Direct message routing between agents via sessions |
| **Cron Scheduler** | Persistent background job scheduling |
| **Supervisor** | Process monitoring and crash recovery |
| **Config Reload** | Hot-reload of configuration changes |
| **Metering Engine** | Real-time cost tracking per agent with budget enforcement |
| **Librarian** | Background semantic indexer for codebase awareness |

**Tool modules** (`sk-kernel/src/tools/`):
- `shell_exec` — Shell command execution
- `file_ops` — Read, write, delete, move, copy files
- `code_exec` — Sandboxed code execution (Python, Node, Bash)
- `browser` — Browser automation
- `memory` / `shared_memory` — Agent memory tools
- `skills` — Skill registry access
- `host` — Host-level operations (desktop control, system config)
- `agent_tools` — Inter-agent communication
- `builder` — Hand/capability package setup
- `web_tools` — Web search and fetch
- `repo_search` — Repository-aware code search

---

### sk-tools — Tool Definitions & Implementations

Provides tool definition schemas and core implementation functions used by sk-kernel's tool registry.

| Module | Role |
| --- | --- |
| `shell` | Shell execution with policy enforcement |
| `file_ops` | Atomic file operations with path sandboxing |
| `code_exec` | Language-aware code execution |
| `web_search` / `web_fetch` | Web search and content retrieval |
| `memory_tools` | Remember, recall, forget |
| `shared_memory` | Cross-agent shared knowledge |
| `skills` | 100+ expert prompts loaded from `skills/*/SKILL.md` |
| `host` | Desktop control, system config, app management |

---

### sk-hands — Capability Packages

Pre-built agent capability bundles. Each Hand defines a specialized agent role with tools, prompts, and environment requirements.

---

### sk-cli — CLI Surface

The user-facing command-line interface.

**Key commands:** `chat`, `run`, `init`, `status`, `kill`, `hands`, `doctor`, `memory`

---

## Data Flow

```text
User sends message (CLI / Runner)
    → sk-kernel routes to agent session
    → sk-memory loads session transcript
    → sk-engine selects LLM provider + model
    → Agent loop executes with tool calls
    → Tool Registry dispatches to appropriate ToolHandler
    → Approval Manager gates dangerous operations
    → Metering Engine tracks cost
    → Audit Store records action to Merkle chain
    → Response flows back to terminal
```

---

## Build & Test

```bash
# Build everything
cargo build --workspace --release

# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets

# Format
cargo fmt --all
```
