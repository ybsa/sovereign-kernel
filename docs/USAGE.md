# 📖 Sovereign Kernel — Usage Guide

## 🚀 Quick Start

### 1. Build
```bash
cargo build --release
```

### 2. First-Time Setup
```bash
./target/release/sovereign init       # Interactive setup wizard
```
Sets up your LLM API key, execution mode, and agent identity.

### 3. Run

| Command | What It Does |
|---------|-------------|
| `sovereign chat` | Interactive terminal REPL |
| `sovereign start` | Start as background daemon |
| `sovereign run "<task>"` | Run a task autonomously (NL Builder auto-detects mode) |
| `sovereign run "<task>" --schedule "..."` | Schedule a recurring cron task |
| `sovereign status` | Village overview — agents, jobs, daemon status |
| `sovereign kill <agent-id>` | Kill a specific agent |
| `sovereign kill` | Stop the daemon |
| `sovereign stop` | Stop the daemon (legacy) |
| `sovereign dashboard` | Open terminal web dashboard |
| `sovereign hands list` | List all 10 bundled hands |
| `sovereign audit logs` | View cryptographic audit trail |

---

## 🛠️ Setting Your LLM API Key

Add to your `.env` file in the project root (the kernel auto-detects which key is set):

```env
ANTHROPIC_API_KEY=your-key   # Claude (recommended)
OPENAI_API_KEY=your-key      # GPT-4o
GEMINI_API_KEY=your-key      # Google Gemini
GROQ_API_KEY=your-key        # Llama 3 (free tier)
GITHUB_TOKEN=your-token      # GitHub Copilot
```

---

## 🖥️ Terminal Web Dashboard

```bash
sovereign dashboard              # Opens at http://localhost:8080
sovereign dashboard --port 9090  # Custom port
sovereign dashboard --no-open    # Don't auto-open browser
```

The dashboard is **fully embedded in the binary** — no Node.js, no npm.

Features:
- Live agent and hand status panel
- Real-time log stream (every tool call and result)
- Approval queue for risky actions
- REST API at `/api/status`, `/api/hands`, `/api/agents`

---

## ⏰ Autonomous Background Scheduling (Phase 16)

Agents possess a `CronScheduler` that allows them to book tasks for the future in the background.

To use background scheduling, **you must have the daemon running**:
```bash
sovereign start
```

Once running, you can connect via `sovereign chat` and tell your agent to:
> "Schedule a task to check the weather every 1 hour and email me a summary at 8 AM."

The agent will use the `schedule_create` tool. The daemon will monitor the time in the background and automatically wake the agent up precisely when the task is due, completely seamlessly.

---

## 🖐️ Managing Hands

Hands are autonomous capability agents you can activate:

```bash
sovereign hands list                  # Show all 10 hands with requirements
sovereign hands activate browser      # Start the browser automation hand
sovereign hands activate web-search   # Start the web research hand
sovereign hands activate email        # Start the email management hand
sovereign hands status                # Show all running hand instances
sovereign hands deactivate <uuid>     # Stop a specific hand
```

### Setting Up the Email Hand
Add to your `.env`:
```env
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USER=you@gmail.com
SMTP_PASS=your-app-password
IMAP_HOST=imap.gmail.com
```

### Setting Up the Web Search Hand
Add to your `.env`:
```env
BRAVE_API_KEY=your-key    # or TAVILY_API_KEY
```

---

## 🛡️ Execution Modes

### Sandbox Mode *(Default — Recommended)*
- All file/shell actions require approval (`config.toml`: `mode = "Sandbox"`)
- File access restricted to workspace directory
- Shell commands must be on the allowlist (`exec_policy.allowed_commands`)

### Unrestricted Mode
- No approval gates — agent acts autonomously
- Full filesystem and shell access
- Use `config.unrestricted.toml` for trusted local automation:
```bash
sovereign --config config.unrestricted.toml chat
```

---

## 🧰 The Laboratory: Built-In Tools Available to Agents

