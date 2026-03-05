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
| `sovereign status` | Check if daemon is running |
| `sovereign stop` | Stop the daemon |
| `sovereign dashboard` | Open terminal web dashboard |
| `sovereign hands list` | List all 9 bundled hands |
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

## 🖐️ Managing Hands

Hands are autonomous capability agents you can activate:

```bash
sovereign hands list                  # Show all 9 hands with requirements
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

## 🧰 Built-In Tools Available to Agents

| Tool | Description |
|------|-------------|
| `shell_exec` | Run terminal commands (ExecPolicy enforced in Sandbox) |
| `file_read` / `file_write` | Read/write files (path-sanitized) |
| `file_list` / `file_delete` | Browse and manage filesystem |
| `web_fetch` | Fetch and extract text from URLs |
| `web_search` | Search the web (requires BRAVE_API_KEY or TAVILY_API_KEY) |
| `code_exec` | Run Python, Node.js, or Bash scripts |
| `browser_navigate` | Open URLs in Playwright browser |
| `browser_click` / `browser_type` | Interact with web pages |
| `browser_screenshot` | Capture screenshots |
| `memory_store` / `memory_recall` | Persistent key-value memory |
| `knowledge_add_entity` | Add to knowledge graph |
| `schedule_create` / `schedule_list` | Manage cron-based tasks |
| `get_skill` / `list_skills` | Access 52 expert skill prompts |
| `agent_message` | Send a direct message to another active agent |
| `agent_spawn_worker` | Spawn a sandboxed background worker agent |
| `agent_check_worker` | Check the status of a spawned worker |
| `shared_memory_store` | Store facts in global shared memory |
| `shared_memory_recall` | Search the global shared knowledge pool |

---

## 📊 Audit Trail

Every agent action is cryptographically logged in a Merkle chain:
```bash
sovereign audit logs              # View recent audit entries
sovereign audit verify            # Verify chain integrity (detects tampering)
```

Audit entries include: agent ID, action type, timestamp, and SHA-256 chained hash.

---

## 🤖 Multi-Agent Coordination

Agents can now communicate, delegate tasks, and share knowledge across the swarm.

### Agent Messaging
Agents send direct messages to each other via the **Inter-Agent Bus**. Messages are persisted in the recipient's session — they'll see them on their next activation.

### Worker Spawning
A manager agent can spawn a background worker for parallel tasks:
- Workers are **forced into Sandbox mode** — every action requires your approval
- Use `agent_check_worker` to poll the worker's progress
- Workers message results back to the manager via `agent_message`

### Shared Memory
Agents with the `SharedMemory` capability can store and recall facts from a global knowledge pool, enabling cross-agent context sharing.

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
