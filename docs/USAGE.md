# Sovereign Kernel — Usage Guide

## Table of Contents

- [Getting Started](#getting-started)
- [CLI Commands](#cli-commands)
- [Agent Configuration](#agent-configuration)
- [Memory Management](#memory-management)
- [Tool Reference](#tool-reference)
- [Execution Modes](#execution-modes)
- [Autonomous Scheduling](#autonomous-scheduling)
- [Hands (Capability Agents)](#hands-capability-agents)
- [Web Dashboard](#web-dashboard)
- [Audit Trail](#audit-trail)
- [Agent Identity (Soul Files)](#agent-identity-soul-files)
- [Token Efficiency (The Healer)](#token-efficiency-the-healer)
- [Configuration Reference](#configuration-reference)

---

## Getting Started

### 1. Build

```bash
cargo build --release

```

### 2. First-Time Setup

```bash
sovereign init

```

This interactive wizard will:

- Detect available LLM API keys in your environment
- Validate connectivity to your chosen provider
- Generate `~/.sovereign-kernel/config.toml`
- Set your default execution mode

### 3. Verify

```bash
sovereign doctor        # Checks API keys, system deps, connectivity

```

### 4. Run

| Command | Description |
| --- | --- |
| `sovereign chat` | Interactive terminal REPL |
| `sovereign chat --budget-usd 0.05` | Chat with an explicit spending cap |
| `sovereign run "<task>"` | Run a task autonomously |
| `sovereign run "<task>" --max-iterations 30` | Run with explicit loop limits |
| `sovereign do "<task>"` | Fast alias for `run` |
| `sovereign start` | Start as foreground daemon |
| `sovereign start --detach` | Start as background daemon |
| `sovereign status` | Village overview (agents, jobs, daemon status) |
| `sovereign kill <agent-id>` | Kill a specific agent |
| `sovereign kill` | Stop the daemon |
| `sovereign dashboard` | Open web dashboard (The Watchtower) |

---

## CLI Commands

### Core

```bash
sovereign init                            # First-run wizard
sovereign chat                            # Interactive REPL
sovereign run "<task>"                    # Autonomous task execution
sovereign run "<task>" --mode unrestricted # Full host access
sovereign run "<task>" --schedule "cron"   # Scheduled recurring task
sovereign start [--detach]                # Daemon lifecycle
sovereign status                          # Village overview
sovereign kill [<agent-id>]              # Stop agent or daemon
sovereign doctor                          # System diagnostics

```

### Memory

```bash
sovereign memory stats                    # Agent count, memory entries
sovereign memory export --format json     # Export brain to JSON
sovereign memory export --format markdown # Human-readable export
sovereign memory import --input <file>    # Restore brain from file

```

### Management

```bash
sovereign hands list                      # All 30+ bundled hands
sovereign hands activate <name>           # Start a capability hand
sovereign hands status                    # Running hand instances
sovereign dashboard [--port 8080]         # Embedded web dashboard
sovereign audit logs                      # Cryptographic audit trail
sovereign audit verify                    # Verify Merkle chain integrity
sovereign treasury status                 # Budget and spending
sovereign mcp list                        # Active MCP tool servers
sovereign mcp add <name> <cmd>            # Add MCP tool server
sovereign channels list                   # Configured channel adapters
sovereign usage                           # Cost report (The Chronicler)
sovereign tunnel --tailscale|--ssh        # Remote access (The Cartographer)

```

---

## Agent Configuration

### Providing API Keys

Drop keys in your `.env` file in the project root:

```env
ANTHROPIC_API_KEY=your-key     # Claude (recommended)
OPENAI_API_KEY=your-key        # GPT-4o
GEMINI_API_KEY=your-key        # Google Gemini
GROQ_API_KEY=your-key          # Llama 3 (free tier)
GITHUB_TOKEN=your-token        # GitHub Copilot

```

The kernel auto-detects which key is set. See `.env.example` for all supported variables.

### Budget Controls

Every session and task supports configurable limits:

```bash

# Per-session budget

sovereign chat --budget-usd 2.00 --max-tokens 50000 --max-iterations 50

# Per-task budget

sovereign run "task" --budget-usd 1.00 --max-iterations 30

```

- `--budget-usd` — Hard spending cap in USD
- `--max-tokens` — Maximum LLM tokens consumed
- `--max-iterations` — Maximum tool-call loop iterations

---

## Memory Management

### Architecture

The Archive provides four memory stores, all backed by a single SQLite database with WAL mode:

| Store | Engine | Description |
| --- | --- | --- |
| **Structured** | SQLite KV | Persistent key-value pairs (preferences, state) |
| **Semantic** | Vector cosine | Similarity-based memory recall with embeddings |
| **Full-Text** | FTS5 BM25 | Keyword search with ranking |
| **Knowledge** | Entity-Relation | Typed entities and relationships |

### CLI Operations

```bash

# View current memory statistics

sovereign memory stats

# Export everything to JSON (full data retention)

sovereign memory export --format json --output backup.json

# Export as human-readable Markdown

sovereign memory export --format markdown --output knowledge.md

# Import/restore from a previous export

sovereign memory import --input backup.json

```

> **Important:** JSON import provides full restoration of all data types. Markdown import only restores semantic memories (lossy).

### Agent Memory Tools

Agents use these tools for runtime memory operations:

| Tool | Description |
| --- | --- |
| `remember` | Store information in long-term memory |
| `recall` | Search memory by keyword (BM25) or similarity (vector) |
| `forget` | Delete a specific memory by ID |
| `memory_store` | Persistent key-value set |
| `memory_recall` | Persistent key-value get |
| `knowledge_add_entity` | Add entity to the knowledge graph |
| `shared_memory_store` | Store in cross-agent shared memory |
| `shared_memory_recall` | Search the global knowledge pool |

---

## Tool Reference

### Standard Tools (Available in Both Modes)

| Tool | Description |
| --- | --- |
| `shell_exec` | Run terminal commands (ExecPolicy enforced in Sandbox) |
| `file_read` / `file_write` | Read/write files (path-sandboxed) |
| `file_list` / `file_delete` | Browse and manage filesystem |
| `web_fetch` | Fetch and extract text from URLs |
| `web_search` | Search the web (requires `BRAVE_API_KEY` or `TAVILY_API_KEY`) |
| `code_exec` | Run scripts natively or in Docker sandbox |
| `browser_navigate` | Open URLs via native Chromium automation |
| `browser_click` / `browser_type` | Interact with web pages |
| `browser_screenshot` | Capture screenshots |
| `schedule_create` / `schedule_list` | Manage cron-based tasks |
| `get_skill` / `list_skills` | Access 106+ expert skill prompts |
| `agent_message` | Send message to another active agent |
| `text_to_speech` | Convert text to speech (OpenAI TTS-1) |
| `speech_to_text` | Transcribe audio (OpenAI Whisper) |
| `summon_skeleton` | Summon a sandboxed worker (The Witch) |
| `check_skeleton` | Check worker status |
| `builder` | Create a permanent Village member (Hand) |

### Host Tools (Unrestricted Mode Only)

| Tool | Risk Level | Description |
| --- | --- | --- |
| `host_desktop_control` | Medium | Wallpaper, dark mode, OS notifications |
| `host_system_config` | High | Read/edit system configs, manage services |
| `host_install_app` | Critical | Install apps via winget/apt/brew |
| `host_read_file` / `host_write_file` | High | Full filesystem access outside sandbox |
| `host_list_dir` | Low | List any directory on the host |

All host tools have risk-tiered approval gates.

---

## Execution Modes

### Sandbox Mode (Default)

- All file/shell actions require user approval
- File access restricted to workspace directory
- Shell commands must be on the allowlist

```bash
sovereign chat                    # Sandbox by default

```

### Unrestricted Mode

- No approval gates — agent acts autonomously
- Full filesystem and shell access
- Required for host tools

```bash
sovereign chat --mode unrestricted
sovereign run "task" --mode unrestricted

```

---

## Autonomous Scheduling

The kernel includes a built-in `CronScheduler` for background task execution.

### Requirements

The daemon must be running:

```bash
sovereign start --detach

```

### Usage

Tell your agent naturally:

> "Schedule a task to check the weather every hour and email me a summary at 8 AM."

Or via CLI:

```bash
sovereign run "Monitor disk usage" --schedule "*/30 * * * *"

```

The agent uses the `schedule_create` tool internally. The daemon monitors timing and wakes the agent automatically.

---

## Hands (Capability Agents)

Hands are autonomous capability packages with their own tool access:

```bash
sovereign hands list                  # Show all 30+ hands
sovereign hands activate browser      # Start browser automation
sovereign hands activate web-search   # Start web research
sovereign hands activate email        # Start email management
sovereign hands status                # Show running instances
sovereign hands deactivate <uuid>     # Stop a hand

```

### Bundled Hands

| Hand | Category | Description |
| --- | --- | --- |
| **browser** | Automation | CDP-based web browser automation |
| **researcher** | Research | Multi-source deep research with citations |
| **web-search** | Research | Brave/Tavily web search + reports |
| **clip** | Content | Clipboard-to-note capture |
| **collector** | Data | Structured data collection and archival |
| **lead** | Sales | Autonomous lead research |
| **predictor** | Analytics | Trend analysis and forecasting |
| **email** | Communication | SMTP/IMAP email management |
| **twitter** | Social | Twitter/X monitoring |
| **otto** | Builder | Docker-sandboxed code execution |
| **mysql-reporter** | Data | MySQL reporting and dashboards |
| **peka** | Terminal | The Terminal Master — shell expert |

### Required Environment Variables

| Hand | Required |
| --- | --- |
| **email** | `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `IMAP_HOST` |
| **web-search** | `BRAVE_API_KEY` or `TAVILY_API_KEY` |
| **twitter** | `TWITTER_API_KEY`, `TWITTER_API_SECRET` |

---

## Web Dashboard

The Watchtower is an embedded web dashboard — no Node.js, no npm, zero extra dependencies.

```bash
sovereign dashboard                   # Opens at http://localhost:8080
sovereign dashboard --port 9090       # Custom port
sovereign dashboard --no-open         # Don't auto-open browser

```

### Features

- **Three-pane layout**: agent panel, live log stream, command bar
- **Real-time monitoring**: uptime, active hands, approval queue
- **The Gatekeeper**: approve/deny blocked actions and network requests
- **WebChat**: embedded chat interface
- **Usage analytics**: cost breakdown by agent/model
- **REST API**: `/api/status`, `/api/hands`, `/api/agents`

---

## Audit Trail

Every agent action is logged in a tamper-evident cryptographic Merkle chain:

```bash
sovereign audit logs              # View recent audit entries
sovereign audit verify            # Verify chain integrity (detects tampering)

```

Each entry contains: agent ID, action type, timestamp, and SHA-256 chained hash.

---

## Agent Identity (Soul Files)

Edit `soul/SOUL.md` to customize your agent's name, personality, and behavioral constraints. The kernel loads this at startup and injects it into every agent's system prompt.

| File | Purpose |
| --- | --- |
| `soul/SOUL.md` | Agent personality, rules, available tools |
| `soul/IDENTITY.md` | Communication style, behavioral boundaries |
| `soul/AGENTS.md` | Village hierarchy and inter-agent roles |
| `soul/MEMORY.md` | Memory management configuration |
| `soul/USER.md` | Auto-populated user preferences |

---

## Token Efficiency (The Healer)

The kernel automatically manages token usage to prevent context overflow:

- **Smart Truncation**: Tool outputs exceeding 8,000 characters are trimmed to head + tail with a warning marker
- **Context Compaction**: At 80% budget, older messages are condensed into a "Ground-Truth State Manifest" stored in memory, keeping the 10 most recent messages raw

---

## Configuration Reference

### File: `~/.sovereign-kernel/config.toml`

```toml
[kernel]
mode = "Sandbox"                # "Sandbox" or "Unrestricted"
api_listen = "127.0.0.1:4200"   # API Bridge address

[model]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[budget]
max_iterations_per_task = 200
max_tokens_per_task = 128000
total_budget_usd = 5.00

[exec_policy]
allowed_commands = ["git", "cargo", "python", "node", "npm", "docker"]

```
