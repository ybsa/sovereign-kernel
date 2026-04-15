# Sovereign Kernel

![Rust](https://img.shields.io/badge/language-Rust-orange?style=flat-square)
![MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Tests](https://img.shields.io/badge/tests-821%20passing-brightgreen?style=flat-square)

> A terminal-first agentic OS. Single Rust binary. Agents with identity, memory, tools, scheduling, and security.

Sovereign Kernel turns any LLM into an autonomous agent that can search the web, manipulate files, execute shell commands, run background tasks, and remember context — all from a single binary with built-in security sandboxing.

```
┌─────────────────────────────────────────────────────────────┐
│                  Sovereign Kernel (Rust)                    │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  4 LLM   │  │ Terminal │  │ 20+ Tools│  │ Security │   │
│  │ Drivers  │  │   CLI    │  │ Built-in │  │ Sandbox  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Memory  │  │  Agent   │  │  11 Hand │  │  Cron    │   │
│  │ 5 Stores │  │ Registry │  │ Packages │  │Scheduler │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## Requirements

- Rust stable ≥ 1.75 — `rustup install stable`
- At least one LLM API key

## Build

```bash
git clone <repo>
cd sovereign-kernel

# Development build
cargo build

# Production binary (single executable, LTO-optimised)
cargo build --release
# Binary at: target/release/sovereign (or sovereign.exe on Windows)
```

## Configuration

Create a `config.toml` in the project root:

```toml
log_level = "info"
execution_mode = "sandbox"   # or "unrestricted" for full host access

[[llm]]
provider = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-5"

[exec_policy]
mode = "allowlist"           # or "full" for unrestricted shell

[memory]
decay_rate = 0.1
```

See `examples/config/` for full sandbox and unrestricted reference configs.

Set your API key in the environment or a `.env` file (loaded automatically):

```bash
export ANTHROPIC_API_KEY=sk-ant-...
# or write it to .env
```

---

## LLM Providers

Four providers have native drivers built in. Any OpenAI-compatible endpoint also works by setting `base_url`.

| Provider | `provider` value | Key env var | Notes |
|---|---|---|---|
| Anthropic Claude | `anthropic` | `ANTHROPIC_API_KEY` | Native driver, prompt caching |
| OpenAI | `openai` | `OPENAI_API_KEY` | Native driver, prompt caching |
| Google Gemini | `gemini` | `GEMINI_API_KEY` | Native driver |
| GitHub Copilot | `copilot` | `GITHUB_TOKEN` | Native driver |
| Any OpenAI-compat | any string | your env var | Set `base_url` in config |

OpenAI-compatible example (NVIDIA NIM, Groq, Ollama, etc.):

```toml
[[llm]]
provider = "nvidia"
api_key_env = "NVIDIA_API_KEY"
model = "meta/llama-3.3-70b-instruct"
base_url = "https://integrate.api.nvidia.com/v1"
```

---

## Usage

### Interactive chat

```bash
sovereign chat
sovereign chat --max-tokens 8000 --budget-usd 0.50
```

### Run a task autonomously

```bash
sovereign run "summarise the top Hacker News stories today and save to summary.md"
sovereign run "refactor src/main.rs to use async/await" --mode unrestricted
```

### Background daemon

```bash
sovereign start           # foreground, Ctrl+C to stop
sovereign start --detach  # background (logs → data/logs/daemon.json)
sovereign status
sovereign stop
```

### First-time setup wizard

```bash
sovereign setup
```

### Soul — agent persona

```bash
sovereign soul list
sovereign soul create
sovereign soul activate <name>
```

Create `soul/SOUL.md` to give the kernel a persistent identity injected into every agent's system prompt:

```yaml
---
name: "Aria"
role: "Personal AI assistant"
---

You are Aria, a personal AI assistant running on Sovereign Kernel.
```

---

## Hands — autonomous background agents

Hands run continuously for you. You activate them and they work on their own.

```bash
sovereign hands list
sovereign hands activate researcher
sovereign hands status
sovereign hands deactivate researcher
```

**Bundled hands:**

| Hand | What it does | Key requirements |
|---|---|---|
| `researcher` | Web search + browser verification + structured reports | `web_search` API key |
| `clip` | Video download, transcription, ffmpeg clipping | `ffmpeg`, `yt-dlp`, `whisper` |
| `email` | Read, draft, and send email (IMAP/SMTP) | Email credentials |
| `web-search` | Continuous search monitoring and alerts | `web_search` API key |
| `collector` | Data collection pipeline with scheduling | — |
| `lead` | Lead research and knowledge graph | `web_search` API key |
| `predictor` | Trend analysis and forecasting | — |
| `artificer` | Skill and tool building | — |
| `otto` | Coding agent — compiles and runs Rust skills | Rust toolchain |
| `mysql-reporter` | Scheduled MySQL reports via email | MySQL, Himalaya |
| `twitter` | Tweet drafting, scheduling, engagement | Twitter API keys |

---

## Built-in tools

Every agent has access to these tools out of the box:

| Tool | What it does |
|---|---|
| `shell_exec` | Run shell commands (sandboxed or unrestricted) |
| `read_file`, `write_file`, `list_dir` | File system operations |
| `web_search` | Brave / Tavily / Perplexity / DuckDuckGo (auto-selects by key) |
| `web_fetch` | Fetch and clean web pages |
| `browser_navigate`, `browser_click`, `browser_type`, `browser_read_page`, `browser_screenshot` | Browser automation |
| `remember`, `recall`, `forget` | Agent memory |
| `get_skill`, `list_skills` | Load markdown skill packages |
| `village_forge` | Spawn sub-agents for multi-agent workflows |

---

## MCP — external tool servers

Connect any MCP-compatible server (databases, APIs, filesystems):

```bash
sovereign mcp list
sovereign mcp add sqlite --command "npx -y @modelcontextprotocol/server-sqlite -- ./data.db"
sovereign mcp remove sqlite
```

Or declare in `config.toml`:

```toml
[[mcp_servers]]
name = "sqlite"
timeout_secs = 30
env = []

[mcp_servers.transport]
type = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-sqlite", "--", "./data.db"]
```

---

## Execution modes

| Mode | Shell access | When to use |
|---|---|---|
| `sandbox` (default) | Allowlist only — `ls`, `cat`, `grep`, `git`, etc. | Daily use |
| `unrestricted` | Full host access | Local automation, scripting |

Approval gates (configurable in `config.toml`) prompt before risky operations like `write_file`, `browser_click`, or `shell_exec`.

---

## Cost tracking

```bash
sovereign treasury                # show spend summary
sovereign treasury show-budget    # show limits
```

Set budget limits in config:

```toml
[budget]
max_tokens_per_task = 50000
total_usd_limit = 5.00
```

---

## Other commands

```bash
sovereign agents list             # inspect running agents
sovereign agents inspect <id>
sovereign agents stop <id>
sovereign memory list             # view stored memories
sovereign memory export
sovereign audit logs              # audit trail
sovereign audit verify            # verify Merkle chain integrity
sovereign doctor                  # health check — checks config, API keys, tools
```

---

## Architecture

9-crate Rust workspace, strict dependency hierarchy (lower crates have no knowledge of higher ones):

| Crate | Purpose |
|---|---|
| `sk-types` | Shared types: config, agent, message, tool, session, errors |
| `sk-soul` | SOUL.md parsing and persona injection |
| `sk-memory` | SQLite memory substrate: KV, semantic vectors, BM25, knowledge graph, hybrid ranking |
| `sk-engine` | Agent execution loop, 4 LLM drivers, tool runner, MCP runtime, sandbox |
| `sk-mcp` | MCP protocol (JSON-RPC 2.0), stdio + SSE transports |
| `sk-hands` | Hand definitions (HAND.toml), registry, 11 bundled hands |
| `sk-tools` | Built-in tool implementations |
| `sk-kernel` | SovereignKernel struct — composes all subsystems |
| `sk-cli` | `sovereign` binary entry point and all CLI subcommands |

---

## Tests

```bash
cargo test                              # run all tests
cargo test -p sk-engine                 # single crate
cargo test test_extract_base_command    # single test by name
```

**821 tests passing** across 84 files (1 ignored). Coverage by crate:

| Crate | Tests |
|---|---|
| sk-engine | 358 |
| sk-types | 269 |
| sk-memory | 115 |
| sk-hands | 35 |
| sk-soul | 23 |
| sk-kernel | 9 |
| sk-cli | 1 |
| other | 11 |

---

## Platform support

| Platform | Status | Notes |
|---|---|---|
| Windows | Supported | Primary dev platform |
| macOS | Supported | All features including host tools |
| Linux | Supported | All features including host tools |
| iOS / Android | Not supported | Architecture incompatible (no process spawning) |

---

## License

MIT
