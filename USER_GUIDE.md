# 🗺️ Sovereign Kernel User Guide

This guide covers how to interact with Sovereign Kernel beyond basic setup.

## 1. CLI Commands

```bash
sovereign chat                  # Interactive chat mode
sovereign run "task"            # One-shot task execution
sovereign init                  # Interactive setup wizard
sovereign status                # Show kernel and agent status
sovereign kill <agent-id>       # Kill a running agent
sovereign hands list            # List available capability packages
sovereign hands install <name>  # Install a Hand
sovereign doctor                # System diagnostics
sovereign memory export         # Export memory to JSON
sovereign memory import <file>  # Import memory from file
```

## 2. Execution Modes

### Sandbox Mode (Default)
- Dangerous operations (shell exec, code exec, file delete) trigger an approval prompt
- The agent pauses and waits for human approval or denial
- Recommended for interactive use

### Unrestricted Mode
- Set `execution_mode = "unrestricted"` in `config.toml`
- All operations execute without approval prompts
- Use with caution — the agent has full access to your system

## 3. Using the REST API

When running in daemon mode, Sovereign Kernel exposes a REST API on port `50051`:

```bash
# Send a message
curl -X POST http://127.0.0.1:50051/v1/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "What files are in the current directory?"}'

# Check treasury (cost tracking)
curl http://127.0.0.1:50051/v1/treasury/status

# List pending approvals
curl http://127.0.0.1:50051/v1/approvals

# Approve a pending request
curl -X POST http://127.0.0.1:50051/v1/approvals/{id}/approve
```

## 4. Tool Security

Every tool call is classified by risk level:

| Risk Level | Examples | Behavior |
| :--- | :--- | :--- |
| **Low** | `read_file`, `web_search`, `recall` | Auto-approved |
| **Medium** | `write_file`, `browser_navigate` | Logged, auto-approved |
| **High** | `shell_exec`, `delete_file`, `move_file` | Requires approval |
| **Critical** | `code_exec`, `host_desktop_control` | Always requires approval |

## 5. Memory System

Sovereign Kernel maintains persistent memory across sessions:

- **`remember`** — Store a fact in agent-scoped memory
- **`recall`** — Search agent memory using BM25 + semantic similarity
- **`forget`** — Remove a specific memory entry
- **Shared Memory** — Cross-agent knowledge store (requires `SharedMemory` capability)

## 6. Skills

Over 100 expert skill prompts are bundled in `crates/sk-tools/skills/`. Each skill provides domain-specific guidance (e.g., `git`, `docker`, `kubernetes`, `python`, `rust`).

## 7. Troubleshooting

**Context Overflow?**
The Healer automatically activates when context exceeds 80% of the model's token limit. It summarizes older turns while preserving recent context.

**Missing API Key?**
Check that your `.env` file contains the correct key and that `config.toml` references the right `api_key_env` field name.

**Database Issues?**
The memory database (`memory.db`) is stored in the kernel's data directory. You can inspect it with any SQLite client.