| Laboratory Tool | Description |
|------|-------------|
| `shell_exec` | Run terminal commands (ExecPolicy enforced in Sandbox). Supports `use_sandbox` flag for Docker. |
| `file_read` / `file_write` | Read/write files (path-sanitized) |
| `file_list` / `file_delete` | Browse and manage filesystem |
| `web_fetch` | Fetch and extract text from URLs |
| `web_search` | Search the web (requires BRAVE_API_KEY or TAVILY_API_KEY) |
| `code_exec` | Run scripts natively or in Docker (supports `use_sandbox` flag). |
| `browser_navigate` | Open URLs in Playwright browser |
| `browser_click` / `browser_type` | Interact with web pages |
| `browser_screenshot` | Capture screenshots |
| `memory_store` / `memory_recall` | Persistent key-value memory |
| `knowledge_add_entity` | Add to knowledge graph |
| `schedule_create` / `schedule_list` | Manage cron-based tasks |
| `get_skill` / `list_skills` | Access 52 expert skill prompts |
| `agent_message` | Send a direct message to another active agent |
| `spawn_witch_skeleton` | Spawn a sandboxed background witch_skeleton |
| `check_witch_skeleton` | Check the status of a spawned witch_skeleton |
| `shared_memory_store` | Store facts in global shared memory |
| `shared_memory_recall` | Search the global shared knowledge pool |
| `builder` | Create an agent from a natural language task description |
| `host_desktop_control` | Change wallpaper, toggle dark mode, OS notifications (Unrestricted) |
| `host_system_config` | Read/edit system configs, manage services (Unrestricted) |
| `host_install_app` | Install apps via winget/apt/brew (Unrestricted, Critical risk) |
| `host_read_file` / `host_write_file` | Full filesystem access outside sandbox (Unrestricted) |
| `host_list_dir` | List any directory on the host (Unrestricted) |

---

## 📊 Audit Trail

Every agent action is cryptographically logged in a Merkle chain:
```bash
sovereign audit logs              # View recent audit entries
sovereign audit verify            # Verify chain integrity (detects tampering)
```

Audit entries include: agent ID, action type, timestamp, and SHA-256 chained hash.

---

## 🏘️ The Village — Agent Ecosystem

Sovereign Kernel is a living **Agent Village** — agents communicate, delegate, recover from crashes, and can be spawned from plain English.

### Natural Language Builder
Instead of writing agent manifests, just describe what you want:
```bash
sovereign run "Monitor my inbox and summarize new emails every hour"
```
The kernel auto-detects the right tools, mode, and scheduling.

### Crash Recovery (The Resurrector)
Agents save checkpoints every 30 seconds. If an agent crashes, the Supervisor automatically restarts it from the last checkpoint with a `[Resurrector] Restarted from checkpoint` system message.

### Host Tools (Unrestricted Mode)
In unrestricted mode, agents gain full host access:
- Desktop control (wallpaper, dark mode, notifications)
- System configuration
- App installation (`winget`/`apt`/`brew`)
- Unrestricted filesystem access

All host tools have **risk-tiered approval gates** (Low → Medium → High → Critical).

### Agent Messaging & Spawning
Agents send direct messages via the **Inter-Agent Bus**. Manager agents spawn sandboxed witch_skeletons for parallel tasks. Shared Memory enables cross-agent knowledge sharing.

---

## ⚕️ The Healer (Token Efficiency)

Sovereign Kernel natively implements **The Healer**, a background mechanism designed to keep token usage low and prevent model hallucination during long tasks:
- **Smart Truncation**: If a tool like `shell_exec` returns over 8,000 characters, The Healer injects a warning marker and preserves only the head and tail.
- **Conversation Compaction**: When the context window fills up (80% budget), the agent automatically condenses older messages into a "Ground-Truth State Manifest" stored in memory, while keeping the most recent 10 messages raw for immediate context.

---

## 🧬 Agent Identity

Edit `soul/SOUL.md` to customize your agent's name, personality, and behavioral constraints. The kernel loads this at startup and injects it into every agent's system prompt.

---

## ⚙️ Configuration Reference

`config.toml` (default location: `~/.sovereign/config.toml`):

```toml
[kernel]
mode = "Sandbox"          # or "Unrestricted"

[model]
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"

[exec_policy]
allowed_commands = ["git", "cargo", "python", "node", "npm"]
```
