# Getting Started with Sovereign Kernel 🚀

Welcome to Sovereign Kernel, your local-first Agentic Operating System. This guide will help you install the kernel, configure your preferred LLM provider, and spawn your first background daemon.

## 1. Prerequisites

Before installing the kernel, please ensure your system has the following:
- **Rust (v1.75+)**: Installed via [rustup](https://rustup.rs/).
- **SQLite3**: Required for The Archive (Memory Substrate).
- **Supported OS**: Windows 10+, macOS 12+, or Linux (Kernel 5.13+ recommended for Landlock LSM security features).

## 2. Installation

Clone the repository and compile the highly optimized release binary using `cargo`:

```bash
git clone https://github.com/your-org/sovereign-kernel.git
cd sovereign-kernel

# Build the core workspace
cargo build --release --workspace

# Optional: Run the built-in system diagnostics script
cargo run --release -- doctor
```

## 3. Configuration & API Keys

Sovereign Kernel supports over 50 LLM Providers (OpenAI, Anthropic, NVIDIA NIM, Groq, local Ollama). You define your keys using environment variables to keep them completely isolated from the source code.

1. **Copy the environment template**:
   ```bash
   cp .env.example .env
   ```

2. **Add your preferred API key** inside `.env`:
   ```env
   # Example for OpenAI
   OPENAI_API_KEY="sk-proj-..."
   
   # Example for Anthropic
   ANTHROPIC_API_KEY="sk-ant-..."
   
   # Example for NVIDIA NIM (Mistral/Llama)
   NVIDIA_API_KEY="nvapi-..."
   ```

3. **Modify `config.toml`**: Ensure the kernel points to the correct provider loop.
   ```toml
   [default_model]
   provider = "openai" # Or "anthropic", "nvidia"
   model = "gpt-4o"
   api_key_env = "OPENAI_API_KEY"
   ```

## 4. Launching the OS

You can interact with Sovereign Kernel in two primary ways:

### A) The CLI Terminal (Foreground)
Launch the interactive terminal chat to directly converse with the Oracle.
```bash
cargo run --release -- run "Hello Oracle, please generate a Python script that calculates fibonacci."
```

### B) The Background Daemon (API Bridge)
If you want to plug the OS into your own UI, Discord bot, or Web Application, launch the background daemon. 
By default, this binds to `http://127.0.0.1:3030`.
```bash
cargo run --release -- start --detach
```

For advanced instructions on hitting the REST API or using Agentic Tools, please refer to the [USER_GUIDE.md](./USER_GUIDE.md).
