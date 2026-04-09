# Getting Started with Sovereign Kernel 🚀

Welcome to Sovereign Kernel — a local-first Agentic Operating System built in Rust. This guide walks you through installation, configuration, and running your first agent.

## 1. Prerequisites

- **Rust 1.75+** — Install via [rustup](https://rustup.rs/)
- **SQLite3** — Bundled automatically via `rusqlite`
- **Supported OS**: Windows 10+, macOS 12+, or Linux

## 2. Installation

```bash
git clone https://github.com/OpenEris/sovereign-kernel.git
cd sovereign-kernel
cargo build --release --workspace
```

## 3. Configuration

Sovereign Kernel uses two files for configuration:

### Step 1: Create your config file

```bash
cp config.toml.example config.toml
```

Edit `config.toml` to set your preferred LLM provider:

```toml
[[llm]]
provider = "openai"           # anthropic, gemini, nvidia, ollama, groq, etc.
api_key_env = "OPENAI_API_KEY"
```

> **Security note:** Never put API keys directly in `config.toml`. Always use `api_key_env` to reference environment variables.

### Step 2: Set your API keys

```bash
cp .env.example .env
```

Add your API key(s) inside `.env`:

```env
OPENAI_API_KEY=sk-proj-...
# Or for other providers:
ANTHROPIC_API_KEY=sk-ant-...
NVIDIA_API_KEY=nvapi-...
GEMINI_API_KEY=AI...
```

### Supported Providers

| Provider | Environment Variable | Config `provider` |
| :--- | :--- | :--- |
| OpenAI | `OPENAI_API_KEY` | `openai` |
| Anthropic | `ANTHROPIC_API_KEY` | `anthropic` |
| Google Gemini | `GEMINI_API_KEY` | `gemini` |
| NVIDIA NIM | `NVIDIA_API_KEY` | `nvidia` |
| Groq | `GROQ_API_KEY` | `groq` |
| DeepSeek | `DEEPSEEK_API_KEY` | `deepseek` |
| Ollama (local) | *(none needed)* | `ollama` |
| OpenRouter | `OPENROUTER_API_KEY` | `openrouter` |
| Mistral | `MISTRAL_API_KEY` | `mistral` |
| xAI / Grok | `XAI_API_KEY` | `xai` |

## 4. Running the Kernel

### Interactive CLI
```bash
cargo run --release -- chat
```

### One-shot task execution
```bash
cargo run --release -- run "Analyze this project and summarize the architecture"
```

### Execution modes
- **Sandbox** (default) — Dangerous operations require human approval
- **Unrestricted** — Full host access (set `execution_mode = "unrestricted"` in `config.toml`)

## 5. Next Steps

- Read the [User Guide](USER_GUIDE.md) for API usage and tool reference
- Explore [Architecture](docs/ARCHITECTURE.md) for the full crate breakdown
- Check [docs/USAGE.md](docs/USAGE.md) for all CLI commands
